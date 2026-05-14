use crate::commands::embedding;
use crate::commands::vectorstore;
use crate::models::{MemoryNode, MemorySource};
use crate::storage::settings;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

// Shared HTTP client for memory extraction (avoids creating a new connection pool per call)
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

// Debounce: per group-chat, last extraction time
static LAST_EXTRACTION: Lazy<Mutex<HashMap<String, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Daily caps: per character, count of extractions today
static DAILY_EXTRACTION_COUNT: Lazy<Mutex<HashMap<String, (String, u32)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn can_extract(group_chat_id: &str, character_id: &str) -> bool {
    // Debounce: 5 min per group chat
    {
        let map = LAST_EXTRACTION.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(last) = map.get(group_chat_id) {
            if last.elapsed().as_secs() < 300 {
                return false;
            }
        }
    }

    // Daily cap: 10 per character per day
    {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut map = DAILY_EXTRACTION_COUNT.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map
            .entry(character_id.to_string())
            .or_insert_with(|| (today.clone(), 0));
        if entry.0 != today {
            entry.0 = today;
            entry.1 = 0;
        }
        if entry.1 >= 10 {
            return false;
        }
    }

    true
}

pub fn record_extraction(group_chat_id: &str, character_id: &str) {
    {
        let mut map = LAST_EXTRACTION.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(group_chat_id.to_string(), Instant::now());
    }
    {
        let mut map = DAILY_EXTRACTION_COUNT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(character_id) {
            entry.1 += 1;
        }
    }
}

/// Auto-extract memories from group chat turns via LLM.
/// Uses the embedding config's endpoint to derive a chat completions URL.
/// Returns extracted MemoryNodes after dedup check. Errors are silently ignored.
pub async fn auto_extract_memories(
    character_id: &str,
    turns: &[String],
) -> Vec<MemoryNode> {
    let config = match settings::get_embedding_config() {
        Some(c) if c.enabled && c.api_key.is_some() => c,
        _ => return Vec::new(),
    };

    // Derive chat completions endpoint: prefer explicit config, fallback to derivation
    let chat_endpoint = config.chat_endpoint.clone()
        .unwrap_or_else(|| derive_chat_endpoint(&config.endpoint));
    let chat_model = config.chat_model.clone()
        .unwrap_or_else(|| config.model.clone());
    let api_key = config.api_key.unwrap();

    // Build conversation text
    let conversation: String = turns
        .iter()
        .enumerate()
        .map(|(i, t)| format!("Turn {}: {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = format!(
        r#"Analyze the following group chat conversation and extract key facts, preferences, or knowledge about the participants that would be useful for future conversations.

Return a JSON array of objects. Each object has:
- "content": the extracted fact/preference (concise, one sentence)
- "type": one of "fact", "preference", "relationship", "skill"
- "tags": array of relevant keywords

Return ONLY the JSON array, no other text. If nothing worth extracting, return [].

Conversation:
{text}"#,
        text = conversation.chars().take(4000).collect::<String>()
    );

    let body = serde_json::json!({
        "model": chat_model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.1,
        "max_tokens": 1000,
    });

    let resp = match HTTP_CLIENT
        .post(&chat_endpoint)
        .timeout(std::time::Duration::from_secs(30))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log::debug!("[memory-extraction] LLM call failed: {e}");
            return Vec::new();
        }
    };

    if !resp.status().is_success() {
        log::debug!(
            "[memory-extraction] LLM returned HTTP {}",
            resp.status()
        );
        return Vec::new();
    }

    let json: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            log::debug!("[memory-extraction] Failed to parse LLM response: {e}");
            return Vec::new();
        }
    };

    // Extract content from OpenAI-compatible response format
    let content = match json["choices"][0]["message"]["content"].as_str() {
        Some(c) => c,
        None => {
            log::debug!("[memory-extraction] No content in LLM response");
            return Vec::new();
        }
    };

    // Parse JSON array from the response (strip markdown code fences if present)
    let json_str = content.trim();
    let json_str = json_str
        .strip_prefix("```json")
        .or_else(|| json_str.strip_prefix("```"))
        .unwrap_or(json_str)
        .trim_end_matches("```")
        .trim();

    let items: Vec<Value> = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            log::debug!("[memory-extraction] Failed to parse extracted items: {e}");
            return Vec::new();
        }
    };

    let now = crate::models::now_iso();
    let mut results = Vec::new();

    for item in &items {
        let content_text = match item["content"].as_str() {
            Some(c) if !c.trim().is_empty() => c.trim().to_string(),
            _ => continue,
        };

        let raw_type = item["type"].as_str().unwrap_or("fact");
        let memory_type = match raw_type {
            "fact" | "preference" | "relationship" | "skill" => raw_type.to_string(),
            _ => "fact".to_string(),
        };

        let tags: Vec<String> = item["tags"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Dedup check: skip if a very similar memory already exists
        if dedup_check(character_id, &content_text).await {
            let preview: String = content_text.chars().take(50).collect();
            log::debug!("[memory-extraction] Skipping duplicate: {}", preview);
            continue;
        }

        results.push(MemoryNode {
            id: uuid::Uuid::new_v4().to_string(),
            character_id: character_id.to_string(),
            content: content_text,
            memory_type,
            confidence: 70.0,
            source: MemorySource {
                kind: "auto_extract".to_string(),
                run_id: None,
                group_chat_id: None,
            },
            tags,
            created_at: now.clone(),
            updated_at: now.clone(),
            status: "pending".to_string(),
        });
    }

    results
}

/// Derive a chat completions endpoint from an embedding endpoint.
/// e.g. ".../v1/embeddings" -> ".../v1/chat/completions"
fn derive_chat_endpoint(embedding_endpoint: &str) -> String {
    if embedding_endpoint.ends_with("/embeddings") {
        format!(
            "{}/chat/completions",
            &embedding_endpoint[..embedding_endpoint.len() - "/embeddings".len()]
        )
    } else {
        let fallback = format!("{}/chat/completions", embedding_endpoint.trim_end_matches('/'));
        log::warn!(
            "[memory-extraction] Embedding endpoint does not end with /embeddings, derived chat endpoint may be incorrect: {} -> {}",
            embedding_endpoint, fallback
        );
        fallback
    }
}

/// Semantic dedup: check cosine similarity against existing memory vectors.
pub async fn dedup_check(
    character_id: &str,
    candidate_text: &str,
) -> bool {
    let candidate_vec = match embedding::fetch_embedding(candidate_text).await {
        Ok(v) => v,
        Err(_) => return false,
    };

    let results = match vectorstore::vector_search(character_id.to_string(), candidate_vec, 5).await
    {
        Ok(r) => r,
        Err(_) => return false,
    };

    for result in results {
        if result.score > 0.92 {
            return true; // duplicate
        }
    }
    false
}
