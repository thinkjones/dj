use crate::catalog;
use crate::cli::scope::ScopeKind;
use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;
use owo_colors::OwoColorize;

pub mod config;

pub struct Runtimes {
    manifest: Manifest,
}

impl Runtimes {
    pub fn new() -> Runtimes {
        Runtimes {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("runtimes manifest"),
        }
    }
    fn entries(ctx: &PluginContext) -> (Vec<catalog::Runtime>, Vec<catalog::PackageManager>) {
        let runtimes = config::parse_runtimes(&ctx.config_file).unwrap_or_default();
        let pms = config::parse_package_managers(&ctx.config_file).unwrap_or_default();
        (runtimes, pms)
    }
}

impl Default for Runtimes {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Runtimes {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        let (runtimes, pms) = Self::entries(ctx);
        let mut steps: Vec<PlanStep> = runtimes
            .into_iter()
            .map(|r| PlanStep {
                description: format!("runtime {}@{}", r.name, r.version),
                mutates: true,
            })
            .collect();
        steps.extend(pms.into_iter().map(|pm| PlanStep {
            description: format!("package manager: {}", pm.name),
            mutates: true,
        }));
        Ok(steps)
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        let (runtimes, pms) = Self::entries(ctx);
        for r in &runtimes {
            let spec = format!("{}@{}", r.name, r.version);
            println!("{}", format!("  mise use -g {}", spec).cyan());
            std::process::Command::new("mise")
                .args(["use", "-g", &spec])
                .status()?;
        }
        for pm in &pms {
            let on_path = std::process::Command::new("which")
                .arg(&pm.name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !on_path {
                println!("{}", format!("  Installing {}...", pm.name).cyan());
                std::process::Command::new("sh")
                    .args(["-c", &pm.install_script])
                    .status()?;
            }
        }
        Ok(())
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let (runtimes, pms) = Self::entries(ctx);
        let on_path = std::process::Command::new("which")
            .arg("mise")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !on_path {
            return Ok(Health {
                status: HealthStatus::Missing,
                details: vec!["mise not installed".into()],
            });
        }
        Ok(Health {
            status: HealthStatus::Ok,
            details: vec![format!(
                "{} runtimes, {} package managers in catalog",
                runtimes.len(),
                pms.len()
            )],
        })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        let (runtimes, pms) = Self::entries(ctx);
        let mut out: Vec<String> = runtimes
            .into_iter()
            .map(|r| format!("{}@{}", r.name, r.version))
            .collect();
        out.extend(pms.into_iter().map(|pm| pm.name));
        Ok(out)
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
        let p = Runtimes::new();
        assert_eq!(p.manifest().name, "runtimes");
        assert_eq!(p.manifest().scopes, vec![ScopeKind::User]);
    }
}
