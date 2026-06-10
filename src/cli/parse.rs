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
///   <name> [verb] [args…] [--user | --folder [path]] [--dry-run] [-- raw…]
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
            "--user" => scope = Some(Scope::User),
            "--folder" => {
                let path = match iter.peek() {
                    Some(p) if !p.starts_with("--") => {
                        let p = (*p).clone();
                        iter.next();
                        PathBuf::from(p)
                    }
                    _ => PathBuf::new(),
                };
                scope = Some(Scope::Folder(path));
            }
            "--dry-run" => dry_run = true,
            other => args.push(other.to_string()),
        }
    }

    let verb = resolve_verb(explicit_verb, scope.is_some());

    if verb.mutates() && scope.is_none() {
        bail!("'{} run' requires a scope: add --user or --folder", name);
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
    fn scope_flag_implies_run() {
        let inv = parse(&toks(&["symlinks", "--user"])).unwrap();
        assert_eq!(inv.verb, Verb::Run);
        assert_eq!(inv.scope, Some(Scope::User));
    }

    #[test]
    fn inline_arg_before_scope() {
        let inv = parse(&toks(&["apm", "core", "--user"])).unwrap();
        assert_eq!(inv.verb, Verb::Run);
        assert_eq!(inv.args, vec!["core".to_string()]);
    }

    #[test]
    fn explicit_run_without_scope_errors() {
        let err = parse(&toks(&["symlinks", "run"])).unwrap_err();
        assert!(err.to_string().contains("requires a scope"));
    }

    #[test]
    fn folder_with_path() {
        let inv = parse(&toks(&["claude", "--folder", "/tmp/x"])).unwrap();
        assert_eq!(inv.scope, Some(Scope::Folder(PathBuf::from("/tmp/x"))));
    }

    #[test]
    fn dry_run_and_passthrough() {
        let inv = parse(&toks(&["dotfiles", "--user", "--dry-run", "--", "--weird"])).unwrap();
        assert!(inv.dry_run);
        assert_eq!(inv.args, vec!["--weird".to_string()]);
    }

    #[test]
    fn empty_input_errors() {
        let err = parse(&[]).unwrap_err();
        assert!(err.to_string().contains("no command given"));
    }

    #[test]
    fn multiple_scope_flags_last_wins() {
        let inv = parse(&toks(&["cmd", "--user", "--folder", "/tmp"])).unwrap();
        assert!(matches!(inv.scope, Some(Scope::Folder(_))));
    }

    #[test]
    fn folder_without_path_then_dry_run() {
        let inv = parse(&toks(&["cmd", "--folder", "--dry-run"])).unwrap();
        assert!(matches!(inv.scope, Some(Scope::Folder(_))));
        assert!(inv.dry_run);
    }
}
