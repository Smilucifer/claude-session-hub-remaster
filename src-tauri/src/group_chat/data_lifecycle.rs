use crate::storage::characters;
use chrono::Utc;

/// Compact memory log by deduplicating entries (keep latest version of each ID).
/// Returns true if compaction was performed.
pub fn compact_memory_log_if_needed(character_id: &str) -> Result<bool, String> {
    let entries = characters::read_all_memory_log_entries(character_id)?;
    if entries.len() < 10_000 {
        return Ok(false);
    }

    use std::collections::HashMap;
    let mut latest: HashMap<String, usize> = HashMap::new();
    for (i, entry) in entries.iter().enumerate() {
        latest.insert(entry.id.clone(), i);
    }

    let mut compacted: Vec<_> = latest.into_values().map(|i| entries[i].clone()).collect();
    compacted.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    // Rewrite compacted log
    let path = characters::memory_log_path(character_id);
    let tmp_path = path.with_extension(format!("jsonl.{}.tmp", uuid::Uuid::new_v4()));
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("compact: create tmp: {e}"))?;
    use std::io::Write;
    for node in &compacted {
        let line = serde_json::to_string(node)
            .map_err(|e| format!("compact: serialize: {e}"))? + "\n";
        file.write_all(line.as_bytes())
            .map_err(|e| format!("compact: write: {e}"))?;
    }
    file.flush().map_err(|e| format!("compact: flush: {e}"))?;
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| format!("compact: rename: {e}"))?;

    // Trigger derived data rebuild after compaction
    let _ = std::fs::remove_dir_all(characters::char_dir(character_id).join("lancedb"));
    Ok(true)
}

/// Apply retention policy: remove entries older than retention_days.
/// Returns count of removed entries.
pub fn apply_retention_policy(character_id: &str, retention_days: u32) -> Result<usize, String> {
    let entries = characters::read_all_memory_log_entries(character_id)?;
    let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let (keep, removed): (Vec<_>, Vec<_>) = entries
        .into_iter()
        .partition(|e| e.created_at >= cutoff_str);

    let path = characters::memory_log_path(character_id);
    let tmp_path = path.with_extension(format!("jsonl.{}.tmp", uuid::Uuid::new_v4()));
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("retention: create tmp: {e}"))?;
    use std::io::Write;
    for node in &keep {
        let line = serde_json::to_string(node)
            .map_err(|e| format!("retention: serialize: {e}"))? + "\n";
        file.write_all(line.as_bytes())
            .map_err(|e| format!("retention: write: {e}"))?;
    }
    file.flush().map_err(|e| format!("retention: flush: {e}"))?;
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| format!("retention: rename: {e}"))?;

    Ok(removed.len())
}
