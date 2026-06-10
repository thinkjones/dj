use crate::cli::parse::Invocation;
use crate::cli::scope::{Scope, ScopeKind};
use crate::cli::verb::Verb;
use crate::cli::workflow::expand;
use crate::cli::Resolved;
use crate::config::Config;
use crate::plugins::runtime::Runtime;
use anyhow::{bail, Result};
use owo_colors::OwoColorize;

pub fn dispatch(cfg: &Config, rt: &Runtime, inv: &Invocation, yes: bool) -> Result<()> {
    if inv.name != "self" && inv.name != "catalog" {
        crate::cli::onboarding::ensure_catalog(cfg)?;
    }

    match rt.resolve(&inv.name) {
        Resolved::None => bail!("unknown plugin or workflow: '{}'. Try `dj list`.", inv.name),
        Resolved::Both => {
            if inv.verb.mutates() {
                bail!(
                    "'{}' is both a plugin and a workflow. Disambiguate: \
                     `dj plugin:{} run …` or `dj workflow:{} run …`.",
                    inv.name,
                    inv.name,
                    inv.name
                );
            }
            println!(
                "{}",
                format!(
                    "⚠ '{}' is both a plugin and a workflow — showing both.",
                    inv.name
                )
                .yellow()
            );
            run_plugin_verb(cfg, rt, inv, yes)?;
            run_workflow_verb(cfg, rt, inv, yes)
        }
        Resolved::Plugin => run_plugin_verb(cfg, rt, inv, yes),
        Resolved::Workflow => run_workflow_verb(cfg, rt, inv, yes),
    }
}

fn run_plugin_verb(cfg: &Config, rt: &Runtime, inv: &Invocation, yes: bool) -> Result<()> {
    let p = rt.plugin(&inv.name).expect("resolved as plugin");
    match inv.verb {
        Verb::Info => {
            let m = p.manifest();
            println!("{} — {}", m.name.bold(), m.summary);
            let scopes: Vec<_> = m.scopes.iter().map(|k| scope_label(*k)).collect();
            println!("  scopes: {}", scopes.join(", "));
            println!("  version: {}", m.version);
            println!("  cadence: {:?}", m.cadence);
            Ok(())
        }
        Verb::Version => {
            println!("{} {}", p.manifest().name, p.manifest().version);
            Ok(())
        }
        Verb::Doctor => {
            let scope = inv.scope.clone().unwrap_or(Scope::User);
            let health = rt.doctor_one(&inv.name, &scope, cfg)?;
            println!("{} — {:?}", inv.name, health.status);
            for d in &health.details {
                println!("  {}", d);
            }
            Ok(())
        }
        Verb::List => {
            let scope = inv.scope.clone().unwrap_or(Scope::User);
            let items = rt.list_one(&inv.name, &scope, cfg)?;
            for item in items {
                println!("{}", item);
            }
            Ok(())
        }
        Verb::Run => {
            let scope = inv.scope.clone().expect("run has scope (parser-enforced)");
            rt.run(&inv.name, &scope, &inv.args, inv.dry_run, yes, cfg)
        }
    }
}

fn run_workflow_verb(cfg: &Config, rt: &Runtime, inv: &Invocation, yes: bool) -> Result<()> {
    match inv.verb {
        Verb::Info => {
            println!("{} — workflow", inv.name.bold());
            Ok(())
        }
        Verb::Version => {
            println!("{} {}", inv.name, env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Verb::List | Verb::Doctor => {
            // Workflow list/doctor delegates to global fan-out for now
            bail!("Workflow list/doctor not implemented; use `dj <plugin> list|doctor`")
        }
        Verb::Run => {
            let scope = inv.scope.clone().expect("run has scope");
            let calls = expand(&inv.name, scope.kind(), rt.workflows())?;
            if inv.dry_run {
                println!(
                    "{}",
                    format!(
                        "dry-run: {} ({} scope) would run:",
                        inv.name,
                        scope_label(scope.kind())
                    )
                    .cyan()
                );
                for c in &calls {
                    println!(
                        "  dj {} {} {}",
                        c.plugin,
                        c.args.join(" "),
                        scope_flag(&scope)
                    );
                }
                return Ok(());
            }
            for c in calls {
                let plugin = rt.plugin(&c.plugin).ok_or_else(|| {
                    anyhow::anyhow!("workflow step '{}' is not a plugin", c.plugin)
                })?;
                if !plugin.manifest().scopes.contains(&scope.kind()) {
                    bail!(
                        "workflow '{}' step '{}' does not support {} scope",
                        inv.name,
                        c.plugin,
                        scope_label(scope.kind())
                    );
                }
                println!("{}", format!("→ {} {}", c.plugin, c.args.join(" ")).cyan());
                rt.run(&c.plugin, &scope, &c.args, false, yes, cfg)?;
            }
            Ok(())
        }
    }
}

fn scope_label(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::User => "user",
        ScopeKind::Folder => "folder",
    }
}

fn scope_flag(scope: &Scope) -> String {
    match scope {
        Scope::User => "--user".into(),
        Scope::Folder(p) if p.as_os_str().is_empty() => "--folder".into(),
        Scope::Folder(p) => format!("--folder {}", p.display()),
    }
}
