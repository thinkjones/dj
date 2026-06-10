//! Parse catalog/workflows/ directory into the workflow map the engine expands.
use crate::cli::workflow::{Step, Workflow};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

/// Parse all `*.md` files in `dir`. Each file stem is a workflow name.
/// Returns empty if the directory is absent or contains no `.md` files.
pub fn parse_dir(dir: &Path) -> Result<HashMap<String, Workflow>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(HashMap::new());
    };
    let mut out = HashMap::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let content = std::fs::read_to_string(&path)?;
        let wf = parse_one(&stem, &content, &filename)?;
        out.insert(stem, wf);
    }
    Ok(out)
}

fn parse_one(_name: &str, content: &str, filename: &str) -> Result<Workflow> {
    let mut wf = Workflow {
        user: vec![],
        folder: vec![],
    };
    let mut cur_scope: Option<&'static str> = None;

    for (i, raw) in content.lines().enumerate() {
        let line = raw.trim();
        let lineno = i + 1;
        if line.is_empty() {
            continue;
        }
        if line.starts_with("# ") {
            continue; // optional human-readable title, ignored
        }
        if let Some(rest) = line.strip_prefix("## ") {
            cur_scope = match rest.trim() {
                "user" => Some("user"),
                "folder" => Some("folder"),
                other => bail!("{filename}:{lineno}: unknown scope section '{other}'"),
            };
            continue;
        }
        if let Some(rest) = line.strip_prefix("- ") {
            let scope = cur_scope.ok_or_else(|| {
                anyhow::anyhow!("{filename}:{lineno}: step before any '## user|folder'")
            })?;
            let mut toks = rest.split_whitespace();
            let step = Step {
                name: toks.next().unwrap_or_default().to_string(),
                args: toks.map(|s| s.to_string()).collect(),
            };
            match scope {
                "user" => wf.user.push(step),
                _ => wf.folder.push(step),
            }
            continue;
        }
        bail!("{filename}:{lineno}: unexpected line '{line}'");
    }
    Ok(wf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_dir(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        for (name, content) in files {
            fs::write(dir.path().join(name), content).unwrap();
        }
        dir
    }

    #[test]
    fn parses_per_file_workflows() {
        let dir = make_dir(&[
            (
                "setup.md",
                "## user\n- brew\n- runtimes\n## folder\n- dev-setup\n",
            ),
            (
                "dev-setup.md",
                "## user\n- claude\n## folder\n- claude\n- apm core\n",
            ),
        ]);
        let ws = parse_dir(dir.path()).unwrap();
        assert_eq!(ws.len(), 2);
        let setup = &ws["setup"];
        assert_eq!(setup.user[0].name, "brew");
        assert_eq!(setup.user[1].name, "runtimes");
        assert_eq!(setup.folder[0].name, "dev-setup");
        let dev = &ws["dev-setup"];
        assert_eq!(dev.user[0].name, "claude");
        assert_eq!(
            dev.folder[1],
            Step {
                name: "apm".into(),
                args: vec!["core".into()]
            }
        );
    }

    #[test]
    fn missing_dir_is_empty() {
        assert!(parse_dir(Path::new("/no/such/workflows"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn ignores_non_md_files() {
        let dir = make_dir(&[
            ("setup.md", "## user\n- brew\n"),
            ("readme.txt", "not a workflow"),
        ]);
        let ws = parse_dir(dir.path()).unwrap();
        assert_eq!(ws.len(), 1);
        assert!(ws.contains_key("setup"));
    }

    #[test]
    fn title_line_ignored() {
        let dir = make_dir(&[("setup.md", "# Setup Workflow\n## user\n- brew\n")]);
        let ws = parse_dir(dir.path()).unwrap();
        assert_eq!(ws["setup"].user[0].name, "brew");
    }

    #[test]
    fn errors_on_step_before_scope() {
        let dir = make_dir(&[("bad.md", "- brew\n")]);
        assert!(parse_dir(dir.path()).is_err());
    }

    #[test]
    fn errors_on_unknown_scope() {
        let dir = make_dir(&[("bad.md", "## unknown\n- brew\n")]);
        assert!(parse_dir(dir.path()).is_err());
    }
}
