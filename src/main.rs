mod catalog;
mod cli;
mod commands;
mod config;
mod example_catalog;
mod permissions;
mod plugins;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use cli::scope::{Scope, ScopeKind};
use config::Config;
use owo_colors::OwoColorize;

#[derive(Parser)]
#[command(
    name = "dj",
    version,
    about = "macOS developer workstation manager",
    allow_external_subcommands = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Health check across all plugins
    Doctor {
        /// Show full detail for all items, not just problems
        #[arg(long)]
        detail: bool,
    },
    /// Show dj + plugin versions
    Version,
    /// Set up your catalog for the first time
    Onboard,
    /// Regenerate all artifacts from catalog
    Rebuild,
    /// DESTRUCTIVE: wipe and reinstall dj
    Reinstall {
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Uninstall dj binary
    Uninstall {
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Generate/install shell completions
    Completions {
        #[command(subcommand)]
        cmd: CompletionsCmd,
    },
    /// Show all installed plugins and their catalog actions
    Plugins,
    /// <plugin|workflow> [verb] [args] [--user|--folder] [--dry-run]
    #[command(external_subcommand)]
    External(Vec<String>),
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

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    let cfg = Config::load().unwrap_or_else(|_| Config {
        catalog_root: dirs::home_dir()
            .unwrap_or_default()
            .join(".config/dj/catalog"),
        default_agent_stack: "core".into(),
    });

    let rt = plugins::runtime::Runtime::load(&cfg)?;

    // Warn if catalog pins differ from built-in plugin versions
    check_catalog_versions(&cfg, &rt);

    // Custom help: no args or --help
    if args.len() <= 1 || args[1] == "--help" || args[1] == "-h" {
        print_custom_help(&rt);
        return Ok(());
    }

    // Parse with clap for everything else
    let cli = Cli::parse();

    match cli.command {
        None => {
            print_custom_help(&rt);
        }
        Some(Commands::Rebuild) => commands::rebuild::run(&cfg)?,
        Some(Commands::Reinstall { yes }) => commands::reinstall::run(&cfg, yes)?,
        Some(Commands::Uninstall { yes }) => commands::reinstall::uninstall(&cfg, yes)?,
        Some(Commands::Completions { cmd }) => {
            let mut cli_cmd = Cli::command();
            match cmd {
                CompletionsCmd::Bash => {
                    commands::completions::print_completions(Shell::Bash, &mut cli_cmd)
                }
                CompletionsCmd::Fish => {
                    commands::completions::print_completions(Shell::Fish, &mut cli_cmd)
                }
                CompletionsCmd::Zsh => {
                    commands::completions::print_completions(Shell::Zsh, &mut cli_cmd)
                }
                CompletionsCmd::Install { yes } => {
                    commands::completions::install(yes, &mut cli_cmd)?
                }
            }
        }
        Some(Commands::Doctor { detail }) => {
            commands::list::run(&cfg, detail)?;

            // Plugin-level doctor details only shown in detail mode or when unhealthy
            for p in rt.plugin_iter() {
                let m = p.manifest();
                for kind in &m.scopes {
                    let scope = match kind {
                        ScopeKind::User => Scope::User,
                        ScopeKind::Folder => Scope::Folder(std::path::PathBuf::new()),
                    };
                    match rt.doctor_one(&m.name, &scope, &cfg) {
                        Ok(health) => {
                            if detail || !matches!(health.status, crate::plugins::HealthStatus::Ok)
                            {
                                println!("{}@{} — {:?}", m.name, scope_label(*kind), health.status);
                                for d in &health.details {
                                    println!("  {}", d);
                                }
                            }
                        }
                        Err(e) => eprintln!("{}@{} — error: {}", m.name, scope_label(*kind), e),
                    }
                }
            }
        }
        Some(Commands::Version) => {
            println!("dj {}", env!("CARGO_PKG_VERSION"));
            for m in rt.manifests() {
                println!("{} {}", m.name, m.version);
            }
        }
        Some(Commands::Plugins) => {
            use comfy_table::Table;

            fn plugins_table(cfg: &Config, rt: &plugins::runtime::Runtime, user_only: bool) {
                let root = config::catalog_root(cfg);
                let mut t = Table::new();
                t.set_header(["Plugin", "Description", "Catalog", "Items"]);
                t.set_width(100);

                let mut total_items = 0usize;
                let mut installed_items = 0usize;
                let mut missing_items = 0usize;

                for p in rt.plugin_iter() {
                    let m = p.manifest();
                    let is_user = m.scopes.contains(&ScopeKind::User);
                    let is_folder = m.scopes.contains(&ScopeKind::Folder);
                    if user_only && (is_folder || !is_user) {
                        continue;
                    }
                    if !user_only && !is_folder {
                        continue;
                    }

                    let default_config = "config.md".to_string();
                    let config_file = m
                        .config
                        .get(&ScopeKind::User)
                        .or_else(|| m.config.get(&ScopeKind::Folder))
                        .unwrap_or(&default_config);
                    let catalog_path = format!("{}/{}", m.name, config_file);
                    let full_path = root.join(&m.name).join(config_file);
                    let configured = full_path.exists();

                    let items = if configured {
                        match commands::list::plugin_counts(cfg, &m.name) {
                            Some(c) => {
                                total_items += c.total;
                                installed_items += c.installed;
                                missing_items += c.missing;
                                if c.total == 0 {
                                    format!("{}", "configured".dimmed())
                                } else if c.missing == 0 {
                                    format!("{}", format!("{} installed", c.total).green())
                                } else {
                                    format!(
                                        "{}",
                                        format!("{} / {} missing", c.installed, c.missing).yellow()
                                    )
                                }
                            }
                            None => format!("{}", "configured".dimmed()),
                        }
                    } else {
                        format!("{}", "not configured".dimmed())
                    };

                    t.add_row([&m.name, &m.summary, &catalog_path, &items]);
                }
                println!("{t}");

                if total_items > 0 {
                    if missing_items == 0 {
                        println!(
                            "{}",
                            format!("✅ All {total_items} items installed").green()
                        );
                    } else {
                        println!(
                            "{}",
                            format!(
                                "⚠️  {missing_items} of {total_items} items missing ({installed_items} installed)"
                            )
                            .yellow()
                        );
                    }
                }
            }

            println!("{}", "Machine / User Setup".bold());
            plugins_table(&cfg, &rt, true);
            println!("\n{}", "Folder / Project Setup".bold());
            plugins_table(&cfg, &rt, false);
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

fn print_custom_help(rt: &plugins::runtime::Runtime) {
    println!("macOS developer workstation manager\n");
    println!("Usage: dj [COMMAND]\n");

    println!("{}", "Machine / User Setup".bold());
    for p in rt.plugin_iter() {
        let m = p.manifest();
        if m.scopes.contains(&ScopeKind::User) && !m.scopes.contains(&ScopeKind::Folder) {
            println!("  {:<12} {}", m.name, m.summary);
        }
    }

    println!("\n{}", "Folder / Project Setup".bold());
    for p in rt.plugin_iter() {
        let m = p.manifest();
        if m.scopes.contains(&ScopeKind::Folder) {
            println!("  {:<12} {}", m.name, m.summary);
        }
    }

    println!("\n{}", "Tool Management".bold());
    println!("  {:<12} Regenerate all artifacts from catalog", "rebuild");
    println!("  {:<12} DESTRUCTIVE: wipe and reinstall dj", "reinstall");
    println!("  {:<12} Uninstall dj binary", "uninstall");
    println!("  {:<12} Generate/install shell completions", "completions");
    println!(
        "  {:<12} Show all installed plugins and catalog actions",
        "plugins"
    );
    println!("  {:<12} Health check across all plugins", "doctor");
    println!("  {:<12} Show dj + plugin versions", "version");
    println!("  {:<12} Set up your catalog for the first time", "onboard");
    println!(
        "  {:<12} Print this message or the help of the given subcommand(s)",
        "help"
    );

    println!("\nOptions:");
    println!("  -h, --help     Print help");
    println!("  -V, --version  Print version");
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
            "  Update dj or adjust dj-catalog.toml to resolve."
                .yellow()
                .dimmed()
        );
    }
}
