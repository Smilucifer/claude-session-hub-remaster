use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedCliApp {
    Claude,
    Codex,
}

impl ManagedCliApp {
    pub fn from_optional(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("claude") {
            "claude" | "cc" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            other => Err(format!(
                "Unsupported managed app '{}'. Supported: claude, codex",
                other
            )),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    pub fn dot_dir(self) -> &'static str {
        match self {
            Self::Claude => ".claude",
            Self::Codex => ".codex",
        }
    }

    pub fn user_dir(self) -> Result<PathBuf, String> {
        crate::storage::dirs_next()
            .map(|home| home.join(self.dot_dir()))
            .ok_or_else(|| "Could not determine home directory".to_string())
    }

    pub fn user_skills_dir(self) -> Result<PathBuf, String> {
        Ok(self.user_dir()?.join("skills"))
    }

    pub fn user_plugins_dir(self) -> Result<PathBuf, String> {
        Ok(self.user_dir()?.join("plugins"))
    }

    pub fn project_skills_dir(self, cwd: &str) -> Result<PathBuf, String> {
        if cwd.trim().is_empty() {
            return Err("Working directory required for project-scope skills".to_string());
        }
        let cwd_path = PathBuf::from(cwd);
        if !cwd_path.is_dir() {
            return Err(format!("Working directory does not exist: {}", cwd));
        }
        Ok(cwd_path.join(self.dot_dir()).join("skills"))
    }
}
