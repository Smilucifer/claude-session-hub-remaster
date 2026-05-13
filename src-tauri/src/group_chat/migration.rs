use crate::models::AiCharacter;
use crate::storage::{self, group_chats, settings};
use std::fs;

/// Scan all group chats for participants with empty `character_id`.
/// Match `participant.label` against `AiCharacter.label` (case-insensitive).
/// Set `character_id = character.id` or `"__orphan__"` if no match.
pub fn migrate_participant_character_ids() -> Result<usize, String> {
    let all_settings = settings::load();
    let characters: Vec<AiCharacter> = all_settings.user.ai_characters;
    let ids = list_group_chat_ids()?;

    let mut migrated = 0;
    for id in &ids {
        let mut meta = match group_chats::get_group_chat(id) {
            Some(m) => m,
            None => continue,
        };

        let mut changed = false;
        for participant in &mut meta.participants {
            if !participant.character_id.is_empty() {
                continue;
            }
            // Match by label (case-insensitive)
            if let Some(ch) = characters
                .iter()
                .find(|c| c.label.eq_ignore_ascii_case(&participant.label))
            {
                participant.character_id = ch.id.clone();
                changed = true;
            } else {
                participant.character_id = "__orphan__".to_string();
                changed = true;
            }
        }

        if changed {
            group_chats::save_group_chat(&meta)?;
            migrated += 1;
        }
    }

    Ok(migrated)
}

fn list_group_chat_ids() -> Result<Vec<String>, String> {
    let dir = storage::data_dir().join("group-chats");
    let entries = fs::read_dir(&dir).map_err(|e| format!("read group-chats dir: {e}"))?;
    let mut ids = Vec::new();
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                ids.push(name.to_string());
            }
        }
    }
    Ok(ids)
}
