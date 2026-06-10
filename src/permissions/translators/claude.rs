use super::{TranslationFile, TranslationResult};
use crate::permissions::TemplatePermissions;
use anyhow::{Context, Result};
use std::path::Path;

pub fn translate(perms: &TemplatePermissions, dest: &Path) -> Result<TranslationResult> {
    // Read existing JSON (if any)
    let mut value: serde_json::Value = if dest.exists() {
        let content = std::fs::read_to_string(dest)
            .with_context(|| format!("cannot read {}", dest.display()))?;
        serde_json::from_str(&content).unwrap_or(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };

    // Ensure permissions object exists
    let perms_obj = value
        .as_object_mut()
        .unwrap()
        .entry("permissions")
        .or_insert(serde_json::json!({}));

    // Replace only allow and deny
    let allow_arr: serde_json::Value = perms
        .raw_allow
        .iter()
        .map(|s| serde_json::Value::String(s.clone()))
        .collect::<Vec<_>>()
        .into();
    let deny_arr: serde_json::Value = perms
        .raw_deny
        .iter()
        .map(|s| serde_json::Value::String(s.clone()))
        .collect::<Vec<_>>()
        .into();

    let obj = perms_obj.as_object_mut().unwrap();
    obj.insert("allow".to_string(), allow_arr);
    obj.insert("deny".to_string(), deny_arr);

    let content = serde_json::to_string_pretty(&value)? + "\n";

    Ok(TranslationResult {
        files: vec![TranslationFile {
            path: dest.to_path_buf(),
            content,
        }],
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
        tp.allow
            .push(parse_pattern("Bash(git:*)", Decision::Allow).unwrap());
        tp.allow
            .push(parse_pattern("WebSearch", Decision::Allow).unwrap());
        tp.deny
            .push(parse_pattern("Bash(rm -rf:*)", Decision::Deny).unwrap());
        tp
    }

    #[test]
    fn translate_creates_valid_json() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("settings.json");
        let perms = sample_perms();
        let result = translate(&perms, &dest).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.warnings.len(), 0);

        let v: serde_json::Value = serde_json::from_str(&result.files[0].content).unwrap();
        let allow = &v["permissions"]["allow"];
        assert!(allow.is_array());
        assert_eq!(allow.as_array().unwrap().len(), 2);
    }

    #[test]
    fn translate_merges_with_existing() {
        let dir = TempDir::new().unwrap();
        let dest = dir.path().join("settings.json");
        // Write existing file with extra key
        std::fs::write(
            &dest,
            r#"{"env": {"FOO": "bar"}, "permissions": {"allow": [], "deny": []}}"#,
        )
        .unwrap();

        let perms = sample_perms();
        let result = translate(&perms, &dest).unwrap();
        let v: serde_json::Value = serde_json::from_str(&result.files[0].content).unwrap();

        // Extra key preserved
        assert_eq!(v["env"]["FOO"], "bar");
        // Permissions updated
        assert_eq!(v["permissions"]["allow"].as_array().unwrap().len(), 2);
    }
}
