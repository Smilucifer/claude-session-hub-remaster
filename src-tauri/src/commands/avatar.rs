use std::path::Path;

#[tauri::command]
pub fn upload_character_avatar(
    character_id: String,
    file_path: String,
) -> Result<String, String> {
    log::debug!("[avatar] upload_character_avatar: character_id={}, file_path={}", character_id, file_path);

    let src = Path::new(&file_path);
    if !src.exists() {
        return Err("Source file not found".into());
    }

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let filename = format!("avatar.{}", ext);
    let dst = crate::storage::characters::char_dir(&character_id).join(&filename);

    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    std::fs::copy(&src, &dst).map_err(|e| e.to_string())?;
    Ok(dst.to_string_lossy().to_string())
}
