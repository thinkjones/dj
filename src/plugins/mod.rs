pub mod apm;
pub mod brew;
pub mod claude;
pub mod custom;
pub mod dotfiles;
pub mod lastrun;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod runtimes;
pub mod shell;
pub mod symlinks;
pub mod workflows;

use crate::cli::scope::{Scope, ScopeKind};
use crate::config::Config;
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    OneTime,
    Regular,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub summary: String,
    pub version: String,
    pub scopes: Vec<ScopeKind>,
    #[allow(dead_code)]
    pub cadence: Cadence,
    /// scope -> config filename under catalog/<name>/
    pub config: BTreeMap<ScopeKind, String>,
}

pub struct PluginContext<'a> {
    pub scope: Scope,
    pub args: Vec<String>,
    pub config_file: PathBuf,
    pub cfg: &'a Config,
    pub yes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    pub description: String,
    pub mutates: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Ok,
    Warn,
    Missing,
}

#[derive(Debug, Clone)]
pub struct Health {
    pub status: HealthStatus,
    pub details: Vec<String>,
}

/// The one interface every built-in plugin meets the runtime through.
pub trait Plugin {
    fn manifest(&self) -> &Manifest;
    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>>;
    fn run(&self, ctx: &PluginContext) -> Result<()>;
    fn doctor(&self, ctx: &PluginContext) -> Result<Health>;
    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>>;
}

#[derive(Deserialize)]
struct RawManifest {
    name: String,
    summary: String,
    version: String,
    scopes: Vec<String>,
    cadence: String,
    #[serde(default)]
    config: BTreeMap<String, String>,
}

impl Manifest {
    pub fn from_toml(s: &str) -> Result<Manifest> {
        use anyhow::bail;
        let raw: RawManifest = toml::from_str(s)?;
        let scope_of = |t: &str| match t {
            "user" => Ok(ScopeKind::User),
            "folder" => Ok(ScopeKind::Folder),
            other => bail!("unknown scope '{other}' in plugin '{}'", raw.name),
        };
        let scopes = raw
            .scopes
            .iter()
            .map(|s| scope_of(s))
            .collect::<Result<Vec<_>>>()?;
        let cadence = match raw.cadence.as_str() {
            "one-time" => Cadence::OneTime,
            "regular" => Cadence::Regular,
            other => bail!("unknown cadence '{other}' in plugin '{}'", raw.name),
        };
        let mut config = BTreeMap::new();
        for (k, v) in raw.config {
            let kind = scope_of(&k)?;
            if !scopes.contains(&kind) {
                bail!(
                    "plugin '{}' declares config for unsupported scope '{k}'",
                    raw.name
                );
            }
            config.insert(kind, v);
        }
        Ok(Manifest {
            name: raw.name,
            summary: raw.summary,
            version: raw.version,
            scopes,
            cadence,
            config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A no-op plugin proves the trait is object-safe and the types compose.
    struct Dummy(Manifest);
    impl Plugin for Dummy {
        fn manifest(&self) -> &Manifest {
            &self.0
        }
        fn plan(&self, _: &PluginContext) -> Result<Vec<PlanStep>> {
            Ok(vec![])
        }
        fn run(&self, _: &PluginContext) -> Result<()> {
            Ok(())
        }
        fn doctor(&self, _: &PluginContext) -> Result<Health> {
            Ok(Health {
                status: HealthStatus::Ok,
                details: vec![],
            })
        }
        fn list(&self, _: &PluginContext) -> Result<Vec<String>> {
            Ok(vec![])
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let m = Manifest {
            name: "x".into(),
            summary: "t".into(),
            version: "0.1.0".into(),
            scopes: vec![ScopeKind::User],
            cadence: Cadence::Regular,
            config: BTreeMap::new(),
        };
        let p: Box<dyn Plugin> = Box::new(Dummy(m));
        assert_eq!(p.manifest().name, "x");
    }

    #[test]
    fn parses_multi_scope_manifest() {
        let m = Manifest::from_toml(
            r#"
name = "claude"
summary = "Claude config"
version = "0.1.0"
scopes = ["user", "folder"]
cadence = "regular"
[config]
user = "user.md"
folder = "project.md"
"#,
        )
        .unwrap();
        assert_eq!(m.name, "claude");
        assert_eq!(m.scopes, vec![ScopeKind::User, ScopeKind::Folder]);
        assert_eq!(m.cadence, Cadence::Regular);
        assert_eq!(m.config.get(&ScopeKind::Folder).unwrap(), "project.md");
    }

    #[test]
    fn rejects_config_for_undeclared_scope() {
        let err = Manifest::from_toml(
            r#"
name = "brew"
summary = "x"
version = "0.1.0"
scopes = ["user"]
cadence = "regular"
[config]
folder = "nope.md"
"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unsupported scope"));
    }

    #[test]
    fn rejects_unknown_cadence() {
        assert!(Manifest::from_toml(
            "name='x'\nsummary='x'\nversion='0'\nscopes=['user']\ncadence='weekly'\n"
        )
        .is_err());
    }
}
