use crate::commands::embedding;
use crate::commands::vectorstore;
use crate::models::MemoryNode;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

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

/// Auto-extract memories from group chat turns.
/// Placeholder — real LLM call to be implemented in a follow-up.
pub async fn auto_extract_memories(
    _character_id: &str,
    _turns: &[String],
) -> Vec<MemoryNode> {
    Vec::new()
}

/// Semantic dedup: check cosine similarity against existing memory vectors.
#[allow(dead_code)]
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
