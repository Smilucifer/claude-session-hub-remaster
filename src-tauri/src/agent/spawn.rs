use crate::agent::adapter::{self, AdapterSettings};

fn native_command(default_command: &str, settings: &AdapterSettings) -> String {
    settings
        .command_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default_command)
        .to_string()
}

fn native_yolo_enabled(settings: &AdapterSettings) -> bool {
    settings.yolo_mode.unwrap_or(false)
}

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
            if native_yolo_enabled(settings) {
                args.push("--dangerously-bypass-approvals-and-sandbox".to_string());
            }
            if let Some(ref m) = settings.model {
                if !m.is_empty() {
                    args.push("--model".to_string());
                    args.push(m.to_string());
                }
            }
            for dir in &settings.add_dirs {
                args.push("--add-dir".to_string());
                args.push(dir.to_string());
            }
            if settings.no_session_persistence {
                args.push("--ephemeral".to_string());
            }
            args.extend(settings.extra_args.iter().cloned());
            if !prompt.is_empty() {
                args.push(prompt.to_string());
            }
            log::debug!("[spawn] codex command: codex {}", args.join(" "));
            Ok((native_command("codex", settings), args))
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
            if native_yolo_enabled(settings) {
                args.push("--yolo".to_string());
            }
            for dir in &settings.add_dirs {
                args.push("--include-directories".to_string());
                args.push(dir.to_string());
            }
            args.extend(settings.extra_args.iter().cloned());
            if !prompt.is_empty() {
                args.push("-p".to_string());
                args.push(prompt.to_string());
            }
            log::debug!("[spawn] gemini command: gemini {}", args.join(" "));
            Ok((native_command("gemini", settings), args))
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
            command_path: None,
            extra_args: vec![],
            yolo_mode: None,
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

    #[test]
    fn builds_codex_yolo_and_add_dir_args() {
        let mut s = settings(Some("gpt-5.5"));
        s.add_dirs = vec!["D:/shared".to_string()];
        s.yolo_mode = Some(true);

        let (command, args) =
            build_agent_command("codex", "Fix it", &s, true).expect("codex command");

        assert_eq!(command, "codex");
        assert!(args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
        assert!(args.windows(2).any(|w| w == ["--add-dir", "D:/shared"]));
        assert!(args.windows(2).any(|w| w == ["--model", "gpt-5.5"]));
        assert_eq!(args.last().map(String::as_str), Some("Fix it"));
    }

    #[test]
    fn builds_gemini_yolo_and_include_directories_args() {
        let mut s = settings(Some("gemini-2.5-pro"));
        s.add_dirs = vec!["D:/shared".to_string()];
        s.yolo_mode = Some(true);

        let (_command, args) =
            build_agent_command("gemini", "Explain this repo", &s, true).expect("gemini command");

        assert!(args.contains(&"--yolo".to_string()));
        assert!(args
            .windows(2)
            .any(|w| w == ["--include-directories", "D:/shared"]));
        assert_eq!(args.last().map(String::as_str), Some("Explain this repo"));
    }

    #[test]
    fn native_agents_do_not_inherit_global_permission_bypass() {
        let mut s = settings(None);
        s.permission_mode = Some("bypassPermissions".to_string());

        let (_codex_command, codex_args) =
            build_agent_command("codex", "Fix it", &s, true).expect("codex command");
        assert!(!codex_args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));

        let (_gemini_command, gemini_args) =
            build_agent_command("gemini", "Fix it", &s, true).expect("gemini command");
        assert!(!gemini_args.contains(&"--yolo".to_string()));
    }
}
