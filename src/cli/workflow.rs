use crate::cli::scope::ScopeKind;
use anyhow::{bail, Result};
use std::collections::HashMap;

/// One step of a workflow: a plugin or another workflow, plus inline args.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub args: Vec<String>,
}

/// A scope-aware workflow: a separate ordered step list per scope.
#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub user: Vec<Step>,
    pub folder: Vec<Step>,
}

impl Workflow {
    pub fn steps_for(&self, kind: ScopeKind) -> &[Step] {
        match kind {
            ScopeKind::User => &self.user,
            ScopeKind::Folder => &self.folder,
        }
    }
}

/// A resolved, flattened step ready to execute: a concrete plugin name + args.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCall {
    pub plugin: String,
    pub args: Vec<String>,
}

/// Recursively flatten a workflow into an ordered list of plugin calls for the
/// given scope. `workflows` maps name -> Workflow; any step name not found in it
/// is treated as a terminal plugin call. Detects cycles.
pub fn expand(
    name: &str,
    kind: ScopeKind,
    workflows: &HashMap<String, Workflow>,
) -> Result<Vec<PluginCall>> {
    let mut out = Vec::new();
    let mut stack = Vec::new();
    expand_into(name, &[], kind, workflows, &mut stack, &mut out)?;
    Ok(out)
}

fn expand_into(
    name: &str,
    args: &[String],
    kind: ScopeKind,
    workflows: &HashMap<String, Workflow>,
    stack: &mut Vec<String>,
    out: &mut Vec<PluginCall>,
) -> Result<()> {
    if stack.iter().any(|n| n == name) {
        stack.push(name.to_string());
        bail!("workflow cycle detected: {}", stack.join(" -> "));
    }
    match workflows.get(name) {
        None => {
            out.push(PluginCall {
                plugin: name.to_string(),
                args: args.to_vec(),
            });
        }
        Some(wf) => {
            stack.push(name.to_string());
            for step in wf.steps_for(kind) {
                expand_into(&step.name, &step.args, kind, workflows, stack, out)?;
            }
            stack.pop();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(name: &str, args: &[&str]) -> Step {
        Step {
            name: name.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn map(ws: Vec<Workflow>) -> HashMap<String, Workflow> {
        ws.into_iter().map(|w| (w.name.clone(), w)).collect()
    }

    #[test]
    fn expands_nested_workflow_in_order() {
        let dev = Workflow {
            name: "dev-setup".into(),
            user: vec![step("apm", &["core"]), step("claude", &[])],
            folder: vec![step("permissions", &[]), step("claude", &[])],
        };
        let setup = Workflow {
            name: "setup".into(),
            user: vec![step("brew", &[]), step("dev-setup", &[])],
            folder: vec![step("dev-setup", &[])],
        };
        let ws = map(vec![dev, setup]);

        let user = expand("setup", ScopeKind::User, &ws).unwrap();
        assert_eq!(
            user,
            vec![
                PluginCall {
                    plugin: "brew".into(),
                    args: vec![]
                },
                PluginCall {
                    plugin: "apm".into(),
                    args: vec!["core".into()]
                },
                PluginCall {
                    plugin: "claude".into(),
                    args: vec![]
                },
            ]
        );

        let folder = expand("setup", ScopeKind::Folder, &ws).unwrap();
        assert_eq!(
            folder,
            vec![
                PluginCall {
                    plugin: "permissions".into(),
                    args: vec![]
                },
                PluginCall {
                    plugin: "claude".into(),
                    args: vec![]
                },
            ]
        );
    }

    #[test]
    fn detects_cycles() {
        let a = Workflow {
            name: "a".into(),
            user: vec![step("b", &[])],
            folder: vec![],
        };
        let b = Workflow {
            name: "b".into(),
            user: vec![step("a", &[])],
            folder: vec![],
        };
        let ws = map(vec![a, b]);
        let err = expand("a", ScopeKind::User, &ws).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }
}
