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
    Doctor,
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
        Some(Commands::Doctor) => {
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
        Some(Commands::Version) => {
            println!("dj {}", env!("CARGO_PKG_VERSION"));
            for m in rt.manifests() {
                println!("{} {}", m.name, m.version);
            }
        }
        Some(Commands::Plugins) => {
            println!("{}", "Machine / User Setup".bold());
            for p in rt.plugin_iter() {
                let m = p.manifest();
                if m.scopes.contains(&ScopeKind::User) && !m.scopes.contains(&ScopeKind::Folder) {
                    print_plugin_line(&cfg, m);
                }
            }
            println!("\n{}", "Folder / Project Setup".bold());
            for p in rt.plugin_iter() {
                let m = p.manifest();
                if m.scopes.contains(&ScopeKind::Folder) {
                    print_plugin_line(&cfg, m);
                }
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

fn print_plugin_line(cfg: &Config, m: &plugins::Manifest) {
    let root = config::catalog_root(cfg);
    let config_file = root.join(&m.name).join(
        m.config
            .get(&ScopeKind::User)
            .unwrap_or(&"config.md".to_string()),
    );
    let has_config = config_file.exists();
    let status = if has_config {
        "●".green().to_string()
    } else {
        "○".dimmed().to_string()
    };
    println!("  {:<12} {} {}", m.name, status, m.summary);
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
