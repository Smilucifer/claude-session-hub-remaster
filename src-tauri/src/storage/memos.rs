use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MemoItem {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MemoScope {
    Global,
    Project { cwd: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MemoFile {
    schema_version: u32,
    items: Vec<MemoItem>,
}

const SCHEMA_VERSION: u32 = 1;

static MEMO_LOCKS: Lazy<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn empty_file() -> MemoFile {
    MemoFile {
        schema_version: SCHEMA_VERSION,
        items: vec![],
    }
}

fn project_key(cwd: &str) -> String {
    let canonical = Path::new(cwd)
        .canonicalize()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| cwd.trim().replace('\\', "/"));

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let hash = hasher.finalize();
    hash[..6]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

fn memo_path(scope: &MemoScope) -> PathBuf {
    match scope {
        MemoScope::Global => super::data_dir().join("memos").join("global.json"),
        MemoScope::Project { cwd } => super::data_dir()
            .join("projects")
            .join(project_key(cwd))
            .join("memo.json"),
    }
}

fn memo_lock(scope: &MemoScope) -> Arc<Mutex<()>> {
    let path = memo_path(scope);
    MEMO_LOCKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(path)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn load(scope: &MemoScope) -> MemoFile {
    let path = memo_path(scope);
    if !path.exists() {
        return empty_file();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            log::warn!("[memos] failed to parse {}: {}", path.display(), e);
            empty_file()
        }),
        Err(e) => {
            log::warn!("[memos] failed to read {}: {}", path.display(), e);
            empty_file()
        }
    }
}

fn write_atomic_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    let tmp = path.with_extension(format!("json.{}.tmp", uuid::Uuid::new_v4()));
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| format!("write tmp: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(first_err) if path.exists() => {
            fs::remove_file(path)
                .map_err(|e| format!("remove existing after rename failed: {e}"))?;
            fs::rename(&tmp, path).map_err(|e| {
                format!("rename after removing existing failed: {e}; first error: {first_err}")
            })
        }
        Err(e) => Err(format!("rename: {e}")),
    }
}

fn save(scope: &MemoScope, file: &MemoFile) -> Result<(), String> {
    let path = memo_path(scope);
    super::ensure_dir(path.parent().unwrap()).map_err(|e| e.to_string())?;
    write_atomic_json(&path, file)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn list_memos(scope: MemoScope) -> Vec<MemoItem> {
    let lock = memo_lock(&scope);
    let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
    load(&scope).items
}

pub fn add_memo(scope: MemoScope, text: String) -> Result<MemoItem, String> {
    let lock = memo_lock(&scope);
    let _guard = lock.lock().map_err(|e| format!("memo lock: {e}"))?;
    let mut file = load(&scope);
    let now = now_rfc3339();
    let item = MemoItem {
        id: uuid::Uuid::new_v4().to_string(),
        text,
        created_at: now.clone(),
        updated_at: now,
    };
    file.items.push(item.clone());
    save(&scope, &file)?;
    Ok(item)
}

pub fn update_memo(scope: MemoScope, id: String, text: String) -> Result<MemoItem, String> {
    let lock = memo_lock(&scope);
    let _guard = lock.lock().map_err(|e| format!("memo lock: {e}"))?;
    let mut file = load(&scope);
    let item = file
        .items
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| "Memo not found".to_string())?;
    item.text = text;
    item.updated_at = now_rfc3339();
    if item.updated_at == item.created_at {
        item.updated_at = (chrono::Utc::now() + chrono::Duration::milliseconds(1)).to_rfc3339();
    }
    let updated = item.clone();
    save(&scope, &file)?;
    Ok(updated)
}

pub fn delete_memo(scope: MemoScope, id: String) -> Result<(), String> {
    let lock = memo_lock(&scope);
    let _guard = lock.lock().map_err(|e| format!("memo lock: {e}"))?;
    let mut file = load(&scope);
    let before = file.items.len();
    file.items.retain(|item| item.id != id);
    if file.items.len() == before {
        return Err("Memo not found".to_string());
    }
    save(&scope, &file)
}

pub fn clear_memos(scope: MemoScope) -> Result<(), String> {
    let lock = memo_lock(&scope);
    let _guard = lock.lock().map_err(|e| format!("memo lock: {e}"))?;
    save(&scope, &empty_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
    fn missing_file_returns_empty_items() {
        with_temp_data_dir(|| {
            assert_eq!(list_memos(MemoScope::Global), Vec::<MemoItem>::new());
        });
    }

    #[test]
    fn corrupt_file_returns_empty_items_and_preserves_file() {
        with_temp_data_dir(|| {
            let path = crate::storage::data_dir().join("memos").join("global.json");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "{not-json").unwrap();

            assert_eq!(list_memos(MemoScope::Global), Vec::<MemoItem>::new());
            assert_eq!(fs::read_to_string(path).unwrap(), "{not-json");
        });
    }

    #[test]
    fn add_update_delete_and_clear_global_memos() {
        with_temp_data_dir(|| {
            let added = add_memo(MemoScope::Global, "first".to_string()).unwrap();
            assert_eq!(added.text, "first");
            assert!(!added.id.is_empty());
            assert_eq!(list_memos(MemoScope::Global), vec![added.clone()]);

            let updated =
                update_memo(MemoScope::Global, added.id.clone(), "changed".to_string()).unwrap();
            assert_eq!(updated.text, "changed");
            assert_eq!(updated.created_at, added.created_at);
            assert_ne!(updated.updated_at, added.updated_at);
            assert_eq!(list_memos(MemoScope::Global), vec![updated.clone()]);

            delete_memo(MemoScope::Global, updated.id.clone()).unwrap();
            assert!(list_memos(MemoScope::Global).is_empty());

            add_memo(MemoScope::Global, "again".to_string()).unwrap();
            clear_memos(MemoScope::Global).unwrap();
            assert!(list_memos(MemoScope::Global).is_empty());
        });
    }

    #[test]
    fn project_memos_are_isolated_by_cwd() {
        with_temp_data_dir(|| {
            let first = MemoScope::Project {
                cwd: "C:\\projects\\one".to_string(),
            };
            let second = MemoScope::Project {
                cwd: "C:\\projects\\two".to_string(),
            };

            let item = add_memo(first.clone(), "project one".to_string()).unwrap();

            assert_eq!(list_memos(first), vec![item]);
            assert!(list_memos(second).is_empty());
        });
    }

    #[test]
    fn update_and_delete_missing_item_return_error() {
        with_temp_data_dir(|| {
            assert!(
                update_memo(MemoScope::Global, "missing".to_string(), "x".to_string()).is_err()
            );
            assert!(delete_memo(MemoScope::Global, "missing".to_string()).is_err());
        });
    }

    #[test]
    fn concurrent_adds_to_same_scope_do_not_lose_items() {
        with_temp_data_dir(|| {
            let mut handles = vec![];
            for i in 0..24 {
                handles.push(std::thread::spawn(move || {
                    add_memo(MemoScope::Global, format!("memo {i}")).unwrap();
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }

            assert_eq!(list_memos(MemoScope::Global).len(), 24);
        });
    }
}
