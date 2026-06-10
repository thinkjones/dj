use super::{TranslationFile, TranslationResult};
use crate::permissions::{Decision, PatternCategory, TemplatePermissions};
use anyhow::Result;
use std::path::Path;

pub fn translate(
    perms: &TemplatePermissions,
    scope_root: &Path,
    is_global: bool,
) -> Result<TranslationResult> {
    let settings_path = scope_root.join("settings.json");
    let ignore_path = scope_root.join(".geminiignore");
    let policy_path = scope_root.join("policies").join("00-from-template.yaml");

    // --- settings.json ---
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let root = settings.as_object_mut().unwrap();
    // Always set tools.yolo: false
    let tools = root.entry("tools").or_insert(serde_json::json!({}));
    tools
        .as_object_mut()
        .unwrap()
        .insert("yolo".to_string(), serde_json::json!(false));

    // Build excludeTools
    let mut exclude_tools: Vec<String> = vec![];
    let mut warnings: Vec<String> = vec![];

    for p in &perms.deny {
        if p.category == PatternCategory::Shell {
            if p.is_shell_embedded_wildcard() {
                // Don't add to excludeTools; will go into policy YAML
            } else {
                let cmd = p.shell_base_command().unwrap_or_default();
                exclude_tools.push(format!("ShellTool({})", cmd.trim_end_matches(" *").trim()));
            }
        }
    }

    // Preserve non-owned keys; only update excludeTools
    let exclude_arr: serde_json::Value = exclude_tools
        .iter()
        .map(|s| serde_json::Value::String(s.clone()))
        .collect::<Vec<_>>()
        .into();
    root.insert("excludeTools".to_string(), exclude_arr);

    let settings_content = serde_json::to_string_pretty(&settings)? + "\n";

    // --- .geminiignore ---
    let mut ignore_lines: Vec<String> = vec![];
    for p in perms.deny.iter().chain(perms.allow.iter()) {
        if (p.category == PatternCategory::Edit || p.category == PatternCategory::Read)
            && p.decision == Decision::Deny
        {
            ignore_lines.push(p.pattern.clone());
        }
    }
    let ignore_content = if ignore_lines.is_empty() {
        String::new()
    } else {
        ignore_lines.join("\n") + "\n"
    };

    // --- policy YAML ---
    let mut policy_rules: Vec<String> = vec![];
    policy_rules.push("# >>> dj ai permissions (managed) >>>".to_string());
    policy_rules.push("rules:".to_string());
    for p in perms.allow.iter().chain(perms.deny.iter()) {
        let decision_str = match p.decision {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
        };
        match p.category {
            PatternCategory::Shell => {
                let cmd = p.shell_base_command().unwrap_or_default();
                if p.is_shell_embedded_wildcard() {
                    policy_rules.push(format!(
                        "  - decision: {}\n    action: run_shell_command\n    match: contains\n    pattern: \"{}\"",
                        decision_str, p.pattern
                    ));
                } else {
                    policy_rules.push(format!(
                        "  - decision: {}\n    action: run_shell_command\n    match: prefix\n    pattern: \"{}\"",
                        decision_str, cmd.trim_end_matches(" *")
                    ));
                }
            }
            PatternCategory::Edit | PatternCategory::Read => {
                policy_rules.push(format!(
                    "  - decision: {}\n    action: file_access\n    glob: \"{}\"",
                    decision_str, p.pattern
                ));
            }
            PatternCategory::WebSearch => {
                policy_rules.push(format!(
                    "  - decision: {}\n    action: google_web_search",
                    decision_str
                ));
            }
            PatternCategory::WebFetch => {
                policy_rules.push(format!(
                    "  # no native domain allowlist mapping for WebFetch(domain:{})",
                    p.domain.as_deref().unwrap_or("")
                ));
            }
        }
    }
    policy_rules.push("# <<< dj ai permissions (managed) <<<".to_string());
    let policy_content = policy_rules.join("\n") + "\n";

    let mut files = vec![
        TranslationFile {
            path: settings_path,
            content: settings_content,
        },
        TranslationFile {
            path: policy_path,
            content: policy_content,
        },
    ];
    if !ignore_content.is_empty() {
        files.push(TranslationFile {
            path: ignore_path,
            content: ignore_content,
        });
    }

    if !is_global {
        warnings.push(
            "project-scope: Gemini policy-engine workspace tier is upstream-broken (issue #18186); only excludeTools and .geminiignore reliably take effect at project tier".to_string()
        );
    }

    Ok(TranslationResult { files, warnings })
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
        tp.allow
            .push(parse_pattern("Bash(git:*)", Decision::Allow).unwrap());
        tp.allow
            .push(parse_pattern("WebSearch", Decision::Allow).unwrap());
        tp.deny
            .push(parse_pattern("Bash(rm -rf:*)", Decision::Deny).unwrap());
        tp.deny
            .push(parse_pattern("Edit(**/.env)", Decision::Deny).unwrap());
        tp
    }

    #[test]
    fn translate_creates_settings_and_policy() {
        let dir = TempDir::new().unwrap();
        let perms = sample_perms();
        let result = translate(&perms, dir.path(), true).unwrap();
        // At least settings and policy files
        assert!(result.files.len() >= 2);
        // settings.json present
        let settings_file = result
            .files
            .iter()
            .find(|f| f.path.ends_with("settings.json"))
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&settings_file.content).unwrap();
        assert_eq!(v["tools"]["yolo"], false);
    }

    #[test]
    fn translate_project_scope_warns() {
        let dir = TempDir::new().unwrap();
        let perms = sample_perms();
        let result = translate(&perms, dir.path(), false).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("upstream-broken"));
    }

    #[test]
    fn translate_global_scope_no_warnings() {
        let dir = TempDir::new().unwrap();
        let perms = TemplatePermissions::default();
        let result = translate(&perms, dir.path(), true).unwrap();
        assert!(result.warnings.is_empty());
    }
}
