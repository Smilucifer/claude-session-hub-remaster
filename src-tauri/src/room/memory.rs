use super::models::{MemoryKind, PendingMemoryCandidate, SeatMemoryEntry, SeatProfile};
use std::collections::HashMap;

pub const MAX_SEAT_MEMORY_ENTRIES: usize = 50;
pub const MEMORY_PROMPT_CHAR_CAP: usize = 2000;
pub const MAX_PROFILE_ENTRIES: usize = 20;
pub const CHECKPOINT_THRESHOLD: u64 = 3;
pub const CHECKPOINT_COOLDOWN_SECS: u64 = 60;
pub const MAX_REMINDER_COUNT: u32 = 3;
pub const INBOX_EXPIRY_DAYS: i64 = 7;

/// Evict lowest-recall non-persisted entries when over limit.
/// Persisted entries are never evicted — if all entries are persisted, the cap is relaxed.
pub fn evict_seat_memory(entries: &mut Vec<SeatMemoryEntry>) {
    if entries.len() <= MAX_SEAT_MEMORY_ENTRIES {
        return;
    }
    // Sort non-persisted by recall descending, keep high-recall ones
    let persisted_count = entries.iter().filter(|e| e.persisted).count();
    let budget = MAX_SEAT_MEMORY_ENTRIES.saturating_sub(persisted_count);
    let mut persisted: Vec<SeatMemoryEntry> = entries.drain(..).filter(|e| e.persisted).collect();
    let mut non_persisted: Vec<SeatMemoryEntry> = entries.drain(..).collect();
    non_persisted.sort_by(|a, b| b.recall.cmp(&a.recall).then_with(|| b.created_at.cmp(&a.created_at)));
    non_persisted.truncate(budget);
    persisted.extend(non_persisted);
    *entries = persisted;
}

/// Build the `## 你的背景知识` prompt section from seat memory entries.
/// Returns None if no entries. Caps output at MEMORY_PROMPT_CHAR_CAP chars.
pub fn build_memory_prompt_section(entries: &[SeatMemoryEntry]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let mut sorted: Vec<&SeatMemoryEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| {
        b.recall
            .cmp(&a.recall)
            .then_with(|| b.created_at.cmp(&a.created_at))
    });

    let mut body = String::from("\n## 你的背景知识\n");
    let cap = MEMORY_PROMPT_CHAR_CAP;
    for entry in sorted {
        let kind_label = memory_kind_label(&entry.kind);
        let safe_key: String = entry.key.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        let safe_content: String = entry.content.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        let line = format!("- [{}] {}: {}\n", kind_label, safe_key, safe_content);
        if body.len() + line.len() > cap {
            body.push_str("...(更多记忆已省略)\n");
            break;
        }
        body.push_str(&line);
    }
    Some(body)
}

/// Increment recall and update last_accessed for entries whose IDs are in `ids`.
pub fn recall_entries(entries: &mut Vec<SeatMemoryEntry>, ids: &[String], now: &str) {
    let id_set: std::collections::HashSet<&String> = ids.iter().collect();
    for entry in entries {
        if id_set.contains(&entry.id) {
            entry.recall += 1;
            entry.last_accessed = now.to_string();
        }
    }
}

/// Parse a memory marker line: `[insight] content`, `[lesson] content`, etc.
pub fn parse_seat_memory_line(line: &str) -> Option<(MemoryKind, &str)> {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("[insight]") {
        return Some((MemoryKind::Insight, line[9..].trim()));
    }
    if lower.starts_with("[lesson]") {
        return Some((MemoryKind::Lesson, line[8..].trim()));
    }
    if lower.starts_with("[preference]") {
        return Some((MemoryKind::Preference, line[12..].trim()));
    }
    if lower.starts_with("[fact]") {
        return Some((MemoryKind::Fact, line[6..].trim()));
    }
    None
}

/// Extract memory candidates from a response text (line-by-line scan).
pub fn extract_seat_memory_candidates(
    response_text: &str,
    source_participant_id: &str,
    source_turn_id: &str,
    generated_at: &str,
) -> Vec<PendingMemoryCandidate> {
    let mut candidates = Vec::new();
    for line in response_text.lines() {
        let trimmed = line.trim().trim_start_matches("- ").trim_start_matches("* ");
        if let Some((kind, text)) = parse_seat_memory_line(trimmed) {
            if !text.is_empty() {
                let id = format!(
                    "pending-{}-{}",
                    &generated_at[..10],
                    &uuid_simple()[..8]
                );
                let expires_at = chrono_naive_add_days(generated_at, INBOX_EXPIRY_DAYS);
                candidates.push(PendingMemoryCandidate {
                    id,
                    kind,
                    key: truncate_str(text, 60),
                    content: text.to_string(),
                    source_participant_id: source_participant_id.to_string(),
                    source_turn_id: source_turn_id.to_string(),
                    created_at: generated_at.to_string(),
                    reminder_count: 0,
                    expires_at,
                });
            }
        }
    }
    candidates
}

/// Remove expired entries from inbox.
pub fn expire_inbox_entries(inbox: &mut Vec<PendingMemoryCandidate>, now: &str) {
    let Ok(now_dt) = chrono_parse(now) else { return };
    inbox.retain(|item| {
        chrono_parse(&item.expires_at).map(|exp| exp > now_dt).unwrap_or(true)
    });
}

/// Increment reminder count, remove entries exceeding MAX_REMINDER_COUNT.
pub fn increment_reminders(inbox: &mut Vec<PendingMemoryCandidate>) {
    for item in inbox.iter_mut() {
        item.reminder_count += 1;
    }
    inbox.retain(|item| item.reminder_count < MAX_REMINDER_COUNT);
}

/// Accept an inbox entry: remove from inbox, return it for promotion.
pub fn accept_inbox_entry(
    inbox: &mut Vec<PendingMemoryCandidate>,
    entry_id: &str,
) -> Option<PendingMemoryCandidate> {
    if let Some(pos) = inbox.iter().position(|item| item.id == entry_id) {
        Some(inbox.remove(pos))
    } else {
        None
    }
}

/// Reject an inbox entry: simply remove.
pub fn reject_inbox_entry(inbox: &mut Vec<PendingMemoryCandidate>, entry_id: &str) {
    inbox.retain(|item| item.id != entry_id);
}

/// Auto-merge same kind+key entries in inbox (keep the one with longer content, then newer).
pub fn auto_merge_same_key(inbox: &mut Vec<PendingMemoryCandidate>) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut to_remove = Vec::new();
    for (i, item) in inbox.iter().enumerate() {
        let key = format!("{:?}:{}", item.kind, item.key);
        if let Some(&prev_idx) = seen.get(&key) {
            let prev = &inbox[prev_idx];
            // Keep longer content; tie-break: newer created_at wins
            if item.content.len() > prev.content.len()
                || (item.content.len() == prev.content.len() && item.created_at > prev.created_at)
            {
                to_remove.push(prev_idx);
                seen.insert(key, i);
            } else {
                to_remove.push(i);
            }
        } else {
            seen.insert(key, i);
        }
    }
    to_remove.sort_unstable_by(|a, b| b.cmp(a)); // remove from end
    for idx in to_remove {
        if idx < inbox.len() {
            inbox.remove(idx);
        }
    }
}

/// Build profile consensus from all seat memories.
/// Collects entries with recall >= 3 across all seats, deduplicates by kind+key, takes top 20.
pub fn build_profile_from_seat_memories(
    seat_memories: &HashMap<String, Vec<SeatMemoryEntry>>,
) -> SeatProfile {
    let mut candidates: Vec<&SeatMemoryEntry> = Vec::new();
    for entries in seat_memories.values() {
        for entry in entries {
            if entry.persisted || entry.recall >= 3 {
                candidates.push(entry);
            }
        }
    }
    // Deduplicate by kind+key, keeping highest recall
    let mut best: HashMap<String, &SeatMemoryEntry> = HashMap::new();
    for entry in candidates {
        let key = format!("{:?}:{}", entry.kind, entry.key);
        match best.get(&key) {
            Some(existing) if existing.recall >= entry.recall => {}
            _ => {
                best.insert(key, entry);
            }
        }
    }
    let mut entries: Vec<SeatMemoryEntry> = best.values().map(|e| (*e).clone()).collect();
    entries.sort_by(|a, b| b.recall.cmp(&a.recall));
    entries.truncate(MAX_PROFILE_ENTRIES);
    SeatProfile {
        entries,
        updated_at: crate::models::now_iso(),
    }
}

/// Build the `## 圆桌共识` prompt section from profile.
pub fn build_profile_prompt_section(profile: &SeatProfile) -> Option<String> {
    if profile.entries.is_empty() {
        return None;
    }
    let mut body = String::from("\n## 圆桌共识\n");
    for entry in &profile.entries {
        let kind_label = memory_kind_label(&entry.kind);
        let safe_key: String = entry.key.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        let safe_content: String = entry.content.chars().filter(|c| *c != '\n' && *c != '\r').collect();
        body.push_str(&format!("- [{}] {}: {}\n", kind_label, safe_key, safe_content));
    }
    Some(body)
}

/// Check if a checkpoint should run based on turn index and cooldown.
pub fn should_run_checkpoint(
    turn_idx: u64,
    last_checkpoint_turn: u64,
    last_checkpoint_at: Option<&str>,
    now: &str,
) -> bool {
    // Check turn threshold
    if turn_idx.saturating_sub(last_checkpoint_turn) < CHECKPOINT_THRESHOLD {
        return false;
    }
    // Check cooldown
    if let Some(last_at) = last_checkpoint_at {
        if let (Ok(last), Ok(cur)) = (
            chrono_parse(last_at),
            chrono_parse(now),
        ) {
            let elapsed = cur.signed_duration_since(last);
            if elapsed.num_seconds() < CHECKPOINT_COOLDOWN_SECS as i64 {
                return false;
            }
        }
    }
    true
}

fn memory_kind_label(kind: &MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Insight => "洞见",
        MemoryKind::Lesson => "教训",
        MemoryKind::Preference => "偏好",
        MemoryKind::Fact => "事实",
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

fn uuid_simple() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn chrono_parse(s: &str) -> Result<chrono::DateTime<chrono::Utc>, chrono::ParseError> {
    chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&chrono::Utc))
}

fn chrono_naive_add_days(s: &str, days: i64) -> String {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        (dt + chrono::Duration::days(days)).to_rfc3339()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, kind: MemoryKind, recall: u32, persisted: bool) -> SeatMemoryEntry {
        SeatMemoryEntry {
            id: id.to_string(),
            kind,
            key: format!("key-{}", id),
            content: format!("content for {}", id),
            recall,
            last_accessed: "2026-01-01".to_string(),
            created_at: "2026-01-01".to_string(),
            persisted,
            source_turn_id: None,
        }
    }

    #[test]
    fn evict_removes_lowest_recall() {
        let mut entries: Vec<SeatMemoryEntry> = (0..52)
            .map(|i| make_entry(&format!("e{}", i), MemoryKind::Fact, i as u32, false))
            .collect();
        evict_seat_memory(&mut entries);
        assert_eq!(entries.len(), MAX_SEAT_MEMORY_ENTRIES);
        // Highest recall entries survive
        assert!(entries.iter().any(|e| e.id == "e51"));
        assert!(!entries.iter().any(|e| e.id == "e0"));
    }

    #[test]
    fn evict_keeps_persisted() {
        let mut entries: Vec<SeatMemoryEntry> = (0..52)
            .map(|i| {
                make_entry(
                    &format!("e{}", i),
                    MemoryKind::Fact,
                    i as u32,
                    i == 0, // first one persisted
                )
            })
            .collect();
        evict_seat_memory(&mut entries);
        assert!(entries.iter().any(|e| e.id == "e0" && e.persisted));
    }

    #[test]
    fn build_memory_prompt_section_respects_cap() {
        let entries: Vec<SeatMemoryEntry> = (0..100)
            .map(|i| {
                SeatMemoryEntry {
                    id: format!("e{}", i),
                    kind: MemoryKind::Insight,
                    key: format!("key-{}", i),
                    content: "x".repeat(100),
                    recall: i,
                    last_accessed: String::new(),
                    created_at: "2026-01-01".to_string(),
                    persisted: false,
                    source_turn_id: None,
                }
            })
            .collect();
        let section = build_memory_prompt_section(&entries).unwrap();
        assert!(section.chars().count() <= MEMORY_PROMPT_CHAR_CAP + 100); // some slack for header
    }

    #[test]
    fn build_memory_prompt_section_none_when_empty() {
        assert!(build_memory_prompt_section(&[]).is_none());
    }

    #[test]
    fn parse_seat_memory_line_all_kinds() {
        assert!(matches!(
            parse_seat_memory_line("[insight] something"),
            Some((MemoryKind::Insight, "something"))
        ));
        assert!(matches!(
            parse_seat_memory_line("[lesson] learned"),
            Some((MemoryKind::Lesson, "learned"))
        ));
        assert!(matches!(
            parse_seat_memory_line("[preference] like this"),
            Some((MemoryKind::Preference, "like this"))
        ));
        assert!(matches!(
            parse_seat_memory_line("[fact] it is true"),
            Some((MemoryKind::Fact, "it is true"))
        ));
        assert!(parse_seat_memory_line("no marker here").is_none());
    }

    #[test]
    fn extract_candidates_from_text() {
        let text = "Some analysis\n[insight] Bull market likely\n[lesson] Cut losses early\nNormal line";
        let candidates = extract_seat_memory_candidates(text, "p1", "t1", "2026-05-11T10:00:00Z");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].content, "Bull market likely");
        assert_eq!(candidates[1].content, "Cut losses early");
    }

    #[test]
    fn should_run_checkpoint_respects_threshold() {
        assert!(!should_run_checkpoint(2, 0, None, "2026-01-01T00:01:00Z"));
        assert!(should_run_checkpoint(3, 0, None, "2026-01-01T00:01:00Z"));
    }

    #[test]
    fn expire_removes_old_entries() {
        let mut inbox = vec![PendingMemoryCandidate {
            id: "p1".to_string(),
            kind: MemoryKind::Fact,
            key: "k".to_string(),
            content: "c".to_string(),
            source_participant_id: "s".to_string(),
            source_turn_id: "t".to_string(),
            created_at: "2026-01-01".to_string(),
            reminder_count: 0,
            expires_at: "2026-01-05".to_string(),
        }];
        expire_inbox_entries(&mut inbox, "2026-01-10T00:00:00Z");
        assert!(inbox.is_empty());
    }

    #[test]
    fn auto_merge_keeps_longer_content() {
        let mut inbox = vec![
            PendingMemoryCandidate {
                id: "p1".to_string(),
                kind: MemoryKind::Fact,
                key: "same".to_string(),
                content: "short".to_string(),
                source_participant_id: "s".to_string(),
                source_turn_id: "t".to_string(),
                created_at: "2026-01-01".to_string(),
                reminder_count: 0,
                expires_at: "2026-12-01".to_string(),
            },
            PendingMemoryCandidate {
                id: "p2".to_string(),
                kind: MemoryKind::Fact,
                key: "same".to_string(),
                content: "much longer content here".to_string(),
                source_participant_id: "s".to_string(),
                source_turn_id: "t".to_string(),
                created_at: "2026-01-02".to_string(),
                reminder_count: 0,
                expires_at: "2026-12-01".to_string(),
            },
        ];
        auto_merge_same_key(&mut inbox);
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].content, "much longer content here");
    }
}
