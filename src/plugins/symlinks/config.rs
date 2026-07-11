use crate::catalog::Symlink;
use anyhow::{bail, Result};
use std::path::Path;

pub fn parse(path: &Path) -> Result<Vec<Symlink>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(vec![]);
    };
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "symlinks.md".to_string());

    let mut out: Vec<Symlink> = Vec::new();
    let mut current_dest: Option<String> = None;

    for (i, raw) in content.lines().enumerate() {
        let line = raw.trim();
        let lineno = i + 1;

        if line.is_empty() {
            continue;
        }

        if line.starts_with("## ") {
            continue;
        }

        if let Some(rest) = line.strip_prefix("# ") {
            let dest = rest.trim();
            if !dest.starts_with("~/") {
                bail!("{file}:{lineno}: destination heading must be a `~/` path, got `{dest}`");
            }
            current_dest = Some(dest.to_string());
            continue;
        }

        if line.starts_with("~/") {
            let Some(dest) = current_dest.clone() else {
                bail!("{file}:{lineno}: `{line}` appears before any `# ~/destination` heading");
            };
            let name = link_name(line);
            if out.iter().any(|s| s.destination == dest && s.name == name) {
                bail!("{file}:{lineno}: duplicate link name `{name}` under `{dest}`");
            }
            out.push(Symlink {
                name,
                destination: dest,
                target: line.to_string(),
            });
            continue;
        }

        bail!(
            "{file}:{lineno}: unexpected line `{line}` \
             (expected `# ~/destination`, `## section`, or `~/path`)"
        );
    }

    Ok(out)
}

fn link_name(target: &str) -> String {
    let rel = target.strip_prefix("~/").unwrap_or(target);
    let segs: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    match segs.as_slice() {
        [.., parent, leaf] => format!("{parent}-{leaf}"),
        [single] => (*single).to_string(),
        [] => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn parse_str(s: &str) -> Result<Vec<Symlink>> {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(s.as_bytes()).unwrap();
        parse(f.path())
    }

    #[test]
    fn flattens_parent_and_leaf() {
        assert_eq!(link_name("~/dev/repos/fs-website/docs"), "fs-website-docs");
        assert_eq!(link_name("~/dev/repos/dj"), "dev-dj");
        assert_eq!(link_name("~/.local"), ".local");
    }

    #[test]
    fn groups_targets_under_destination() {
        let links = parse_str(
            "# ~/dev/repos\n\n## Four Signals\n~/dev/repos/fs-website/docs\n~/dev/fs-news-digest/docs\n",
        )
        .unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].name, "fs-website-docs");
        assert_eq!(links[0].destination, "~/dev/repos");
        assert_eq!(links[0].target, "~/dev/repos/fs-website/docs");
        assert_eq!(links[1].name, "fs-news-digest-docs");
    }

    #[test]
    fn switches_destination_on_new_heading() {
        let links = parse_str("# ~/dev/a\n~/dev/x/docs\n# ~/dev/b\n~/dev/y/docs\n").unwrap();
        assert_eq!(links[0].destination, "~/dev/a");
        assert_eq!(links[1].destination, "~/dev/b");
    }

    #[test]
    fn missing_file_is_empty() {
        assert!(parse(Path::new("/no/such/symlinks.md")).unwrap().is_empty());
    }

    #[test]
    fn errors_on_target_before_destination() {
        assert!(parse_str("~/dev/x/docs\n").is_err());
    }

    #[test]
    fn errors_on_non_tilde_destination() {
        assert!(parse_str("# Heading\n~/dev/x/docs\n").is_err());
    }

    #[test]
    fn errors_on_unexpected_line() {
        assert!(parse_str("# ~/dev/a\nsome prose line\n").is_err());
    }

    #[test]
    fn errors_on_duplicate_link_name() {
        assert!(parse_str("# ~/dev/a\n~/dev/x/docs\n~/dev/x/docs\n").is_err());
    }
}
