pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod kimicode;
pub mod opencode;

use std::path::PathBuf;

pub struct TranslationFile {
    pub path: PathBuf,
    pub content: String,
}

pub struct TranslationResult {
    pub files: Vec<TranslationFile>,
    pub warnings: Vec<String>,
}
