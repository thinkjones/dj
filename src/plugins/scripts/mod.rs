use crate::catalog;

use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;
use owo_colors::OwoColorize;

pub mod config;

pub struct Scripts {
    manifest: Manifest,
}

impl Scripts {
    pub fn new() -> Scripts {
        Scripts {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("scripts manifest"),
        }
    }
    fn entries(ctx: &PluginContext) -> Vec<catalog::ScriptInstall> {
        config::parse(&ctx.config_file).unwrap_or_default()
    }
}

impl Default for Scripts {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Scripts {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        Ok(Self::entries(ctx)
            .into_iter()
            .map(|e| PlanStep {
                description: format!("script: {}", e.name),
                mutates: true,
            })
            .collect())
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        let entries = Self::entries(ctx);
        for c in &entries {
            let on_path = std::process::Command::new("which")
                .arg(&c.name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if on_path {
                println!(
                    "{}",
                    format!("  {} already on PATH — skipping", c.name).dimmed()
                );
            } else {
                println!("{}", format!("  Running script {}...", c.name).cyan());
                std::process::Command::new("sh")
                    .args(["-c", &c.install_script])
                    .status()?;
            }
        }
        Ok(())
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let entries = Self::entries(ctx);
        let mut details = Vec::new();
        for e in &entries {
            let on_path = std::process::Command::new("which")
                .arg(&e.name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            details.push(format!(
                "{}: {}",
                e.name,
                if on_path { "installed" } else { "missing" }
            ));
        }
        let status = if entries.is_empty() {
            HealthStatus::Missing
        } else {
            HealthStatus::Ok
        };
        Ok(Health { status, details })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(Self::entries(ctx).into_iter().map(|e| e.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::scope::ScopeKind;
    #[test]
    fn manifest_loads_and_is_user_scoped() {
        let p = Scripts::new();
        assert_eq!(p.manifest().name, "scripts");
        assert_eq!(p.manifest().scopes, vec![ScopeKind::User]);
    }
}
