
use crate::commands;
use crate::config::{catalog_root, Config};
use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;
use std::path::PathBuf;

pub struct Dotfiles {
    manifest: Manifest,
}

impl Dotfiles {
    pub fn new() -> Dotfiles {
        Dotfiles {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("dotfiles manifest"),
        }
    }
    fn source_dir(cfg: &Config) -> PathBuf {
        catalog_root(cfg).join("chezmoi")
    }
}

impl Default for Dotfiles {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Dotfiles {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        let dir = Self::source_dir(ctx.cfg);
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut steps = vec![PlanStep {
            description: format!("apply dotfiles from {}", dir.display()),
            mutates: true,
        }];
        // List individual files as additional informational steps
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                steps.push(PlanStep {
                    description: format!("  dotfile: {name}"),
                    mutates: false,
                });
            }
        }
        Ok(steps)
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        commands::dotfiles::run(ctx.cfg, false, ctx.yes)
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let dir = Self::source_dir(ctx.cfg);
        if !dir.exists() {
            return Ok(Health {
                status: HealthStatus::Missing,
                details: vec![format!("chezmoi source not found: {}", dir.display())],
            });
        }
        Ok(Health {
            status: HealthStatus::Ok,
            details: vec![format!("chezmoi source: {}", dir.display())],
        })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        let dir = Self::source_dir(ctx.cfg);
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut out = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') {
                    out.push(name);
                }
            }
        }
        Ok(out)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::scope::ScopeKind;
    #[test]
    fn manifest_loads_and_is_user_scoped() {
        let p = Dotfiles::new();
        assert_eq!(p.manifest().name, "dotfiles");
        assert_eq!(p.manifest().scopes, vec![ScopeKind::User]);
    }
}
