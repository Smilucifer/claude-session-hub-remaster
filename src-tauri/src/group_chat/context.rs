use crate::group_chat::models::{
    GroupChat, GroupChatParticipant, GroupChatTurn, GroupChatTurnMode,
};
use crate::storage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Constants ──

const HANDOFF_TURN_THRESHOLD: u32 = 25;
const BOOTSTRAP_TOKEN_CAP: usize = 8000; // ~2000 tokens at ~4 chars/token
const BOOTSTRAP_TURN_WINDOW: usize = 5;
const TRUNCATION_SUFFIX: &str = "\n…[truncated]";

// ── ParticipantMeta ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParticipantMeta {
    /// Last delivered turn index for this participant.
    pub delivery_cursor: usize,
    /// Number of turns this participant has responded to in the current session.
    pub session_turn_count: u32,
    /// Session restart counter — incremented on each handoff.
    pub session_seq: u32,
}

impl Default for ParticipantMeta {
    fn default() -> Self {
        Self {
            delivery_cursor: 0,
            session_turn_count: 0,
            session_seq: 0,
        }
    }
}

// ── Storage helpers ──

fn participants_dir(group_chat_id: &str) -> PathBuf {
    storage::data_dir()
        .join("group-chats")
        .join(group_chat_id)
        .join("participants")
}

fn participant_meta_path(group_chat_id: &str, participant_id: &str) -> PathBuf {
    participants_dir(group_chat_id).join(format!("{participant_id}.meta.json"))
}

pub fn load_participant_meta(group_chat_id: &str, participant_id: &str) -> ParticipantMeta {
    let path = participant_meta_path(group_chat_id, participant_id);
    fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_participant_meta(
    group_chat_id: &str,
    participant_id: &str,
    meta: &ParticipantMeta,
) -> Result<(), String> {
    let dir = participants_dir(group_chat_id);
    storage::ensure_dir(&dir).map_err(|e| e.to_string())?;
    let path = participant_meta_path(group_chat_id, participant_id);
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("write participant meta: {e}"))
}

// ── Cursor tracking ──

/// Update the delivery cursor for a participant to the given turn index.
pub fn advance_delivery_cursor(
    group_chat_id: &str,
    participant_id: &str,
    turn_idx: usize,
) -> Result<(), String> {
    let mut meta = load_participant_meta(group_chat_id, participant_id);
    if turn_idx > meta.delivery_cursor {
        meta.delivery_cursor = turn_idx;
        save_participant_meta(group_chat_id, participant_id, &meta)?;
    }
    Ok(())
}

// ── Visibility filter (Task 37) ──

/// Filter turns based on the given mode and participant identity.
///
/// Rules:
/// - `Fanout`: all turns visible
/// - `Debate`: all turns visible
/// - `Summary`: all turns visible
/// - `Private`: only turns where participant is sender or target
/// - `SingleTarget`: only turns where participant is sender or target
pub fn filter_visible_messages(
    turns: &[GroupChatTurn],
    participant_id: &str,
    mode: &GroupChatTurnMode,
) -> Vec<GroupChatTurn> {
    match mode {
        GroupChatTurnMode::Fanout
        | GroupChatTurnMode::Debate
        | GroupChatTurnMode::Summary => turns.to_vec(),
        GroupChatTurnMode::Private | GroupChatTurnMode::SingleTarget => turns
            .iter()
            .filter(|turn| {
                let is_sender = turn.responses.iter().any(|r| r.participant_id == participant_id);
                let is_target = turn.target_participant_ids.iter().any(|id| id == participant_id);
                is_sender || is_target
            })
            .cloned()
            .collect(),
    }
}

// ── Session handoff (Task 38) ──

/// Result of checking whether a participant needs session handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandoffDecision {
    /// No handoff needed.
    Continue,
    /// Handoff threshold reached — caller should spawn a fresh session.
    Handoff { bootstrap_context: String },
}

/// Check whether a participant has exceeded the turn threshold and needs a
/// fresh session. Returns `Handoff` with a bootstrap context string if so.
///
/// The bootstrap context includes:
/// - Group chat name
/// - Character role (participant label + role)
/// - Plan status summary (if a plan exists)
/// - Last 5 public turns (truncated)
/// - Own last response (truncated)
/// - Total output capped at ~2000 tokens (~8000 chars)
pub fn check_handoff(
    group_chat: &GroupChat,
    participant: &GroupChatParticipant,
    public_turns: &[GroupChatTurn],
) -> HandoffDecision {
    let meta = load_participant_meta(&group_chat.id, &participant.id);
    if meta.session_turn_count <= HANDOFF_TURN_THRESHOLD {
        return HandoffDecision::Continue;
    }

    let bootstrap = build_bootstrap_context(group_chat, participant, public_turns);
    HandoffDecision::Handoff {
        bootstrap_context: bootstrap,
    }
}

/// Increment the session turn count for a participant. Call this after each
/// completed turn response.
pub fn record_participant_turn(group_chat_id: &str, participant_id: &str) -> Result<(), String> {
    let mut meta = load_participant_meta(group_chat_id, participant_id);
    meta.session_turn_count = meta.session_turn_count.saturating_add(1);
    save_participant_meta(group_chat_id, participant_id, &meta)
}

/// Reset session state after a successful handoff. Bumps session_seq, zeroes
/// the turn count, and persists.
pub fn reset_session_after_handoff(
    group_chat_id: &str,
    participant_id: &str,
) -> Result<(), String> {
    let mut meta = load_participant_meta(group_chat_id, participant_id);
    meta.session_seq = meta.session_seq.saturating_add(1);
    meta.session_turn_count = 0;
    save_participant_meta(group_chat_id, participant_id, &meta)
}

// ── Bootstrap context builder ──

/// Build a bootstrap context string for session handoff.
///
/// Template structure:
/// 1. Group chat name
/// 2. Participant label + role
/// 3. Plan status (if active plan exists)
/// 4. Last 5 public turns (user_input + truncated responses)
/// 5. Own last response (truncated)
///
/// Total output is capped at ~8000 characters (~2000 tokens).
fn build_bootstrap_context(
    group_chat: &GroupChat,
    participant: &GroupChatParticipant,
    public_turns: &[GroupChatTurn],
) -> String {
    let mut sections: Vec<String> = Vec::new();

    // 1. Group chat header
    sections.push(format!("You are participating in group chat \"{}\".", group_chat.name));

    // 2. Participant identity
    sections.push(format!(
        "Your role: {} ({}).",
        participant.label, participant.role
    ));

    // 3. Plan status
    if let Some(plan_id) = &group_chat.active_plan_id {
        if let Some(plan) = storage::group_chats::get_plan_for_group_chat(&group_chat.id) {
            let task_summary = format!(
                "{} tasks ({} todo, {} in-progress, {} done, {} blocked)",
                plan.tasks.len(),
                plan.tasks.iter().filter(|t| t.status == crate::group_chat::models::TaskStatus::Todo).count(),
                plan.tasks.iter().filter(|t| t.status == crate::group_chat::models::TaskStatus::InProgress).count(),
                plan.tasks.iter().filter(|t| t.status == crate::group_chat::models::TaskStatus::Done).count(),
                plan.tasks.iter().filter(|t| t.status == crate::group_chat::models::TaskStatus::Blocked).count(),
            );
            sections.push(format!(
                "Active plan \"{}\" [{}]: {}.",
                plan.title,
                format!("{:?}", plan.status).to_lowercase(),
                task_summary,
            ));
            let _ = plan_id; // suppress unused warning
        }
    }

    // 4. Last N public turns
    let recent_turns: Vec<&GroupChatTurn> = public_turns
        .iter()
        .rev()
        .take(BOOTSTRAP_TURN_WINDOW)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if !recent_turns.is_empty() {
        let mut turn_text = String::from("Recent conversation:\n");
        for turn in &recent_turns {
            turn_text.push_str(&format!("\nUser: {}\n", &turn.user_input));
            for response in &turn.responses {
                let preview = response
                    .preview
                    .as_deref()
                    .unwrap_or("(no response)");
                let truncated = truncate_to_tokens(preview, 1500);
                turn_text.push_str(&format!("{}: {}\n", response.participant_id, truncated));
            }
        }
        sections.push(turn_text);
    }

    // 5. Own last response
    if let Some(own_last) = find_own_last_response(&participant.id, public_turns) {
        let truncated = truncate_to_tokens(&own_last, 1500);
        sections.push(format!("Your last response:\n{}", truncated));
    }

    // Join and cap at token limit
    let combined = sections.join("\n\n");
    truncate_to_tokens(&combined, BOOTSTRAP_TOKEN_CAP)
}

/// Find the last response by a specific participant across public turns.
fn find_own_last_response(
    participant_id: &str,
    public_turns: &[GroupChatTurn],
) -> Option<String> {
    for turn in public_turns.iter().rev() {
        for response in &turn.responses {
            if response.participant_id == participant_id {
                return response.preview.clone();
            }
        }
    }
    None
}

/// Truncate text to approximate token count (4 chars per token).
fn truncate_to_tokens(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_chars).collect();
    format!("{}{}", truncated, TRUNCATION_SUFFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group_chat::models::*;

    fn make_turn(idx: u64, mode: GroupChatTurnMode, sender_id: &str, target_ids: Vec<&str>) -> GroupChatTurn {
        GroupChatTurn {
            id: format!("turn-{idx}"),
            idx,
            mode,
            user_input: format!("input {idx}"),
            target_participant_ids: target_ids.into_iter().map(String::from).collect(),
            responses: vec![GroupChatResponseRef {
                participant_id: sender_id.to_string(),
                run_id: "run-x".to_string(),
                event_seq_start: 0,
                event_seq_end: 0,
                preview: Some(format!("response {idx}")),
                status: "complete".to_string(),
                error: None,
            }],
            started_at: "2026-05-13T00:00:00Z".to_string(),
            completed_at: Some("2026-05-13T00:00:01Z".to_string()),
        }
    }

    #[test]
    fn fanout_shows_all_turns() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::Fanout, "p1", vec![]),
            make_turn(2, GroupChatTurnMode::Fanout, "p2", vec![]),
        ];
        let visible = filter_visible_messages(&turns, "p1", &GroupChatTurnMode::Fanout);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn debate_shows_all_turns() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::Debate, "p1", vec![]),
            make_turn(2, GroupChatTurnMode::Debate, "p2", vec![]),
        ];
        let visible = filter_visible_messages(&turns, "p1", &GroupChatTurnMode::Debate);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn summary_shows_all_turns() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::Summary, "p1", vec![]),
            make_turn(2, GroupChatTurnMode::Summary, "p2", vec![]),
        ];
        let visible = filter_visible_messages(&turns, "p1", &GroupChatTurnMode::Summary);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn private_only_shows_sender_or_target() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::Private, "p1", vec!["p2"]),
            make_turn(2, GroupChatTurnMode::Private, "p3", vec!["p4"]),
            make_turn(3, GroupChatTurnMode::Private, "p5", vec!["p1"]),
        ];
        let visible = filter_visible_messages(&turns, "p1", &GroupChatTurnMode::Private);
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].idx, 1);
        assert_eq!(visible[1].idx, 3);
    }

    #[test]
    fn single_target_only_shows_sender_or_target() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::SingleTarget, "p1", vec!["p2"]),
            make_turn(2, GroupChatTurnMode::SingleTarget, "p3", vec!["p4"]),
        ];
        let visible = filter_visible_messages(&turns, "p2", &GroupChatTurnMode::SingleTarget);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].idx, 1);
    }

    #[test]
    fn participant_meta_defaults_to_zero() {
        let meta = ParticipantMeta::default();
        assert_eq!(meta.delivery_cursor, 0);
        assert_eq!(meta.session_turn_count, 0);
        assert_eq!(meta.session_seq, 0);
    }

    #[test]
    fn truncate_to_tokens_short_text_unchanged() {
        let text = "short text";
        assert_eq!(truncate_to_tokens(text, 100), "short text");
    }

    #[test]
    fn truncate_to_tokens_long_text_truncated() {
        let text = "a".repeat(1000);
        let result = truncate_to_tokens(&text, 100);
        assert_eq!(result.len(), 100 + TRUNCATION_SUFFIX.len());
        assert!(result.ends_with(TRUNCATION_SUFFIX));
    }

    #[test]
    fn find_own_last_response_returns_most_recent() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::Fanout, "p1", vec![]),
            make_turn(2, GroupChatTurnMode::Fanout, "p2", vec![]),
            make_turn(3, GroupChatTurnMode::Fanout, "p1", vec![]),
        ];
        let result = find_own_last_response("p1", &turns);
        assert_eq!(result.as_deref(), Some("response 3"));
    }

    #[test]
    fn find_own_last_response_none_when_no_responses() {
        let turns = vec![
            make_turn(1, GroupChatTurnMode::Fanout, "p2", vec![]),
        ];
        let result = find_own_last_response("p1", &turns);
        assert!(result.is_none());
    }
}
