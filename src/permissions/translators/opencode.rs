use anyhow::Result;
use std::path::Path;
use serde_json::{json, Map, Value};
use crate::permissions::{PatternCategory, TemplatePermissions};
use super::{TranslationFile, TranslationResult};

pub fn translate(perms: &TemplatePermissions, dest: &Path) -> Result<TranslationResult> {
    // Read existing JSON
    let mut root: Value = if dest.exists() {
        let content = std::fs::read_to_string(dest)?;
        serde_json::from_str(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };

    // Build bash map: "*": "ask" first, then specifics
    let mut bash_map = Map::new();
    bash_map.insert("*".to_string(), json!("ask"));

    for p in &perms.allow {
        if p.category == PatternCategory::Shell {
            let key = p.pattern.clone();
            bash_map.insert(key, json!("allow"));
        }
    }
    for p in &perms.deny {
        if p.category == PatternCategory::Shell {
            let key = p.pattern.clone();
            bash_map.insert(key, json!("deny"));
        }
    }

    // webfetch policy
    let has_webfetch_allow = perms.allow.iter().any(|p| p.category == PatternCategory::WebFetch);
    let has_webfetch_deny  = perms.deny.iter().any(|p| p.category == PatternCategory::WebFetch);
    let webfetch = if has_webfetch_allow { "allow" } else if has_webfetch_deny { "deny" } else { "ask" };

    // websearch
    let has_websearch_allow = perms.allow.iter().any(|p| p.category == PatternCategory::WebSearch);

    let mut permission = Map::new();
    permission.insert("bash".to_string(), Value::Object(bash_map));
    permission.insert("edit".to_string(), json!("ask"));
    permission.insert("webfetch".to_string(), json!(webfetch));
    if has_websearch_allow {
        permission.insert("websearch".to_string(), json!("allow"));
    }

    // Merge into existing root (preserve non-permission keys)
    let root_obj = root.as_object_mut().unwrap();
    if !root_obj.contains_key("$schema") {
        root_obj.insert("$schema".to_string(), json!("https://opencode.ai/config.json"));
    }
    root_obj.insert("permission".to_string(), Value::Object(permission));

    let content = serde_json::to_string_pretty(&root)? + "\n";
    Ok(TranslationResult {
        files: vec![TranslationFile { path: dest.to_path_buf(), content }],
        warnings: vec![],
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
            raw_deny: vec!["Bash(rm -rf:*)".to_string()],
            allow: vec![],
            deny: vec![],
        };
        tp.allow.push(parse_pattern("Bash(git:*)", Decision::Allow).unwrap());
        tp.allow.push(parse_pattern("WebSearch", Decision::Allow).unwrap());
        tp.deny.push(parse_pattern("Bash(rm -rf:*)", Decision::Deny).unwrap());
        tp
    }

    #[test]
    fn translate_creates_valid_opencode_json() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("opencode.json");
        let perms = sample_perms();
        let result = translate(&perms, &dest).unwrap();
        assert_eq!(result.files.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&result.files[0].content).unwrap();
        assert!(v["permission"]["bash"].is_object());
        assert_eq!(v["permission"]["bash"]["*"], "ask");
        assert_eq!(v["permission"]["bash"]["git *"], "allow");
        assert_eq!(v["permission"]["websearch"], "allow");
    }

    #[test]
    fn translate_merges_existing_schema() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("opencode.json");
        std::fs::write(&dest, r#"{"$schema":"https://opencode.ai/config.json","theme":"dark"}"#).unwrap();
        let perms = sample_perms();
        let result = translate(&perms, &dest).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result.files[0].content).unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["$schema"], "https://opencode.ai/config.json");
    }
}
