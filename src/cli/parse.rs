use crate::cli::scope::Scope;
use crate::cli::verb::{resolve_verb, Verb};
use anyhow::{bail, Result};
use std::path::PathBuf;

/// A fully-resolved command: which name, which verb, scope, plugin args, dry-run.
#[derive(Debug, PartialEq, Eq)]
pub struct Invocation {
    pub name: String,
    pub verb: Verb,
    pub scope: Option<Scope>,
    pub args: Vec<String>,
    pub dry_run: bool,
}

/// Parse tokens of the form:
///   <name> [verb] [args…] [--scope user|folder] [--path PATH] [--dry-run] [-- raw…]
pub fn parse(tokens: &[String]) -> Result<Invocation> {
    let mut iter = tokens.iter().peekable();
    let name = match iter.next() {
        Some(n) => n.clone(),
        None => bail!("no command given"),
    };

    let mut explicit_verb: Option<Verb> = None;
    let mut scope: Option<Scope> = None;
    let mut dry_run = false;
    let mut args: Vec<String> = Vec::new();
    let mut passthrough = false;

    // Optional verb is only the *first* token if it is a known verb.
    if let Some(tok) = iter.peek() {
        if let Some(v) = Verb::from_token(tok) {
            explicit_verb = Some(v);
            iter.next();
        }
    }

    while let Some(tok) = iter.next() {
        if passthrough {
            args.push(tok.clone());
            continue;
        }
        match tok.as_str() {
            "--" => passthrough = true,
            "--scope" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--scope requires a value (user or folder)"))?
                    .as_str();
                scope = Some(match val {
                    "user" => Scope::User,
                    "folder" => Scope::Folder(PathBuf::new()),
                    other => bail!("invalid --scope '{}': expected 'user' or 'folder'", other),
                });
            }
            "--path" => {
                let val = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("--path requires a value"))?
                    .as_str();
                scope = match scope {
                    Some(Scope::Folder(_)) => Some(Scope::Folder(PathBuf::from(val))),
                    _ => bail!("--path can only be used with --scope folder"),
                };
            }
            "--dry-run" => dry_run = true,
            other => args.push(other.to_string()),
        }
    }

    let verb = resolve_verb(explicit_verb);

    if verb == Verb::Install && scope.is_none() {
        bail!("'{} install' requires --scope user|folder", name);
    }

    Ok(Invocation {
        name,
        verb,
        scope,
        args,
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn bare_name_is_info() {
        let inv = parse(&toks(&["symlinks"])).unwrap();
        assert_eq!(inv.verb, Verb::Info);
        assert_eq!(inv.scope, None);
    }

    #[test]
    fn install_requires_scope() {
        let err = parse(&toks(&["symlinks", "install"])).unwrap_err();
        assert!(err.to_string().contains("requires --scope"));
    }

    #[test]
    fn install_with_scope_user() {
        let inv = parse(&toks(&["brew", "install", "--scope", "user"])).unwrap();
        assert_eq!(inv.verb, Verb::Install);
        assert_eq!(inv.scope, Some(Scope::User));
    }

    #[test]
    fn install_with_scope_folder() {
        let inv = parse(&toks(&[
            "brew", "install", "--scope", "folder", "--path", "/tmp",
        ]))
        .unwrap();
        assert_eq!(inv.verb, Verb::Install);
        assert_eq!(inv.scope, Some(Scope::Folder(PathBuf::from("/tmp"))));
    }

    #[test]
    fn dry_run_flag() {
        let inv = parse(&toks(&["brew", "install", "--scope", "user", "--dry-run"])).unwrap();
        assert!(inv.dry_run);
    }

    #[test]
    fn explicit_info_verb() {
        let inv = parse(&toks(&["brew", "info"])).unwrap();
        assert_eq!(inv.verb, Verb::Info);
    }

    #[test]
    fn status_verb() {
        let inv = parse(&toks(&["brew", "status", "--scope", "user"])).unwrap();
        assert_eq!(inv.verb, Verb::Status);
        assert_eq!(inv.scope, Some(Scope::User));
    }

    #[test]
    fn passthrough_args() {
        let inv = parse(&toks(&[
            "apm", "install", "--scope", "user", "--", "--weird",
        ]))
        .unwrap();
        assert_eq!(inv.verb, Verb::Install);
        assert_eq!(inv.args, vec!["--weird".to_string()]);
    }

    #[test]
    fn empty_input_errors() {
        let err = parse(&[]).unwrap_err();
        assert!(err.to_string().contains("no command given"));
    }

    #[test]
    fn invalid_scope_value() {
        let err = parse(&toks(&["brew", "install", "--scope", "invalid"])).unwrap_err();
        assert!(err.to_string().contains("invalid --scope"));
    }

    #[test]
    fn path_without_folder_scope_errors() {
        let err = parse(&toks(&[
            "brew", "install", "--scope", "user", "--path", "/tmp",
        ]))
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("--path can only be used with --scope folder"));
    }
}
