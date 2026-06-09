use crate::cli::scope::{Scope, ScopeKind};
use crate::cli::workflow::Workflow;
use crate::cli::Resolved;
use crate::config::{catalog_root, Config};
use crate::plugins::{lastrun, registry, workflows, Health, Plugin, PluginContext};
use anyhow::{bail, Result};
use std::collections::HashMap;

pub struct Runtime {
    plugins: Vec<Box<dyn Plugin>>,
    workflows: HashMap<String, Workflow>,
}

impl Runtime {
    pub fn load(cfg: &Config) -> Result<Runtime> {
        let plugins = registry::built_ins();
        let root = catalog_root(cfg);
        let workflows = workflows::parse(&root.join("workflows.md"))?;
        let rt = Runtime { plugins, workflows };
        rt.validate()?;
        Ok(rt)
    }

    fn validate(&self) -> Result<()> {
        // every workflow step resolves to a known plugin or workflow; no cycles.
        for name in self.workflows.keys() {
            for kind in [ScopeKind::User, ScopeKind::Folder] {
                crate::cli::workflow::expand(name, kind, &self.workflows)?; // surfaces cycles
            }
        }
        Ok(())
    }

    pub fn plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.iter().map(|p| p.as_ref()).find(|p| p.manifest().name == name)
    }

    pub fn plugin_iter(&self) -> impl Iterator<Item = &dyn Plugin> {
        self.plugins.iter().map(|p| p.as_ref())
    }

    pub fn workflows(&self) -> &HashMap<String, Workflow> {
        &self.workflows
    }

    pub fn resolve(&self, name: &str) -> Resolved {
        let is_plugin = self.plugins.iter().any(|p| p.manifest().name == name);
        let is_workflow = self.workflows.contains_key(name);
        match (is_plugin, is_workflow) {
            (true, true) => Resolved::Both,
            (true, false) => Resolved::Plugin,
            (false, true) => Resolved::Workflow,
            (false, false) => Resolved::None,
        }
    }

    pub fn manifests(&self) -> impl Iterator<Item = &crate::plugins::Manifest> {
        self.plugins.iter().map(|p| p.manifest())
    }

    /// Build the per-call context: validate scope support, resolve config paths.
    fn context<'a>(&self, name: &str, scope: &Scope, args: &[String], cfg: &'a Config, yes: bool)
        -> Result<(PluginContext<'a>, &dyn Plugin)>
    {
        let p = self.plugin(name).ok_or_else(|| anyhow::anyhow!("no such plugin: {name}"))?;
        let m = p.manifest();
        if !m.scopes.contains(&scope.kind()) {
            bail!("{} does not support that scope", name);
        }
        let config_dir = catalog_root(cfg).join(name);
        let file = m.config.get(&scope.kind()).cloned().unwrap_or_default();
        let config_file = config_dir.join(file);
        let ctx = PluginContext {
            scope: scope.clone(), args: args.to_vec(), config_dir, config_file, cfg, yes,
        };
        Ok((ctx, p))
    }

    pub fn run(&self, name: &str, scope: &Scope, args: &[String], dry_run: bool, yes: bool, cfg: &Config) -> Result<()> {
        let (ctx, p) = self.context(name, scope, args, cfg, yes)?;
        if dry_run {
            for step in p.plan(&ctx)? {
                let mark = if step.mutates { "~" } else { " " };
                println!("  [{mark}] {}", step.description);
            }
            return Ok(());
        }
        p.run(&ctx)?;
        let scope_str = match scope.kind() { ScopeKind::User => "user", ScopeKind::Folder => "folder" };
        let _ = lastrun::record(&lastrun::key(name, scope_str));
        Ok(())
    }

    pub fn doctor_one(&self, name: &str, scope: &Scope, cfg: &Config) -> Result<Health> {
        let (ctx, p) = self.context(name, scope, &[], cfg, true)?;
        p.doctor(&ctx)
    }

    pub fn list_one(&self, name: &str, scope: &Scope, cfg: &Config) -> Result<Vec<String>> {
        let (ctx, p) = self.context(name, scope, &[], cfg, true)?;
        p.list(&ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::{Cadence, Health, HealthStatus, Manifest, PlanStep};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Fake;

    static RUN_CALLED: AtomicBool = AtomicBool::new(false);

    impl Plugin for Fake {
        fn manifest(&self) -> &Manifest {
            // leaked once for the test; fine in a unit test
            static M: std::sync::OnceLock<Manifest> = std::sync::OnceLock::new();
            M.get_or_init(|| Manifest {
                name: "fake".into(), summary: "t".into(), version: "0.1.0".into(),
                scopes: vec![ScopeKind::User], cadence: Cadence::Regular,
                config: BTreeMap::from([(ScopeKind::User, "config.md".into())]),
            })
        }
        fn plan(&self, _: &PluginContext) -> Result<Vec<PlanStep>> {
            Ok(vec![PlanStep { description: "would do X".into(), mutates: true }])
        }
        fn run(&self, _: &PluginContext) -> Result<()> {
            RUN_CALLED.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn doctor(&self, _: &PluginContext) -> Result<Health> {
            Ok(Health { status: HealthStatus::Ok, details: vec![] })
        }
        fn list(&self, _: &PluginContext) -> Result<Vec<String>> { Ok(vec!["entry".into()]) }
        fn example_config(&self, _: ScopeKind) -> Option<String> { None }
    }

    fn rt() -> Runtime {
        RUN_CALLED.store(false, Ordering::SeqCst);
        Runtime { plugins: vec![Box::new(Fake)], workflows: HashMap::new() }
    }

    #[test]
    fn rejects_unsupported_scope() {
        let cfg = Config { catalog_root: "/tmp".into(), default_agent_stack: "core".into(), catalog: None };
        let err = rt().run("fake", &Scope::Folder(std::path::PathBuf::new()), &[], false, true, &cfg).unwrap_err();
        assert!(err.to_string().contains("does not support that scope"));
    }

    #[test]
    fn dry_run_prints_plan_without_running() {
        let cfg = Config { catalog_root: "/tmp".into(), default_agent_stack: "core".into(), catalog: None };
        rt().run("fake", &Scope::User, &[], true, true, &cfg).unwrap();
        assert!(!RUN_CALLED.load(Ordering::SeqCst), "run() should not be called during dry-run");
    }
}
