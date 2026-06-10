use crate::catalog;

use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;
use owo_colors::OwoColorize;

pub mod config;

pub struct Symlinks {
    manifest: Manifest,
}

impl Symlinks {
    pub fn new() -> Symlinks {
        Symlinks {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("symlinks manifest"),
        }
    }
    fn entries(ctx: &PluginContext) -> Vec<catalog::Symlink> {
        config::parse(&ctx.config_file).unwrap_or_default()
    }
}

impl Default for Symlinks {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Symlinks {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        Ok(Self::entries(ctx)
            .into_iter()
            .map(|s| {
                let link = catalog::expand_tilde(&s.destination).join(&s.name);
                let exists = link.exists() || link.symlink_metadata().is_ok();
                PlanStep {
                    description: format!(
                        "symlink {} → {} ({})",
                        s.name,
                        s.target,
                        if exists { "linked" } else { "to create" }
                    ),
                    mutates: !exists,
                }
            })
            .collect())
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        let entries = Self::entries(ctx);
        for s in &entries {
            let target = catalog::expand_tilde(&s.target);
            let dest_dir = catalog::expand_tilde(&s.destination);
            let link = dest_dir.join(&s.name);
            if target.exists() || target.symlink_metadata().is_ok() {
                println!(
                    "{}",
                    format!("  linking {} → {}", link.display(), target.display()).cyan()
                );
                std::fs::create_dir_all(&dest_dir)?;
                let _ = std::fs::remove_file(&link);
                std::os::unix::fs::symlink(&target, &link)?;
            } else {
                println!(
                    "{}",
                    format!(
                        "  skipping {} (target {} does not exist)",
                        s.name,
                        target.display()
                    )
                    .yellow()
                );
            }
        }
        Ok(())
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let entries = Self::entries(ctx);
        let mut linked = 0;
        let mut missing = 0;
        for s in &entries {
            let link = catalog::expand_tilde(&s.destination).join(&s.name);
            if link.exists() || link.symlink_metadata().is_ok() {
                linked += 1;
            } else {
                missing += 1;
            }
        }
        let status = if missing > 0 {
            HealthStatus::Warn
        } else if entries.is_empty() {
            HealthStatus::Missing
        } else {
            HealthStatus::Ok
        };
        Ok(Health {
            status,
            details: vec![format!("{linked} linked, {missing} missing")],
        })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(Self::entries(ctx)
            .into_iter()
            .map(|s| format!("{} → {}", s.name, s.target))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::scope::ScopeKind;
    #[test]
    fn manifest_loads_and_is_user_scoped() {
        let p = Symlinks::new();
        assert_eq!(p.manifest().name, "symlinks");
        assert_eq!(p.manifest().scopes, vec![ScopeKind::User]);
    }
}
