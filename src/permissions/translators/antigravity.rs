use anyhow::Result;
use std::path::Path;
use serde_json::{json, Value};
use crate::permissions::{PatternCategory, TemplatePermissions};
use super::{TranslationFile, TranslationResult};

pub fn translate(perms: &TemplatePermissions, dest: &Path, is_cli: bool) -> Result<TranslationResult> {
    // Read existing
    let mut root: Value = if dest.exists() {
        let content = std::fs::read_to_string(dest)?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    let mut allow_cmds: Vec<Value> = vec![];
    let mut deny_cmds: Vec<Value> = vec![];
    let mut warnings: Vec<String> = vec![];

    for p in &perms.allow {
        if p.category == PatternCategory::Shell {
            if let Some(cmd) = p.shell_base_command() {
                if p.is_shell_embedded_wildcard() {
                    warnings.push(format!(
                        "wildcard pattern {} translated to base-command allow; review manually", p.raw
                    ));
                }
                allow_cmds.push(json!(format!("command({})", cmd)));
            }
        }
        if p.category == PatternCategory::WebSearch || p.category == PatternCategory::WebFetch {
            warnings.push(format!(
                "{}: governed by Browser URL Allowlist (out of scope); configure in Browser settings panel", p.raw
            ));
        }
    }

    let mut has_file_deny_globs = false;
    for p in &perms.deny {
        if p.category == PatternCategory::Shell {
            if let Some(cmd) = p.shell_base_command() {
                if p.is_shell_embedded_wildcard() {
                    warnings.push(format!(
                        "wildcard pattern {} translated to base-command deny; review manually", p.raw
                    ));
                }
                deny_cmds.push(json!(format!("command({})", cmd)));
            }
        }
        if p.category == PatternCategory::Edit || p.category == PatternCategory::Read {
            has_file_deny_globs = true;
        }
    }

    let permissions_obj = json!({
        "allow": allow_cmds,
        "deny": deny_cmds,
        "ask": []
    });

    let root_obj = root.as_object_mut().unwrap();
    root_obj.insert("permissions".to_string(), permissions_obj);

    if is_cli {
        // antigravity-cli: set toolPermission if any deny rules
        if perms.has_any_deny() {
            root_obj.insert("toolPermission".to_string(), json!("request-review"));
        }
    } else {
        // IDE: terminalExecutionPolicy and enableTerminalSandbox
        if perms.has_any_deny() {
            root_obj.insert("terminalExecutionPolicy".to_string(), json!("auto"));
        }
        if has_file_deny_globs {
            root_obj.insert("enableTerminalSandbox".to_string(), json!(true));
            warnings.push(format!(
                "{} file-glob rules cannot be expressed in Antigravity native permissions; terminal sandbox enabled as best-effort defense",
                perms.deny.iter().filter(|p| p.category == PatternCategory::Edit || p.category == PatternCategory::Read).count()
            ));
        } else {
            root_obj.insert("enableTerminalSandbox".to_string(), json!(false));
        }
    }

    let content = serde_json::to_string_pretty(&root)? + "\n";
    Ok(TranslationResult {
        files: vec![TranslationFile { path: dest.to_path_buf(), content }],
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
            raw_allow: vec!["Bash(git:*)".to_string(), "WebSearch".to_string()],
            raw_deny: vec!["Bash(rm -rf:*)".to_string(), "Edit(**/.env)".to_string()],
            allow: vec![],
            deny: vec![],
        };
        tp.allow.push(parse_pattern("Bash(git:*)", Decision::Allow).unwrap());
        tp.allow.push(parse_pattern("WebSearch", Decision::Allow).unwrap());
        tp.deny.push(parse_pattern("Bash(rm -rf:*)", Decision::Deny).unwrap());
        tp.deny.push(parse_pattern("Edit(**/.env)", Decision::Deny).unwrap());
        tp
    }

    #[test]
    fn translate_ide_adds_terminal_sandbox() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("settings.json");
        let perms = sample_perms();
        let result = translate(&perms, &dest, false).unwrap();
        assert_eq!(result.files.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&result.files[0].content).unwrap();
        assert_eq!(v["enableTerminalSandbox"], true);
        assert_eq!(v["terminalExecutionPolicy"], "auto");
    }

    #[test]
    fn translate_cli_adds_tool_permission() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("settings.json");
        let perms = sample_perms();
        let result = translate(&perms, &dest, true).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result.files[0].content).unwrap();
        assert_eq!(v["toolPermission"], "request-review");
    }

    #[test]
    fn translate_websearch_emits_warning() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("settings.json");
        let perms = sample_perms();
        let result = translate(&perms, &dest, false).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("Browser URL Allowlist")));
    }
}
