mod catalog;
mod cli;
mod commands;
mod config;
mod example_catalog;
mod permissions;
mod plugins;

use anyhow::bail;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use cli::scope::{Scope, ScopeKind};
use config::Config;
use owo_colors::OwoColorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dj", version, about = "macOS developer workstation manager", allow_external_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Tool self-management
    #[command(name = "self")]
    Selff {
        #[command(subcommand)]
        action: SelfAction,
    },
    /// Catalog management
    Catalog {
        #[command(subcommand)]
        action: CatalogAction,
    },
    /// Health check across all plugins
    Doctor {
        #[arg(long)]
        explain: bool,
        #[arg(long)]
        json: bool,
    },
    /// List all catalog entries
    List,
    /// Show dj + plugin versions
    Version,
    /// Overview of installed plugins & workflows
    Info,
    /// Set up your catalog for the first time
    Onboard,
    /// <plugin|workflow> [verb] [args] [--user|--folder] [--dry-run]
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand)]
enum SelfAction {
    /// Regenerate all artifacts from catalog
    Rebuild,
    /// DESTRUCTIVE: wipe and reinstall dj
    Reinstall { #[arg(long, short = 'y')] yes: bool },
    /// Uninstall dj binary
    Uninstall { #[arg(long, short = 'y')] yes: bool },
    /// Generate/install shell completions
    Completions {
        #[command(subcommand)]
        cmd: CompletionsCmd,
    },
}

#[derive(Subcommand)]
enum CompletionsCmd {
    /// Print bash completions to stdout
    Bash,
    /// Print fish completions to stdout
    Fish,
    /// Print zsh completions to stdout
    Zsh,
    /// Auto-detect current shell and install completions
    Install {
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum CatalogAction {
    /// Show catalog info
    Info,
    /// Use a catalog source
    Use {
        #[arg(long)]
        example: bool,
        path: Option<String>,
    },
    /// Fetch a catalog from a GitHub repo
    Fetch {
        repo: String,
        #[arg(long, default_value = "main")]
        branch: String,
    },
    /// List all catalog entries
    List,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let cfg = Config::load().unwrap_or_else(|_| Config {
        catalog_root: dirs::home_dir()
            .unwrap_or_default()
            .join(".config/dj/catalog"),
        default_agent_stack: "core".into(),
        catalog: None,
    });

    let rt = plugins::runtime::Runtime::load(&cfg)?;

    // Warn if catalog pins differ from built-in plugin versions
    check_catalog_versions(&cfg, &rt);

    match cli.command {
        None => {
            println!("dj — macOS developer workstation manager");
            println!("Run `dj --help`, `dj list`, or `dj <plugin> info`.");
        }
        Some(Commands::Selff { action }) => match action {
            SelfAction::Rebuild => commands::rebuild::run(&cfg)?,
            SelfAction::Reinstall { yes } => commands::reinstall::run(&cfg, yes)?,
            SelfAction::Uninstall { yes } => commands::reinstall::uninstall(&cfg, yes)?,
            SelfAction::Completions { cmd } => {
                let mut cli_cmd = Cli::command();
                match cmd {
                    CompletionsCmd::Bash => commands::completions::print_completions(Shell::Bash, &mut cli_cmd),
                    CompletionsCmd::Fish => commands::completions::print_completions(Shell::Fish, &mut cli_cmd),
                    CompletionsCmd::Zsh => commands::completions::print_completions(Shell::Zsh, &mut cli_cmd),
                    CompletionsCmd::Install { yes } => commands::completions::install(yes, &mut cli_cmd)?,
                }
            }
        },
        Some(Commands::Catalog { action }) => match action {
            CatalogAction::Info => commands::catalog::run(&cfg, commands::catalog::CatalogAction::Info)?,
            CatalogAction::Use { example, path } => {
                if example {
                    commands::catalog::run(&cfg, commands::catalog::CatalogAction::UseExample)?
                } else if let Some(path) = path {
                    commands::catalog::run(&cfg, commands::catalog::CatalogAction::UseLocal { path: PathBuf::from(path) })?
                } else {
                    bail!("dj catalog use requires --example or a path");
                }
            }
            CatalogAction::Fetch { repo, branch } => {
                commands::catalog::run(&cfg, commands::catalog::CatalogAction::Fetch { repo, branch })?
            }
            CatalogAction::List => commands::catalog::run(&cfg, commands::catalog::CatalogAction::List)?,
        },
        Some(Commands::Doctor { explain: _, json: _ }) => {
            // Global doctor: fan out over all plugins for all supported scopes
            for p in rt.plugin_iter() {
                let m = p.manifest();
                for kind in &m.scopes {
                    let scope = match kind {
                        ScopeKind::User => Scope::User,
                        ScopeKind::Folder => Scope::Folder(std::path::PathBuf::new()),
                    };
                    match rt.doctor_one(&m.name, &scope, &cfg) {
                        Ok(health) => {
                            println!("{}@{} — {:?}", m.name, scope_label(*kind), health.status);
                            for d in &health.details {
                                println!("  {}", d);
                            }
                        }
                        Err(e) => eprintln!("{}@{} — error: {}", m.name, scope_label(*kind), e),
                    }
                }
            }
        }
        Some(Commands::List) => {
            // Global list: fan out over all plugins
            for p in rt.plugin_iter() {
                let m = p.manifest();
                let scope = if m.scopes.contains(&ScopeKind::User) {
                    Scope::User
                } else {
                    Scope::Folder(std::path::PathBuf::new())
                };
                match rt.list_one(&m.name, &scope, &cfg) {
                    Ok(items) => {
                        if !items.is_empty() {
                            println!("{}:", m.name);
                            for item in items {
                                println!("  {}", item);
                            }
                        }
                    }
                    Err(e) => eprintln!("{} — error: {}", m.name, e),
                }
            }
        }
        Some(Commands::Version) => {
            println!("dj {}", env!("CARGO_PKG_VERSION"));
            for m in rt.manifests() {
                println!("{} {}", m.name, m.version);
            }
        }
        Some(Commands::Info) => {
            println!("Plugins:");
            for m in rt.manifests() {
                println!("  {:<12} {}", m.name, m.summary);
            }
            println!("Workflows:");
            let mut names: Vec<_> = rt.workflows().keys().collect();
            names.sort();
            for n in names {
                println!("  {n}");
            }
        }
        Some(Commands::Onboard) => {
            cli::onboarding::ensure_catalog(&cfg)?;
        }
        Some(Commands::External(tokens)) => {
            let inv = cli::parse::parse(&tokens)?;
            cli::dispatch::dispatch(&cfg, &rt, &inv, true)?;
        }
    }

    Ok(())
}

fn scope_label(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::User => "user",
        ScopeKind::Folder => "folder",
    }
}

fn check_catalog_versions(cfg: &Config, rt: &plugins::runtime::Runtime) {
    let root = config::catalog_root(cfg);
    let catalog_toml = root.join("dj-catalog.toml");
    if !catalog_toml.exists() {
        return;
    }
    let content = match std::fs::read_to_string(&catalog_toml) {
        Ok(c) => c,
        Err(_) => return,
    };
    let doc: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return,
    };
    let plugin_table = doc.get("plugins").and_then(|p| p.as_table());
    let Some(table) = plugin_table else { return };

    let mut mismatches = Vec::new();
    for (name, pinned) in table {
        let pinned_str = pinned.as_str().unwrap_or("");
        if let Some(plugin) = rt.plugin(name) {
            let built_in = plugin.manifest().version.as_str();
            if pinned_str != built_in {
                mismatches.push(format!(
                    "  {}: catalog={} built-in={}",
                    name, pinned_str, built_in
                ));
            }
        }
    }

    if !mismatches.is_empty() {
        println!(
            "{}",
            "⚠ Catalog plugin version pins differ from built-in plugins:".yellow()
        );
        for m in mismatches {
            println!("{}", m.yellow());
        }
        println!(
            "{}",
            "  Update dj or adjust dj-catalog.toml to resolve.".yellow().dimmed()
        );
    }
}
