use crate::models::now_iso;
use crate::room::models::{Room, RoomParticipant, RoomSummary};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

static ROOM_LOCKS: Lazy<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn room_lock(id: &str) -> Arc<Mutex<()>> {
    let mut locks = ROOM_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    locks
        .entry(id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn rooms_dir() -> PathBuf {
    super::data_dir().join("rooms")
}

fn room_dir(id: &str) -> PathBuf {
    rooms_dir().join(id)
}

fn room_file(id: &str) -> PathBuf {
    room_dir(id).join("room.json")
}

fn validate_room_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id == "." || id == ".." {
        return Err(format!("Invalid room id: {}", id));
    }
    Ok(())
}

fn save_room(room: &Room) -> Result<(), String> {
    validate_room_id(&room.id)?;
    let dir = room_dir(&room.id);
    super::ensure_dir(&dir).map_err(|e| e.to_string())?;
    let path = room_file(&room.id);
    let tmp = dir.join(format!(
        "room.json.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let json = serde_json::to_string_pretty(room).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| format!("write tmp: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("rename room file: {e}")
    })
}

pub fn create_room(name: String, description: String, cwd: Option<String>) -> Result<Room, String> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err("Room name is required".to_string());
    }

    let now = now_iso();
    let room = Room {
        id: uuid::Uuid::new_v4().to_string(),
        name: trimmed_name.to_string(),
        description: description.trim().to_string(),
        cwd: cwd.filter(|s| !s.trim().is_empty()),
        memo: String::new(),
        participants: vec![],
        created_at: now.clone(),
        updated_at: now,
    };
    save_room(&room)?;
    Ok(room)
}

pub fn get_room(id: &str) -> Option<Room> {
    if validate_room_id(id).is_err() {
        return None;
    }
    let path = room_file(id);
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn list_rooms() -> Vec<RoomSummary> {
    let entries = match fs::read_dir(rooms_dir()) {
        Ok(entries) => entries,
        Err(e) => {
            log::debug!("[rooms] cannot read rooms dir: {}", e);
            return vec![];
        }
    };

    let mut rooms = entries
        .flatten()
        .filter_map(|entry| {
            if !entry.path().is_dir() {
                return None;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            get_room(&id).map(|room| {
                let memo_preview = memo_preview(&room.memo);
                RoomSummary {
                    id: room.id,
                    name: room.name,
                    description: room.description,
                    cwd: room.cwd,
                    participant_count: room.participants.len(),
                    memo_preview,
                    updated_at: room.updated_at,
                }
            })
        })
        .collect::<Vec<_>>();
    rooms.sort_by_key(|r| std::cmp::Reverse(r.updated_at.clone()));
    rooms
}

pub fn attach_run(
    room_id: &str,
    run_id: &str,
    label: Option<String>,
    role: Option<String>,
) -> Result<Room, String> {
    validate_room_id(room_id)?;
    let run = super::runs::get_run(run_id).ok_or_else(|| format!("Run {} not found", run_id))?;
    let room_lock = room_lock(room_id);
    let _guard = room_lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut room = get_room(room_id).ok_or_else(|| format!("Room {} not found", room_id))?;

    let label = label
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let role = role.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    if let Some(participant) = room.participants.iter_mut().find(|p| p.run_id == run_id) {
        let mut changed = false;
        if let Some(label) = label {
            if participant.label != label {
                participant.label = label;
                changed = true;
            }
        }
        if let Some(role) = role {
            if participant.role != role {
                participant.role = role;
                changed = true;
            }
        }
        if changed {
            room.updated_at = now_iso();
            save_room(&room)?;
        }
        return Ok(room);
    }

    let participant = RoomParticipant {
        id: uuid::Uuid::new_v4().to_string(),
        run_id: run_id.to_string(),
        agent: run.agent,
        label: label.unwrap_or_else(|| "Claude".to_string()),
        role: role.unwrap_or_else(|| "participant".to_string()),
        joined_at: now_iso(),
    };
    room.participants.push(participant);
    room.updated_at = now_iso();
    save_room(&room)?;
    Ok(room)
}

pub fn update_memo(room_id: &str, memo: String) -> Result<Room, String> {
    validate_room_id(room_id)?;
    let room_lock = room_lock(room_id);
    let _guard = room_lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut room = get_room(room_id).ok_or_else(|| format!("Room {} not found", room_id))?;
    room.memo = memo;
    room.updated_at = now_iso();
    save_room(&room)?;
    Ok(room)
}

pub fn delete_room(room_id: &str) -> Result<(), String> {
    validate_room_id(room_id)?;
    let room_lock = room_lock(room_id);
    let _guard = room_lock.lock().unwrap_or_else(|e| e.into_inner());
    let dir = room_dir(room_id);
    if !dir.exists() {
        return Err(format!("Room {} not found", room_id));
    }
    fs::remove_dir_all(&dir).map_err(|e| format!("delete room: {e}"))
}

fn memo_preview(memo: &str) -> Option<String> {
    let trimmed = memo.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().count() > 120 {
        let end = trimmed
            .char_indices()
            .nth(120)
            .map(|(i, _)| i)
            .unwrap_or(trimmed.len());
        Some(format!("{}...", &trimmed[..end]))
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RunStatus;

    fn with_temp_data_dir<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::storage::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("OPENCOVIBE_DATA_DIR");
        std::env::set_var("OPENCOVIBE_DATA_DIR", tmp.path());
        let result = f();
        match previous {
            Some(value) => std::env::set_var("OPENCOVIBE_DATA_DIR", value),
            None => std::env::remove_var("OPENCOVIBE_DATA_DIR"),
        }
        result
    }

    #[test]
    fn room_can_be_created_listed_and_reopened() {
        with_temp_data_dir(|| {
            let room = create_room(
                "Design Review".to_string(),
                "Compare implementation options".to_string(),
                Some("D:/work/app".to_string()),
            )
            .unwrap();

            let reopened = get_room(&room.id).unwrap();
            assert_eq!(reopened.name, "Design Review");
            assert_eq!(reopened.description, "Compare implementation options");
            assert_eq!(reopened.cwd.as_deref(), Some("D:/work/app"));

            let rooms = list_rooms();
            assert_eq!(rooms.len(), 1);
            assert_eq!(rooms[0].id, room.id);
            assert_eq!(rooms[0].participant_count, 0);
        });
    }

    #[test]
    fn room_attaches_existing_run_by_reference() {
        with_temp_data_dir(|| {
            let room = create_room("Room".to_string(), "".to_string(), None).unwrap();
            let run = crate::storage::runs::create_run(
                "run-1",
                "hello",
                "D:/work/app",
                "claude",
                RunStatus::Idle,
                Some("sonnet".to_string()),
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();

            let updated = attach_run(
                &room.id,
                &run.id,
                Some("Reviewer".to_string()),
                Some("reviewer".to_string()),
            )
            .unwrap();

            assert_eq!(updated.participants.len(), 1);
            assert_eq!(updated.participants[0].run_id, "run-1");
            assert_eq!(updated.participants[0].agent, "claude");
            assert_eq!(updated.participants[0].label, "Reviewer");
            assert_eq!(
                crate::storage::runs::get_run("run-1").unwrap().prompt,
                "hello"
            );
        });
    }

    #[test]
    fn duplicate_attach_updates_existing_participant_metadata() {
        with_temp_data_dir(|| {
            let room = create_room("Room".to_string(), "".to_string(), None).unwrap();
            let run = crate::storage::runs::create_run(
                "run-1",
                "hello",
                "D:/work/app",
                "claude",
                RunStatus::Idle,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();

            attach_run(&room.id, &run.id, Some("Old".to_string()), None).unwrap();
            let updated = attach_run(
                &room.id,
                &run.id,
                Some("Reviewer".to_string()),
                Some("reviewer".to_string()),
            )
            .unwrap();

            assert_eq!(updated.participants.len(), 1);
            assert_eq!(updated.participants[0].label, "Reviewer");
            assert_eq!(updated.participants[0].role, "reviewer");
        });
    }

    #[test]
    fn delete_room_does_not_delete_referenced_run() {
        with_temp_data_dir(|| {
            let room = create_room("Room".to_string(), "".to_string(), None).unwrap();
            crate::storage::runs::create_run(
                "run-1",
                "hello",
                "D:/work/app",
                "claude",
                RunStatus::Idle,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            attach_run(&room.id, "run-1", None, None).unwrap();

            delete_room(&room.id).unwrap();

            assert!(get_room(&room.id).is_none());
            assert!(crate::storage::runs::get_run("run-1").is_some());
        });
    }

    #[test]
    fn room_memo_updates_summary_preview() {
        with_temp_data_dir(|| {
            let room = create_room("Room".to_string(), "".to_string(), None).unwrap();
            update_memo(&room.id, "Remember the API boundary.".to_string()).unwrap();

            let reopened = get_room(&room.id).unwrap();
            assert_eq!(reopened.memo, "Remember the API boundary.");
            assert_eq!(
                list_rooms()[0].memo_preview.as_deref(),
                Some("Remember the API boundary.")
            );
        });
    }
}
