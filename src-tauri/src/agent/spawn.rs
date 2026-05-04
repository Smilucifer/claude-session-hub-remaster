use crate::agent::adapter::{self, AdapterSettings};

/// Build the command + args for a given agent (pipe-exec mode, not stream session)
pub fn build_agent_command(
    agent: &str,
    prompt: &str,
    settings: &AdapterSettings,
    print: bool,
) -> Result<(String, Vec<String>), String> {
    log::debug!(
        "[spawn] build_agent_command: agent={}, print={}, model={:?}, perm={:?}, allowed={}, disallowed={}",
        agent, print, settings.model, settings.permission_mode, settings.allowed_tools.len(), settings.disallowed_tools.len()
    );
    match agent {
        "claude" => {
            let mut args: Vec<String> = vec![];
            if print {
                args.push("--print".to_string());
            }

            // Use shared helper for all settings flags
            args.extend(adapter::build_settings_args(settings, print));

            if !prompt.is_empty() {
                args.push(prompt.to_string());
            }
            log::debug!("[spawn] claude command: claude {}", args.join(" "));
            Ok(("claude".to_string(), args))
        }
        "codex" => {
            let mut args: Vec<String> = vec![
                "exec".to_string(),
                "--json".to_string(),
                "--skip-git-repo-check".to_string(),
            ];
            if let Some(ref m) = settings.model {
                if !m.is_empty() {
                    args.push("--model".to_string());
                    args.push(m.to_string());
                }
            }
            if !prompt.is_empty() {
                args.push(prompt.to_string());
            }
            log::debug!("[spawn] codex command: codex {}", args.join(" "));
            Ok(("codex".to_string(), args))
        }
        "gemini" => {
            let mut args: Vec<String> = vec![];
            if let Some(ref m) = settings.model {
                if !m.is_empty() {
                    args.push("--model".to_string());
                    args.push(m.to_string());
                }
            }
            args.push("--output-format".to_string());
            args.push("text".to_string());
            if !prompt.is_empty() {
                args.push("-p".to_string());
                args.push(prompt.to_string());
            }
            log::debug!("[spawn] gemini command: gemini {}", args.join(" "));
            Ok(("gemini".to_string(), args))
        }
        _ => Err(format!(
            "Unsupported agent: {}. Supported: claude, codex, gemini",
            agent
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(model: Option<&str>) -> AdapterSettings {
        AdapterSettings {
            model: model.map(|m| m.to_string()),
            allowed_tools: vec![],
            disallowed_tools: vec![],
            permission_mode: None,
            append_system_prompt: None,
            max_budget_usd: None,
            fallback_model: None,
            system_prompt: None,
            tool_set: None,
            add_dirs: vec![],
            json_schema: None,
            include_partial_messages: true,
            cli_debug: None,
            no_session_persistence: false,
            max_turns: None,
            effort: None,
            betas: vec![],
            agents_json: None,
        }
    }

    #[test]
    fn builds_gemini_headless_command() {
        let (command, args) = build_agent_command(
            "gemini",
            "Explain this repo",
            &settings(Some("gemini-2.5-pro")),
            true,
        )
        .unwrap();

        assert_eq!(command, "gemini");
        assert_eq!(
            args,
            vec![
                "--model",
                "gemini-2.5-pro",
                "--output-format",
                "text",
                "-p",
                "Explain this repo"
            ]
        );
    }
}
