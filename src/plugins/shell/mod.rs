use crate::catalog;

use crate::plugins::{Health, HealthStatus, Manifest, PlanStep, Plugin, PluginContext};
use anyhow::Result;
use dirs::home_dir;

pub mod config;

pub struct Shell {
    manifest: Manifest,
}

impl Shell {
    pub fn new() -> Shell {
        Shell {
            manifest: Manifest::from_toml(include_str!("plugin.toml")).expect("shell manifest"),
        }
    }
    fn entries(ctx: &PluginContext) -> Vec<catalog::ShellFunction> {
        config::parse(&ctx.config_file).unwrap_or_default()
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for Shell {
    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>> {
        Ok(Self::entries(ctx)
            .into_iter()
            .map(|f| PlanStep {
                description: format!("shell function: {}", f.name),
                mutates: true,
            })
            .collect())
    }

    fn run(&self, ctx: &PluginContext) -> Result<()> {
        let entries = Self::entries(ctx);
        let home = dirs::home_dir().unwrap_or_default();
        let zshrc_path = home.join(".zshrc");

        let mut content = std::fs::read_to_string(&zshrc_path).unwrap_or_default();

        // Shell functions
        let fn_begin = "# === dj:functions begin ===\n";
        let fn_end = "# === dj:functions end ===\n";
        let mut fn_block = String::from(fn_begin);
        for f in &entries {
            fn_block.push_str(&f.body);
            fn_block.push('\n');
        }
        fn_block.push_str(fn_end);
        content = rewrite_sentinel(&content, fn_begin, fn_end, &fn_block);

        std::fs::write(&zshrc_path, &content)?;
        println!("  Updated {}", zshrc_path.display());
        Ok(())
    }

    fn doctor(&self, ctx: &PluginContext) -> Result<Health> {
        let n = Self::entries(ctx).len();
        let zshrc = home_dir().unwrap_or_default().join(".zshrc");
        if !zshrc.exists() {
            return Ok(Health {
                status: HealthStatus::Missing,
                details: vec!["~/.zshrc not found".into()],
            });
        }
        Ok(Health {
            status: HealthStatus::Ok,
            details: vec![format!("{n} shell functions in catalog; ~/.zshrc present")],
        })
    }

    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>> {
        Ok(Self::entries(ctx).into_iter().map(|f| f.name).collect())
    }
}

fn rewrite_sentinel(content: &str, begin: &str, end: &str, block: &str) -> String {
    if content.contains(begin) {
        let before = content.split(begin).next().unwrap_or("");
        let after = content.split(end).nth(1).unwrap_or("");
        format!("{}{}{}", before, block, after)
    } else {
        format!("{}\n{}", content.trim_end(), block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::scope::ScopeKind;
    #[test]
    fn manifest_loads_and_is_user_scoped() {
        let p = Shell::new();
        assert_eq!(p.manifest().name, "shell");
        assert_eq!(p.manifest().scopes, vec![ScopeKind::User]);
    }
}
