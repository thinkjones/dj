use crate::cli::scope::Scope;
use crate::commands;
use crate::config::catalog_root;
use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;
use dirs::home_dir;

pub struct Claude {
    manifest: Manifest,
}

impl Claude {
    pub fn new() -> Claude {
        Claude {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("claude manifest"),
        }
    }
}

impl Default for Claude {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Claude {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        match &ctx.scope {
            Scope::User => Ok(vec![PlanStep {
                description: format!(
                    "write Claude user settings ({})",
                    ctx.cfg.default_agent_stack
                ),
                mutates: true,
            }]),
            Scope::Folder(_) => Ok(vec![PlanStep {
                description: "write Claude project settings".into(),
                mutates: true,
            }]),
        }
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        match &ctx.scope {
            Scope::User => commands::ai::run(ctx.cfg, "claude", "user", None, ctx.yes),
            Scope::Folder(path) => {
                let p = if path.as_os_str().is_empty() {
                    None
                } else {
                    path.to_str()
                };
                commands::ai::run_setup_project(ctx.cfg, p, ctx.yes)
            }
        }
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        match &ctx.scope {
            Scope::User => {
                let settings = claude_desktop_settings_path();
                if settings.exists() {
                    Ok(Health {
                        status: HealthStatus::Ok,
                        details: vec![format!("settings found: {}", settings.display())],
                    })
                } else {
                    Ok(Health {
                        status: HealthStatus::Missing,
                        details: vec!["Claude user settings not found".into()],
                    })
                }
            }
            Scope::Folder(_) => {
                let project_md = catalog_root(ctx.cfg).join("claude").join("project.md");
                if project_md.exists() {
                    Ok(Health {
                        status: HealthStatus::Ok,
                        details: vec!["project.md present".into()],
                    })
                } else {
                    Ok(Health {
                        status: HealthStatus::Missing,
                        details: vec!["project.md not found".into()],
                    })
                }
            }
        }
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(vec![ctx.cfg.default_agent_stack.clone()])
    }
}

#[cfg(target_os = "macos")]
fn claude_desktop_settings_path() -> std::path::PathBuf {
    home_dir()
        .unwrap_or_default()
        .join("Library/Application Support/Claude/settings.json")
}

#[cfg(not(target_os = "macos"))]
fn claude_desktop_settings_path() -> std::path::PathBuf {
    home_dir()
        .unwrap_or_default()
        .join(".config/Claude/settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::scope::ScopeKind;
    #[test]
    fn manifest_loads_and_has_both_scopes() {
        let p = Claude::new();
        assert_eq!(p.manifest().name, "claude");
        assert_eq!(
            p.manifest().scopes,
            vec![ScopeKind::User, ScopeKind::Folder]
        );
    }
}
