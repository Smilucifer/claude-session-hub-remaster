use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::models::PlatformCredential;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderClaudeConfigMaterialized {
    pub provider_id: String,
    pub json_path: PathBuf,
    pub env: HashMap<String, String>,
}

fn provider_claude_config_path(provider_id: &str) -> PathBuf {
    crate::storage::data_dir()
        .join("provider-claude-configs")
        .join(format!("settings-{provider_id}.json"))
}

pub fn platform_to_provider_id(platform_id: &str) -> Option<&'static str> {
    match platform_id {
        "deepseek" => Some("deepseek"),
        "zhipu" | "zhipu-intl" => Some("glm"),
        "bailian" => Some("qwen"),
        "kimi" => Some("kimi"),
        _ => None,
    }
}

pub fn write_provider_claude_config(
    provider_id: &str,
    platform_id: &str,
    cred: &PlatformCredential,
) -> Result<ProviderClaudeConfigMaterialized, String> {
    let env = provider_env_from_credential(platform_id, cred)?;
    let json_value = provider_config_json_from_env(&env);
    let path = provider_claude_config_path(provider_id);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create provider config dir {}: {e}", parent.display()))?;
    }

    let serialized = serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("serialize provider config json: {e}"))?;
    fs::write(&path, serialized)
        .map_err(|e| format!("write provider config {}: {e}", path.display()))?;

    Ok(ProviderClaudeConfigMaterialized {
        provider_id: provider_id.to_string(),
        json_path: path,
        env,
    })
}

pub(crate) fn provider_env_from_credential(
    platform_id: &str,
    cred: &PlatformCredential,
) -> Result<HashMap<String, String>, String> {
    match platform_id {
        "deepseek" => build_deepseek_env(cred),
        "zhipu" | "zhipu-intl" | "bailian" | "kimi" => build_parameterized_env(platform_id, cred),
        _ => Err(format!("unsupported provider-backed Claude platform: {platform_id}")),
    }
}

fn build_deepseek_env(cred: &PlatformCredential) -> Result<HashMap<String, String>, String> {
    let api_key = cred
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "DeepSeek API key is not configured".to_string())?;

    Ok(HashMap::from([
        (
            "ANTHROPIC_BASE_URL".to_string(),
            "https://api.deepseek.com/anthropic".to_string(),
        ),
        ("ANTHROPIC_AUTH_TOKEN".to_string(), api_key),
        ("ANTHROPIC_MODEL".to_string(), "deepseek-v4-pro".to_string()),
        (
            "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
            "deepseek-v4-pro".to_string(),
        ),
        (
            "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
            "deepseek-v4-pro".to_string(),
        ),
        (
            "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
            "deepseek-v4-flash".to_string(),
        ),
        (
            "CLAUDE_CODE_SUBAGENT_MODEL".to_string(),
            "deepseek-v4-pro".to_string(),
        ),
        (
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
            "1".to_string(),
        ),
        (
            "CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK".to_string(),
            "1".to_string(),
        ),
        ("CLAUDE_CODE_EFFORT_LEVEL".to_string(), "max".to_string()),
        (
            "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS".to_string(),
            "true".to_string(),
        ),
        (
            "CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(),
            "400000".to_string(),
        ),
    ]))
}

fn build_parameterized_env(
    platform_id: &str,
    cred: &PlatformCredential,
) -> Result<HashMap<String, String>, String> {
    let api_key = cred
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{platform_id} API key is not configured"))?;
    let base_url = cred
        .base_url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{platform_id} base URL is not configured"))?;
    let model = cred
        .models
        .as_ref()
        .and_then(|models| models.first())
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| format!("{platform_id} model is not configured"))?;

    Ok(HashMap::from([
        ("ANTHROPIC_BASE_URL".to_string(), base_url),
        ("ANTHROPIC_AUTH_TOKEN".to_string(), api_key),
        ("ANTHROPIC_MODEL".to_string(), model.clone()),
        ("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), model.clone()),
        ("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), model.clone()),
        ("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), model),
    ]))
}

fn provider_config_json_from_env(env: &HashMap<String, String>) -> Value {
    let mut env_obj = Map::new();
    for (key, value) in env {
        env_obj.insert(key.clone(), Value::String(value.clone()));
    }

    json!({
        "env": env_obj,
        "permissions": {
            "defaultMode": "bypassPermissions"
        },
        "enabledPlugins": {
            "superpowers@claude-plugins-official": true
        },
        "includeCoAuthoredBy": false,
        "thinking": false,
        "skipDangerousModePermissionPrompt": true,
        "autoUpdatesChannel": "latest",
        "language": "简体中文"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PlatformCredential;

    fn cred(platform_id: &str, api_key: &str, base_url: Option<&str>, model: Option<&str>) -> PlatformCredential {
        PlatformCredential {
            platform_id: platform_id.to_string(),
            api_key: Some(api_key.to_string()),
            base_url: base_url.map(|value| value.to_string()),
            auth_env_var: Some("ANTHROPIC_AUTH_TOKEN".to_string()),
            name: Some(platform_id.to_string()),
            models: model.map(|value| vec![value.to_string()]),
            extra_env: None,
        }
    }

    #[test]
    fn maps_platform_ids_to_provider_ids() {
        assert_eq!(platform_to_provider_id("deepseek"), Some("deepseek"));
        assert_eq!(platform_to_provider_id("zhipu"), Some("glm"));
        assert_eq!(platform_to_provider_id("zhipu-intl"), Some("glm"));
        assert_eq!(platform_to_provider_id("bailian"), Some("qwen"));
        assert_eq!(platform_to_provider_id("kimi"), Some("kimi"));
        assert_eq!(platform_to_provider_id("anthropic"), None);
    }

    #[test]
    fn builds_deepseek_env() {
        let env = build_deepseek_env(&cred("deepseek", "sk-deepseek", None, None)).unwrap();
        assert_eq!(env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str), Some("sk-deepseek"));
        assert_eq!(env.get("ANTHROPIC_MODEL").map(String::as_str), Some("deepseek-v4-pro"));
        assert_eq!(
            env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL").map(String::as_str),
            Some("deepseek-v4-flash")
        );
    }

    #[test]
    fn builds_parameterized_env() {
        let env = build_parameterized_env(
            "bailian",
            &cred(
                "bailian",
                "sk-qwen",
                Some("https://coding.dashscope.aliyuncs.com/apps/anthropic"),
                Some("qwen3.5-plus"),
            ),
        )
        .unwrap();
        assert_eq!(env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str), Some("sk-qwen"));
        assert_eq!(env.get("ANTHROPIC_MODEL").map(String::as_str), Some("qwen3.5-plus"));
        assert_eq!(
            env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://coding.dashscope.aliyuncs.com/apps/anthropic")
        );
    }

    #[test]
    fn writes_provider_specific_json_file_under_data_dir() {
        let _guard = crate::storage::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("OPENCOVIBE_DATA_DIR");
        std::env::set_var("OPENCOVIBE_DATA_DIR", tmp.path());

        let result = write_provider_claude_config(
            "qwen",
            "bailian",
            &cred(
                "bailian",
                "sk-qwen",
                Some("https://coding.dashscope.aliyuncs.com/apps/anthropic"),
                Some("qwen3.5-plus"),
            ),
        )
        .unwrap();

        assert!(result
            .json_path
            .ends_with(std::path::Path::new("provider-claude-configs/settings-qwen.json")));
        let content = fs::read_to_string(&result.json_path).unwrap();
        assert!(content.contains("ANTHROPIC_BASE_URL"));
        assert!(content.contains("qwen3.5-plus"));

        match previous {
            Some(value) => std::env::set_var("OPENCOVIBE_DATA_DIR", value),
            None => std::env::remove_var("OPENCOVIBE_DATA_DIR"),
        }
    }
}
