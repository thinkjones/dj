//! Parse catalog/workflows.md into the workflow map the engine expands.
use crate::cli::workflow::{Step, Workflow};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

/// Parse the workflows file. Returns empty if the file is absent.
pub fn parse(path: &Path) -> Result<HashMap<String, Workflow>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(HashMap::new());
    };
    let mut out: HashMap<String, Workflow> = HashMap::new();
    let mut cur_name: Option<String> = None;
    let mut cur_scope: Option<&'static str> = None; // "user" | "folder"

    for (i, raw) in content.lines().enumerate() {
        let line = raw.trim();
        let lineno = i + 1;
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            let name = rest.trim().to_string();
            cur_name = Some(name.clone());
            cur_scope = None;
            out.entry(name.clone()).or_insert_with(|| Workflow {
                user: vec![],
                folder: vec![],
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            cur_scope = match rest.trim() {
                "user" => Some("user"),
                "folder" => Some("folder"),
                other => bail!("workflows.md:{lineno}: unknown scope section '{other}'"),
            };
            continue;
        }
        if line.starts_with("# ") {
            continue; // H1 title, ignored
        }
        if let Some(rest) = line.strip_prefix("- ") {
            let name = cur_name.clone().ok_or_else(|| {
                anyhow::anyhow!("workflows.md:{lineno}: step before any '## workflow'")
            })?;
            let scope = cur_scope.ok_or_else(|| {
                anyhow::anyhow!("workflows.md:{lineno}: step before any '### user|folder'")
            })?;
            let mut toks = rest.split_whitespace();
            let step = Step {
                name: toks.next().unwrap_or_default().to_string(),
                args: toks.map(|s| s.to_string()).collect(),
            };
            let wf = out.get_mut(&name).expect("workflow inserted at ## line");
            match scope {
                "user" => wf.user.push(step),
                _ => wf.folder.push(step),
            };
            continue;
        }
        bail!("workflows.md:{lineno}: unexpected line '{line}'");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn parse_str(s: &str) -> Result<HashMap<String, Workflow>> {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(s.as_bytes()).unwrap();
        parse(f.path())
    }

    #[test]
    fn parses_scoped_steps_with_args() {
        let ws = parse_str("# Workflows\n## dev-setup\n### user\n- apm core\n- claude\n### folder\n- permissions\n- claude\n").unwrap();
        let dev = &ws["dev-setup"];
        assert_eq!(
            dev.user[0],
            Step {
                name: "apm".into(),
                args: vec!["core".into()]
            }
        );
        assert_eq!(dev.user[1].name, "claude");
        assert_eq!(dev.folder[0].name, "permissions");
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(parse(Path::new("/no/such/workflows.md"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn errors_on_step_before_scope() {
        assert!(parse_str("## w\n- brew\n").is_err());
    }
}
