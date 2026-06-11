#![allow(dead_code)]

pub mod ai;

// ── Brew ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum BrewKind {
    Formula,
    Cask,
    Tap,
}

#[derive(Debug, Clone)]
pub struct BrewEntry {
    pub name: String,
    pub kind: BrewKind,
    pub description: String,
    pub examples: Vec<String>,
    pub arch: Option<String>,
    pub bin: Option<String>,
    pub zsh_plugin: Option<String>,
}

// ── Runtimes ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Runtime {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct PackageManager {
    pub name: String,
    pub description: String,
    pub install_script: String,
}

// ── Script installs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScriptInstall {
    pub name: String,
    pub install_script: String,
    pub description: String,
}

// ── Shell functions ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ShellFunction {
    pub name: String,
    pub body: String,
    pub description: String,
}

// ── Symlinks ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Symlink {
    pub name: String,
    pub destination: String,
    pub target: String,
}

/// Expand a leading `~/` to the user's home directory.
pub fn expand_tilde(path: &str) -> std::path::PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => match dirs::home_dir() {
            Some(home) => home.join(rest),
            None => std::path::PathBuf::from(path),
        },
        None => std::path::PathBuf::from(path),
    }
}

// ── AI / agentic stack ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClaudeSettings {
    pub name: String,
    pub description: String,
    pub settings_path: String,
    pub required_variables: Vec<String>,
    pub json_body: String,
}

#[derive(Debug, Clone)]
pub struct ApmStack {
    pub name: String,
    pub apm_yml_content: String,
}

/// Returns true if the entry's arch restriction matches the current machine.
pub fn arch_compatible(arch: &Option<String>) -> bool {
    match arch.as_deref() {
        None => true,
        Some("arm64") => std::env::consts::ARCH == "aarch64",
        Some("x86_64") => std::env::consts::ARCH == "x86_64",
        _ => true,
    }
}
