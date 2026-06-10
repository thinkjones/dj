use dirs::home_dir;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetId {
    Claude,
    Gemini,
    Antigravity,
    AntigravityCli,
    OpenCode,
    Codex,
    KimiCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportLevel {
    Full,
    Partial,
    Minimal,
}

impl std::fmt::Display for SupportLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Partial => write!(f, "partial"),
            Self::Minimal => write!(f, "minimal"),
        }
    }
}

pub struct Target {
    pub id: TargetId,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub support: SupportLevel,
    pub binary: &'static str,
    pub notes: &'static str,
}

pub fn all_targets() -> Vec<Target> {
    vec![
        Target {
            id: TargetId::Claude,
            name: "claude",
            aliases: &[],
            support: SupportLevel::Full,
            binary: "claude",
            notes: "identity merge",
        },
        Target {
            id: TargetId::Gemini,
            name: "gemini",
            aliases: &[],
            support: SupportLevel::Full,
            binary: "gemini",
            notes: "deny via excludeTools is best-effort",
        },
        Target {
            id: TargetId::Antigravity,
            name: "antigravity",
            aliases: &[],
            support: SupportLevel::Partial,
            binary: "antigravity",
            notes: "full for shell, partial for file globs",
        },
        Target {
            id: TargetId::AntigravityCli,
            name: "antigravity-cli",
            aliases: &["agy"],
            support: SupportLevel::Partial,
            binary: "agy",
            notes: "global only; full for shell, partial for file globs",
        },
        Target {
            id: TargetId::OpenCode,
            name: "opencode",
            aliases: &[],
            support: SupportLevel::Full,
            binary: "opencode",
            notes: "",
        },
        Target {
            id: TargetId::Codex,
            name: "codex",
            aliases: &[],
            support: SupportLevel::Partial,
            binary: "codex",
            notes: "no per-command rules; mapped to sandbox policy",
        },
        Target {
            id: TargetId::KimiCode,
            name: "kimicode",
            aliases: &["kimi"],
            support: SupportLevel::Minimal,
            binary: "kimi",
            notes: "no native allow/deny; managed comment block only",
        },
    ]
}

pub fn resolve_target(name: &str) -> Option<TargetId> {
    for t in all_targets() {
        if t.name == name {
            return Some(t.id);
        }
        for alias in t.aliases {
            if *alias == name {
                return Some(t.id);
            }
        }
    }
    None
}

pub fn has_project_scope(id: TargetId) -> bool {
    id != TargetId::AntigravityCli
}

/// Check if the project marker exists in `cwd`.
pub fn project_marker_exists(id: TargetId, cwd: &Path) -> bool {
    match id {
        TargetId::Claude => cwd.join(".claude").is_dir(),
        TargetId::Gemini => cwd.join(".gemini").is_dir(),
        TargetId::Antigravity => cwd.join(".antigravity").is_dir(),
        TargetId::AntigravityCli => false, // no project scope
        TargetId::OpenCode => cwd.join("opencode.json").is_file(),
        TargetId::Codex => cwd.join(".codex").is_dir(),
        TargetId::KimiCode => cwd.join(".kimi").is_dir(),
    }
}

/// Expected project marker path (for error messages).
pub fn project_marker_path(id: TargetId, cwd: &Path) -> String {
    match id {
        TargetId::Claude => cwd.join(".claude").display().to_string(),
        TargetId::Gemini => cwd.join(".gemini").display().to_string(),
        TargetId::Antigravity => cwd.join(".antigravity").display().to_string(),
        TargetId::AntigravityCli => "(no project scope)".to_string(),
        TargetId::OpenCode => cwd.join("opencode.json").display().to_string(),
        TargetId::Codex => cwd.join(".codex").display().to_string(),
        TargetId::KimiCode => cwd.join(".kimi").display().to_string(),
    }
}

/// Global destination path(s) for a target (primary config file).
pub fn global_destination(id: TargetId) -> Option<PathBuf> {
    let home = home_dir()?;
    Some(match id {
        TargetId::Claude => home.join(".claude").join("settings.json"),
        TargetId::Gemini => home.join(".gemini").join("settings.json"),
        TargetId::Antigravity => antigravity_global_settings_path()?,
        TargetId::AntigravityCli => home
            .join(".gemini")
            .join("antigravity-cli")
            .join("settings.json"),
        TargetId::OpenCode => home.join(".config").join("opencode").join("opencode.json"),
        TargetId::Codex => home.join(".codex").join("config.toml"),
        TargetId::KimiCode => home.join(".kimi").join("config.toml"),
    })
}

/// Project scope destination file.
pub fn project_destination(id: TargetId, cwd: &Path) -> Option<PathBuf> {
    match id {
        TargetId::Claude => Some(cwd.join(".claude").join("settings.json")),
        TargetId::Gemini => Some(cwd.join(".gemini").join("settings.json")),
        TargetId::Antigravity => Some(cwd.join(".antigravity").join("settings.json")),
        TargetId::AntigravityCli => None,
        TargetId::OpenCode => Some(cwd.join("opencode.json")),
        TargetId::Codex => Some(cwd.join(".codex").join("config.toml")),
        TargetId::KimiCode => Some(cwd.join(".kimi").join("config.toml")),
    }
}

fn antigravity_global_settings_path() -> Option<PathBuf> {
    let home = home_dir()?;
    #[cfg(target_os = "macos")]
    return Some(home.join("Library/Application Support/Antigravity/User/settings.json"));
    #[cfg(target_os = "linux")]
    return Some(home.join(".config/Antigravity/User/settings.json"));
    #[cfg(target_os = "windows")]
    return Some(
        PathBuf::from(std::env::var("APPDATA").unwrap_or_default())
            .join("Antigravity/User/settings.json"),
    );
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return Some(home.join(".config/Antigravity/User/settings.json"));
}

pub fn is_global_installed(id: TargetId) -> bool {
    let target = all_targets().into_iter().find(|t| t.id == id).unwrap();
    // Check binary on PATH
    if which::which(target.binary).is_ok() {
        return true;
    }
    // Check destination file exists
    if let Some(dest) = global_destination(id) {
        if dest.exists() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_target_by_name() {
        assert_eq!(resolve_target("claude"), Some(TargetId::Claude));
        assert_eq!(resolve_target("gemini"), Some(TargetId::Gemini));
        assert_eq!(resolve_target("opencode"), Some(TargetId::OpenCode));
    }

    #[test]
    fn resolve_target_by_alias() {
        assert_eq!(resolve_target("agy"), Some(TargetId::AntigravityCli));
        assert_eq!(resolve_target("kimi"), Some(TargetId::KimiCode));
    }

    #[test]
    fn resolve_target_unknown() {
        assert_eq!(resolve_target("nonexistent"), None);
    }

    #[test]
    fn antigravity_cli_has_no_project_scope() {
        assert!(!has_project_scope(TargetId::AntigravityCli));
        assert!(has_project_scope(TargetId::Claude));
        assert!(has_project_scope(TargetId::Gemini));
    }

    #[test]
    fn project_marker_exists_checks_correct_paths() {
        let dir = tempfile::TempDir::new().unwrap();
        let cwd = dir.path();

        // Nothing created yet
        assert!(!project_marker_exists(TargetId::Claude, cwd));
        assert!(!project_marker_exists(TargetId::Gemini, cwd));

        // Create .claude dir
        std::fs::create_dir(cwd.join(".claude")).unwrap();
        assert!(project_marker_exists(TargetId::Claude, cwd));
        assert!(!project_marker_exists(TargetId::Gemini, cwd));

        // OpenCode checks for file
        assert!(!project_marker_exists(TargetId::OpenCode, cwd));
        std::fs::write(cwd.join("opencode.json"), "{}").unwrap();
        assert!(project_marker_exists(TargetId::OpenCode, cwd));
    }

    #[test]
    fn all_targets_has_seven_entries() {
        assert_eq!(all_targets().len(), 7);
    }

    #[test]
    fn support_level_display() {
        assert_eq!(SupportLevel::Full.to_string(), "full");
        assert_eq!(SupportLevel::Partial.to_string(), "partial");
        assert_eq!(SupportLevel::Minimal.to_string(), "minimal");
    }
}
