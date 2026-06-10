use crate::cli::scope::ScopeKind;
use crate::commands;
use crate::config::catalog_root;
use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;

pub struct Permissions {
    manifest: Manifest,
}

impl Permissions {
    pub fn new() -> Permissions {
        Permissions {
            manifest: Manifest::from_toml(include_str!("plugin.toml"))
                .expect("permissions manifest"),
        }
    }
}

impl Default for Permissions {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Permissions {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        let global = ctx.scope.kind() == ScopeKind::User;
        let scope_label = if global { "global" } else { "project" };
        Ok(vec![PlanStep {
            description: format!("sync AI permissions ({scope_label} scope)"),
            mutates: true,
        }])
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        let global = ctx.scope.kind() == ScopeKind::User;
        commands::permissions::run(
            ctx.cfg,
            commands::permissions::PermissionsAction::Sync {
                target: "claude".into(),
                global,
                yes: true,
            },
        )
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let tpl = catalog_root(ctx.cfg)
            .join("permissions")
            .join("template.json");
        if !tpl.exists() {
            return Ok(Health {
                status: HealthStatus::Missing,
                details: vec![format!("template not found: {}", tpl.display())],
            });
        }
        Ok(Health {
            status: HealthStatus::Ok,
            details: vec!["permissions template present".into()],
        })
    }

    fn list(&self, _ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(crate::permissions::targets::all_targets()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manifest_loads_and_has_both_scopes() {
        let p = Permissions::new();
        assert_eq!(p.manifest().name, "permissions");
        assert_eq!(
            p.manifest().scopes,
            vec![ScopeKind::User, ScopeKind::Folder]
        );
    }
}
