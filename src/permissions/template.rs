use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;
use crate::permissions::{parse_pattern, Decision, TemplatePermissions};

pub fn template_path(catalog_root: &std::path::Path) -> std::path::PathBuf {
    catalog_root.join("permissions").join("template.json")
}

pub fn read_template(path: &Path) -> Result<TemplatePermissions> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read template at {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .context("template is not valid JSON")?;

    let raw_allow = json_string_array(value.get("permissions").and_then(|p| p.get("allow")));
    let raw_deny  = json_string_array(value.get("permissions").and_then(|p| p.get("deny")));

    let mut perms = TemplatePermissions {
        raw_allow: raw_allow.clone(),
        raw_deny: raw_deny.clone(),
        allow: vec![],
        deny: vec![],
    };

    for raw in &raw_allow {
        match parse_pattern(raw, Decision::Allow) {
            Some(p) => perms.allow.push(p),
            None => eprintln!("warning: unrecognized template pattern (allow): {}", raw),
        }
    }
    for raw in &raw_deny {
        match parse_pattern(raw, Decision::Deny) {
            Some(p) => perms.deny.push(p),
            None => eprintln!("warning: unrecognized template pattern (deny): {}", raw),
        }
    }

    Ok(perms)
}

pub fn template_sha256(path: &Path) -> Result<String> {
    let content = std::fs::read(path)?;
    let hash = Sha256::digest(&content);
    Ok(hex::encode(hash))
}

fn json_string_array(v: Option<&serde_json::Value>) -> Vec<String> {
    v.and_then(|a| a.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn fx_template() -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(br#"{
  "permissions": {
    "allow": ["Bash(git:*)", "WebSearch"],
    "deny": ["Bash(rm -rf:*)", "Edit(**/.env)"]
  }
}"#).unwrap();
        f
    }

    #[test]
    fn read_template_parses_correctly() {
        let f = fx_template();
        let perms = read_template(f.path()).unwrap();
        assert_eq!(perms.raw_allow.len(), 2);
        assert_eq!(perms.raw_deny.len(), 2);
        assert_eq!(perms.allow.len(), 2);
        assert_eq!(perms.deny.len(), 2);
    }

    #[test]
    fn template_sha256_is_deterministic() {
        let f = fx_template();
        let sha1 = template_sha256(f.path()).unwrap();
        let sha2 = template_sha256(f.path()).unwrap();
        assert_eq!(sha1, sha2);
        assert_eq!(sha1.len(), 64); // hex-encoded SHA256
    }

    #[test]
    fn read_template_missing_file_errors() {
        let result = read_template(std::path::Path::new("/nonexistent/path/to/template.json"));
        assert!(result.is_err());
    }
}
