use crate::agent::provider_claude_config;
use crate::models::{
    AgentSettings, ConfiguredMcpServer, PlatformCredential, PluginOperationResult, UserSettings,
    ValidatePlatformCredentialsResponse,
};
use crate::storage;
use std::collections::HashMap;
use std::sync::atomic::Ordering;

/// Shared logic for updating user settings with token rotation detection.
/// Used by both IPC (Tauri command) and WS (dispatch) paths.
pub async fn update_user_settings_with_rotation(
    patch: serde_json::Value,
    token_ver: &std::sync::atomic::AtomicU64,
    shutdown: &tokio::sync::broadcast::Sender<()>,
    live_token: &tokio::sync::RwLock<String>,
) -> Result<UserSettings, String> {
    let old = storage::settings::get_user_settings();
    let new_settings = storage::settings::update_user_settings(patch)?;
    if old.web_server_token != new_settings.web_server_token {
        match &new_settings.web_server_token {
            Some(new_tok) => *live_token.write().await = new_tok.clone(),
            None => *live_token.write().await = String::new(),
        }
        token_ver.fetch_add(1, Ordering::Relaxed);
        log::debug!("[web_server] token rotated, updating in-memory + disconnecting WS clients");
        let _ = shutdown.send(());
    }
    Ok(new_settings)
}

#[tauri::command]
pub fn get_user_settings() -> UserSettings {
    log::debug!("[settings] get_user_settings");
    storage::settings::get_user_settings()
}

#[tauri::command]
pub async fn update_user_settings(
    patch: serde_json::Value,
    token_ver: tauri::State<'_, crate::SharedTokenVersion>,
    shutdown: tauri::State<'_, crate::WsShutdownSender>,
    live_token: tauri::State<'_, crate::SharedLiveToken>,
) -> Result<UserSettings, String> {
    log::debug!("[settings] update_user_settings");
    update_user_settings_with_rotation(patch, &token_ver, &shutdown, &live_token).await
}

#[tauri::command]
pub fn validate_platform_credentials(
    platform_credentials: Vec<PlatformCredential>,
) -> ValidatePlatformCredentialsResponse {
    log::debug!("[settings] validate_platform_credentials");
    provider_claude_config::validate_platform_credentials(&platform_credentials)
}

#[tauri::command]
pub fn get_agent_settings(agent: String) -> AgentSettings {
    log::debug!("[settings] get_agent_settings: agent={}", agent);
    storage::settings::get_agent_settings(&agent)
}

#[tauri::command]
pub fn update_agent_settings(
    agent: String,
    patch: serde_json::Value,
) -> Result<AgentSettings, String> {
    log::debug!("[settings] update_agent_settings: agent={}", agent);
    storage::settings::update_agent_settings(&agent, patch)
}

// ── Managed MCP server commands ──

#[tauri::command]
pub fn list_managed_mcp_servers() -> Vec<ConfiguredMcpServer> {
    log::debug!("[settings] list_managed_mcp_servers");
    let settings = storage::settings::get_user_settings();
    settings
        .mcp_servers
        .iter()
        .map(|(name, config)| {
            crate::storage::mcp_registry::parse_mcp_entry(name, config, "managed")
        })
        .collect()
}

#[tauri::command]
pub fn add_managed_mcp_server(
    name: String,
    config_json: String,
) -> Result<PluginOperationResult, String> {
    log::debug!("[settings] add_managed_mcp_server: name={}", name);
    crate::storage::mcp_registry::validate_name(&name)?;
    let config: serde_json::Value = serde_json::from_str(&config_json)
        .map_err(|e| format!("Invalid config JSON: {e}"))?;
    if !config.is_object() {
        return Err("Config must be a JSON object".to_string());
    }
    let mut settings = storage::settings::get_user_settings();
    let existed = settings.mcp_servers.contains_key(&name);
    settings.mcp_servers.insert(name.clone(), config);
    let patch = serde_json::json!({ "mcp_servers": settings.mcp_servers });
    storage::settings::update_user_settings(patch)?;
    let verb = if existed { "Updated" } else { "Added" };
    Ok(PluginOperationResult {
        success: true,
        message: format!("{verb} managed MCP server '{}'", name),
    })
}

#[tauri::command]
pub fn remove_managed_mcp_server(
    name: String,
) -> Result<PluginOperationResult, String> {
    log::debug!("[settings] remove_managed_mcp_server: name={}", name);
    let mut settings = storage::settings::get_user_settings();
    if settings.mcp_servers.remove(&name).is_none() {
        return Err(format!("Managed MCP server '{}' not found", name));
    }
    let patch = serde_json::json!({ "mcp_servers": settings.mcp_servers });
    storage::settings::update_user_settings(patch)?;
    Ok(PluginOperationResult {
        success: true,
        message: format!("Removed managed MCP server '{}'", name),
    })
}

// ── Managed hooks commands ──

#[tauri::command]
pub fn list_managed_hooks() -> HashMap<String, serde_json::Value> {
    log::debug!("[settings] list_managed_hooks");
    storage::settings::get_user_settings().hooks
}

#[tauri::command]
pub fn add_managed_hook(
    event: String,
    groups_json: String,
) -> Result<PluginOperationResult, String> {
    log::debug!("[settings] add_managed_hook: event={}", event);
    let groups: serde_json::Value = serde_json::from_str(&groups_json)
        .map_err(|e| format!("Invalid groups JSON: {e}"))?;
    if !groups.is_array() {
        return Err("Groups must be a JSON array".to_string());
    }
    let mut settings = storage::settings::get_user_settings();
    let existed = settings.hooks.contains_key(&event);
    settings.hooks.insert(event.clone(), groups);
    let patch = serde_json::json!({ "hooks": settings.hooks });
    storage::settings::update_user_settings(patch)?;
    let verb = if existed { "Updated" } else { "Added" };
    Ok(PluginOperationResult {
        success: true,
        message: format!("{verb} managed hooks for '{}'", event),
    })
}

#[tauri::command]
pub fn remove_managed_hook(event: String) -> Result<PluginOperationResult, String> {
    log::debug!("[settings] remove_managed_hook: event={}", event);
    let mut settings = storage::settings::get_user_settings();
    if settings.hooks.remove(&event).is_none() {
        return Err(format!("Managed hooks for '{}' not found", event));
    }
    let patch = serde_json::json!({ "hooks": settings.hooks });
    storage::settings::update_user_settings(patch)?;
    Ok(PluginOperationResult {
        success: true,
        message: format!("Removed managed hooks for '{}'", event),
    })
}

// ── Managed plugins commands ──

#[tauri::command]
pub fn list_managed_plugins() -> HashMap<String, bool> {
    log::debug!("[settings] list_managed_plugins");
    storage::settings::get_user_settings().enabled_plugins
}

#[tauri::command]
pub fn set_managed_plugin(
    name: String,
    enabled: bool,
) -> Result<PluginOperationResult, String> {
    log::debug!("[settings] set_managed_plugin: name={}, enabled={}", name, enabled);
    let mut settings = storage::settings::get_user_settings();
    let existed = settings.enabled_plugins.contains_key(&name);
    settings.enabled_plugins.insert(name.clone(), enabled);
    let patch = serde_json::json!({ "enabled_plugins": settings.enabled_plugins });
    storage::settings::update_user_settings(patch)?;
    let verb = if existed { "Updated" } else { "Set" };
    Ok(PluginOperationResult {
        success: true,
        message: format!("{} managed plugin '{}' → {}", verb, name, enabled),
    })
}

#[tauri::command]
pub fn remove_managed_plugin(name: String) -> Result<PluginOperationResult, String> {
    log::debug!("[settings] remove_managed_plugin: name={}", name);
    let mut settings = storage::settings::get_user_settings();
    if settings.enabled_plugins.remove(&name).is_none() {
        return Err(format!("Managed plugin '{}' not found", name));
    }
    let patch = serde_json::json!({ "enabled_plugins": settings.enabled_plugins });
    storage::settings::update_user_settings(patch)?;
    Ok(PluginOperationResult {
        success: true,
        message: format!("Removed managed plugin '{}'", name),
    })
}
