use crate::cli::scope::ScopeKind;
use crate::plugins::{Health, HealthStatus, Manifest, Plugin, PluginContext, PlanStep};
use crate::catalog;
use anyhow::Result;
use owo_colors::OwoColorize;

pub mod config;

pub struct Custom {
    manifest: Manifest,
}

impl Custom {
    pub fn new() -> Custom {
        Custom { manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("custom manifest") }
    }
    fn entries(ctx: &PluginContext) -> Vec<catalog::CustomInstall> {
        config::parse(&ctx.config_file).unwrap_or_default()
    }
}

impl Default for Custom {
    fn default() -> Self { Self::new() }
}

impl Plugin for Custom {
    fn manifest(&self) -> &Manifest { &self.manifest }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        Ok(Self::entries(ctx).into_iter().map(|e| PlanStep {
            description: format!("custom install: {}", e.name),
            mutates: true,
        }).collect())
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
                println!("{}", format!("  {} already on PATH — skipping", c.name).dimmed());
            } else {
                println!("{}", format!("  Installing {}...", c.name).cyan());
                std::process::Command::new("sh").args(["-c", &c.install_script]).status()?;
            }
        }
        Ok(())
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let entries = Self::entries(ctx);
        let mut details = Vec::new();
        for e in &entries {
            let on_path = std::process::Command::new("which").arg(&e.name).output()
                .map(|o| o.status.success()).unwrap_or(false);
            details.push(format!("{}: {}", e.name, if on_path { "installed" } else { "missing" }));
        }
        let status = if entries.is_empty() { HealthStatus::Missing } else { HealthStatus::Ok };
        Ok(Health { status, details })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(Self::entries(ctx).into_iter().map(|e| e.name).collect())
    }

    fn example_config(&self, _: ScopeKind) -> Option<String> {
        Some(include_str!("example.md").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_loads_and_is_user_scoped() {
        let p = Custom::new();
        assert_eq!(p.manifest().name, "custom");
        assert_eq!(p.manifest().scopes, vec![ScopeKind::User]);
    }
}
