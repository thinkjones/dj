use super::{TranslationFile, TranslationResult};
use crate::permissions::{Decision, PatternCategory, TemplatePermissions};
use anyhow::Result;
use std::path::Path;

const DESTRUCTIVE_PATTERNS: &[&str] = &["rm -rf", "git push --force", "sudo"];

pub fn translate(perms: &TemplatePermissions, dest: &Path) -> Result<TranslationResult> {
    let existing = if dest.exists() {
        std::fs::read_to_string(dest).unwrap_or_default()
    } else {
        String::new()
    };

    let mut doc = existing
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_default();

    // Check if any destructive patterns are denied
    let has_destructive_deny = perms.deny.iter().any(|p| {
        if p.category == PatternCategory::Shell {
            let cmd = p.shell_base_command().unwrap_or_default();
            DESTRUCTIVE_PATTERNS.iter().any(|d| cmd.starts_with(d))
        } else {
            false
        }
    });

    if has_destructive_deny {
        doc["default_plan_mode"] = toml_edit::value(false);
    }

    // Build comment block listing untranslatable patterns
    let untranslatable: Vec<String> = perms
        .all_patterns()
        .map(|p| {
            format!(
                "  # {}: {}",
                if p.decision == Decision::Allow {
                    "allow"
                } else {
                    "deny"
                },
                p.raw
            )
        })
        .collect();

    let comment_block = format!(
        "# >>> dj ai permissions (managed) >>>\n\
         # These would be enforced under Claude Code but Kimi Code CLI has no per-command rule support yet.\n\
         # Consider running 'kimi --plan' for read-only sessions or wrapping in a sandbox.\n\
         #\n\
         # Untranslatable patterns:\n\
         {}\n\
         # <<< dj ai permissions (managed) <<<\n",
        untranslatable.join("\n")
    );

    let content = comment_block + &doc.to_string();

    let warnings =
        vec!["kimicode: partial translation only — no native per-command rules.".to_string()];

    Ok(TranslationResult {
        files: vec![TranslationFile {
            path: dest.to_path_buf(),
            content,
        }],
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{parse_pattern, Decision, TemplatePermissions};
    use tempfile::TempDir;

    fn sample_perms() -> TemplatePermissions {
        let mut tp = TemplatePermissions {
            raw_allow: vec!["Bash(git:*)".to_string()],
            raw_deny: vec!["Bash(rm -rf:*)".to_string()],
            allow: vec![],
            deny: vec![],
        };
        tp.allow
            .push(parse_pattern("Bash(git:*)", Decision::Allow).unwrap());
        tp.deny
            .push(parse_pattern("Bash(rm -rf:*)", Decision::Deny).unwrap());
        tp
    }

    #[test]
    fn translate_produces_comment_block() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("config.toml");
        let perms = sample_perms();
        let result = translate(&perms, &dest).unwrap();
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0]
            .content
            .contains(">>> dj ai permissions (managed) >>>"));
        assert!(result.files[0].content.contains("Untranslatable patterns:"));
    }

    #[test]
    fn translate_warns_partial() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("config.toml");
        let perms = sample_perms();
        let result = translate(&perms, &dest).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("partial translation"));
    }

    #[test]
    fn translate_destructive_deny_sets_plan_mode() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("config.toml");
        let perms = sample_perms(); // has "rm -rf" deny
        let result = translate(&perms, &dest).unwrap();
        // The TOML doc will have default_plan_mode = false after the comment block
        // (comment block is prepended, so the toml is after it)
        assert!(result.files[0].content.contains("default_plan_mode"));
    }
}
