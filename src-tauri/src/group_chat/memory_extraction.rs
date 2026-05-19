use crate::commands::embedding;
use crate::commands::vectorstore;
use crate::models::{MemoryNode, MemorySource};
use crate::storage::settings;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Mutex;
use std::time::Instant;

static LOG_WRITER: Lazy<Mutex<Option<BufWriter<File>>>> = Lazy::new(|| Mutex::new(None));

pub fn log_to_file(msg: &str) {
    let mut guard = LOG_WRITER.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        let data_dir = crate::storage::data_dir();
        let log_dir = data_dir.join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("memory-extraction.log");
        match std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            Ok(f) => *guard = Some(BufWriter::new(f)),
            Err(_) => return,
        }
    }
    if let Some(ref mut w) = *guard {
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(w, "[{ts}] {msg}");
    }
}

// Shared HTTP client for memory extraction (avoids creating a new connection pool per call)
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(reqwest::Client::new);

// Debounce: per group-chat, last extraction time
static LAST_EXTRACTION: Lazy<Mutex<HashMap<String, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// Daily caps: per character, count of extractions today
static DAILY_EXTRACTION_COUNT: Lazy<Mutex<HashMap<String, (String, u32)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn debounce_key(group_chat_id: &str, character_id: &str) -> String {
    format!("{}:{}", group_chat_id, character_id)
}

pub fn can_extract(group_chat_id: &str, character_id: &str) -> bool {
    // Debounce: 5 min per (group_chat, character) pair
    {
        let key = debounce_key(group_chat_id, character_id);
        let map = LAST_EXTRACTION.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(last) = map.get(&key) {
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
        let key = debounce_key(group_chat_id, character_id);
        let mut map = LAST_EXTRACTION.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(key, Instant::now());
    }
    {
        let mut map = DAILY_EXTRACTION_COUNT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = map.get_mut(character_id) {
            entry.1 += 1;
        }
    }
}

/// Auto-extract memories from group chat turns via LLM.
/// Returns (MemoryNode, embedding_vec) pairs — the embedding is computed once
/// during dedup and reused for vector upsert, avoiding a redundant API call.
pub async fn auto_extract_memories(
    character_id: &str,
    turns: &[String],
) -> Vec<(MemoryNode, Vec<f32>)> {
    log_to_file(&format!("[memory-extraction] ENTER auto_extract_memories cid={} turns={}", character_id, turns.len()));
    let Some(config) = settings::get_embedding_config() else {
        log_to_file("[memory-extraction] SKIP no embedding config");
        return Vec::new();
    };
    if !config.enabled || (config.chat_api_key.is_none() && config.api_key.is_none()) {
        log_to_file(&format!("[memory-extraction] SKIP config disabled or no api_key: enabled={} chat_key={} embed_key={}", config.enabled, config.chat_api_key.is_some(), config.api_key.is_some()));
        return Vec::new();
    }

    // Derive chat completions endpoint: prefer explicit config, fallback to derivation
    let chat_endpoint = config.chat_endpoint.clone()
        .unwrap_or_else(|| derive_chat_endpoint(&config.endpoint));
    let chat_model = config.chat_model.clone()
        .unwrap_or_else(|| config.model.clone());
    // Prefer dedicated chat_api_key, fall back to embedding api_key
    let Some(api_key) = config.chat_api_key.or(config.api_key) else {
        return Vec::new();
    };

    // Build conversation text
    let conversation: String = turns
        .iter()
        .enumerate()
        .map(|(i, t)| format!("Turn {}: {}", i + 1, t))
        .collect::<Vec<_>>()
        .join("\n\n");

    let truncated_conv: String = conversation.chars().take(4000).collect();
    let prompt = format!(
        r#"分析以下群聊对话，提取关于参与者的关键事实、偏好或知识，这些信息对未来对话有用。

返回一个 JSON 数组。每个对象包含：
- "content"：提取的事实/偏好（简洁，一句话）
- "type"：以下之一 "fact"、"preference"、"relationship"、"skill"
- "tags"：相关关键词数组

要求：
- 内容必须使用与对话相同的语言
- 只返回 JSON 数组，不要其他文本。如果没有值得提取的内容，返回 []

对话：
{text}"#,
        text = truncated_conv
    );

    let body = serde_json::json!({
        "model": chat_model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.1,
        "max_tokens": 2000,
    });

    log_to_file(&format!("[memory-extraction] LLM request: endpoint={} model={} conv_chars={} (truncated from {})", chat_endpoint, chat_model, truncated_conv.len(), conversation.len()));
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
            log_to_file(&format!("[memory-extraction] LLM_HTTP_ERROR cid={} err={}", character_id, e));
            return Vec::new();
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        log_to_file(&format!("[memory-extraction] LLM_HTTP_ERROR cid={} status={} body={}", character_id, status, body_text.chars().take(500).collect::<String>()));
        return Vec::new();
    }

    let resp_text = resp.text().await.unwrap_or_default();
    log_to_file(&format!("[memory-extraction] LLM_RAW_RESPONSE cid={} body={}", character_id, resp_text.chars().take(500).collect::<String>()));

    let json: Value = match serde_json::from_str(&resp_text) {
        Ok(v) => v,
        Err(e) => {
            log_to_file(&format!("[memory-extraction] LLM_JSON_PARSE_ERROR cid={} err={}", character_id, e));
            return Vec::new();
        }
    };

    // Extract content from OpenAI-compatible response format
    // Note: DeepSeek V4 has "reasoning_content" (model thinking) and "content" (actual output).
    // Only "content" contains the structured JSON we need — reasoning_content is internal chain-of-thought.
    let message = &json["choices"][0]["message"];
    let finish_reason = json["choices"][0]["finish_reason"].as_str().unwrap_or("");
    let content = message["content"].as_str().filter(|c| !c.trim().is_empty());
    let Some(content) = content else {
        let reasoning_len = message["reasoning_content"].as_str().map(|s| s.len()).unwrap_or(0);
        log_to_file(&format!("[memory-extraction] LLM_NO_CONTENT cid={} finish_reason={} reasoning_len={}", character_id, finish_reason, reasoning_len));
        return Vec::new();
    };
    log_to_file(&format!("[memory-extraction] LLM_GOT_CONTENT cid={} len={}", character_id, content.len()));

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
            log_to_file(&format!("[memory-extraction] JSON_PARSE_ERROR cid={} err={} json_str={}", character_id, e, json_str.chars().take(300).collect::<String>()));
            return Vec::new();
        }
    };
    log_to_file(&format!("[memory-extraction] PARSED_ITEMS cid={} count={}", character_id, items.len()));

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

        // Compute embedding once — used for both dedup check and later vector upsert
        let embedding_vec = match embedding::fetch_embedding(&content_text).await {
            Ok(v) => v,
            Err(e) => {
                log_to_file(&format!("[memory-extraction] EMBED_ERROR cid={} err={}", character_id, e));
                continue;
            }
        };

        // Dedup check: skip if a very similar memory already exists
        if is_duplicate(character_id, &embedding_vec).await {
            log_to_file(&format!("[memory-extraction] SKIP duplicate cid={} preview={}", character_id, &content_text.chars().take(50).collect::<String>()));
            continue;
        }

        let memory = MemoryNode {
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
        };
        results.push((memory, embedding_vec));
    }

    log_to_file(&format!("[memory-extraction] RESULT cid={} extracted={} after_dedup={}", character_id, items.len(), results.len()));
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
pub async fn is_duplicate(
    character_id: &str,
    candidate_vec: &[f32],
) -> bool {
    let results = match vectorstore::vector_search(character_id.to_string(), candidate_vec.to_vec(), 5).await
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
