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
        Resolved::None => bail!(
            "unknown plugin or workflow: '{}'. Try `dj --help`.",
            inv.name
        ),
        Resolved::Both => {
            if inv.verb.mutates() {
                bail!(
                    "'{}' is both a plugin and a workflow. Disambiguate: \
                     `dj plugin:{} install …` or `dj workflow:{} install …`.",
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
            print_plugin_info(cfg, rt, inv, false);
            Ok(())
        }
        Verb::Status => {
            print_plugin_info(cfg, rt, inv, true);
            Ok(())
        }
        Verb::Version => {
            println!("{} {}", p.manifest().name, p.manifest().version);
            Ok(())
        }
        Verb::Install => {
            let mut scope = inv
                .scope
                .clone()
                .expect("install has scope (parser-enforced)");

            // Prompt for folder path if not provided
            if let Scope::Folder(ref path) = scope {
                if path.as_os_str().is_empty() {
                    use std::io::Write as _;
                    print!("No --path provided for folder scope. Use current directory? [Y/n] ");
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("n") {
                        println!("Aborted.");
                        return Ok(());
                    }
                    scope = Scope::Folder(std::env::current_dir().unwrap_or_default());
                }
            }

            rt.run(&inv.name, &scope, &inv.args, inv.dry_run, yes, cfg)
        }
    }
}

fn print_plugin_info(cfg: &Config, rt: &Runtime, inv: &Invocation, show_status: bool) {
    let p = rt.plugin(&inv.name).expect("resolved as plugin");
    let m = p.manifest();

    println!("{} — {}", m.name.bold(), m.summary);
    println!();

    println!("{}", "Usage".bold());
    if m.scopes.contains(&ScopeKind::User) {
        println!(
            "  dj {} install --scope user {}",
            m.name,
            "Run for user scope".dimmed()
        );
    }
    if m.scopes.contains(&ScopeKind::Folder) {
        println!(
            "  dj {} install --scope folder [--path PATH] {}",
            m.name,
            "Run for folder/project scope".dimmed()
        );
    }
    println!(
        "  dj {} status --scope user|folder {}",
        m.name,
        "Show items and install status".dimmed()
    );
    println!(
        "  dj {} --dry-run ...              {}",
        m.name,
        "Preview changes without applying".dimmed()
    );
    println!();

    let root = crate::config::catalog_root(cfg);
    let default_config = "config.md".to_string();
    let config_file = m
        .config
        .get(&ScopeKind::User)
        .or_else(|| m.config.get(&ScopeKind::Folder))
        .unwrap_or(&default_config);
    let config_path = root.join(&m.name).join(config_file);
    println!("{} {}", "Catalog:".bold(), config_path.display());
    println!();

    if show_status {
        if let Some(items) = crate::commands::list::plugin_items(cfg, &m.name) {
            println!("{} ({})", "Items".bold(), items.len());
            for (name, installed) in items {
                let marker = if installed {
                    "✅ installed".green().to_string()
                } else {
                    "❌ missing".red().to_string()
                };
                println!("  {} {}", marker, name);
            }
        } else {
            let user_scope = Scope::User;
            if let Ok(items) = rt.list_one(&m.name, &user_scope, cfg) {
                if !items.is_empty() {
                    println!("{} ({})", "Items".bold(), items.len());
                    for item in items {
                        println!("  • {}", item);
                    }
                }
            }
        }
    } else {
        let user_scope = Scope::User;
        if let Ok(items) = rt.list_one(&m.name, &user_scope, cfg) {
            if !items.is_empty() {
                println!("{} ({})", "Items".bold(), items.len());
                for item in items {
                    println!("  • {}", item);
                }
            }
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
        Verb::Status => {
            let wf = rt.workflows().get(&inv.name);
            match wf {
                Some(wf) => {
                    println!("{} — workflow", inv.name.bold());
                    println!("\n{}", "User steps".bold());
                    for step in &wf.user {
                        println!("  • {}", step.name);
                    }
                    println!("\n{}", "Folder steps".bold());
                    for step in &wf.folder {
                        println!("  • {}", step.name);
                    }
                }
                None => println!("{} — workflow (no details)", inv.name.bold()),
            }
            Ok(())
        }
        Verb::Install => {
            let mut scope = inv.scope.clone().expect("install has scope");

            // Prompt for folder path if not provided
            if let Scope::Folder(ref path) = scope {
                if path.as_os_str().is_empty() {
                    use std::io::Write as _;
                    print!("No --path provided for folder scope. Use current directory? [Y/n] ");
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().eq_ignore_ascii_case("n") {
                        println!("Aborted.");
                        return Ok(());
                    }
                    scope = Scope::Folder(std::env::current_dir().unwrap_or_default());
                }
            }

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
                        "  dj {} install {} {} {}",
                        c.plugin,
                        scope_flag(&scope),
                        c.args.join(" "),
                        "--dry-run".dimmed()
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
        Scope::User => "--scope user".into(),
        Scope::Folder(p) if p.as_os_str().is_empty() => "--scope folder".into(),
        Scope::Folder(p) => format!("--scope folder --path {}", p.display()),
    }
}
