use crate::commands;
use crate::config::{catalog_root, Config};
use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;

pub struct Apm {
    manifest: Manifest,
}

impl Apm {
    pub fn new() -> Apm {
        Apm {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("apm manifest"),
        }
    }
    fn stacks(cfg: &Config) -> Vec<String> {
        let dir = catalog_root(cfg).join("apm");
        if !dir.exists() {
            return vec![];
        }
        std::fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for Apm {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Apm {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        let stack = ctx.args.first().map(String::as_str).unwrap_or("core");
        Ok(vec![PlanStep {
            description: format!("install APM stack: {stack}"),
            mutates: true,
        }])
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        let stack = ctx.args.first().map(String::as_str).unwrap_or("core");
        let scope = match &ctx.scope {
            crate::cli::scope::Scope::User => "user",
            crate::cli::scope::Scope::Folder(_) => "folder",
        };
        let path = match &ctx.scope {
            crate::cli::scope::Scope::User => None,
            crate::cli::scope::Scope::Folder(p) => Some(p.to_string_lossy().to_string()),
        };
        commands::ai::run_apm(ctx.cfg, stack, scope, path.as_deref(), ctx.yes)
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let stacks = Self::stacks(ctx.cfg);
        if stacks.is_empty() {
            return Ok(Health {
                status: HealthStatus::Missing,
                details: vec!["No APM stacks found in catalog".into()],
            });
        }
        Ok(Health {
            status: HealthStatus::Ok,
            details: vec![format!("Available stacks: {}", stacks.join(", "))],
        })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(Self::stacks(ctx.cfg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::scope::ScopeKind;
    #[test]
    fn manifest_loads_and_has_both_scopes() {
        let p = Apm::new();
        assert_eq!(p.manifest().name, "apm");
        assert_eq!(
            p.manifest().scopes,
            vec![ScopeKind::User, ScopeKind::Folder]
        );
    }
}
