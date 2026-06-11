use anyhow::{bail, Result};
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::path::Path;

use crate::commands::catalog::{run as catalog_run, CatalogAction};
use crate::config::{catalog_root, Config};

pub fn ensure_catalog(cfg: &Config) -> Result<()> {
    let root = catalog_root(cfg);
    if is_catalog_present(&root) {
        if !is_tty() {
            println!("Catalog already exists at {}.", root.display());
        }
        return Ok(());
    }

    // Non-interactive: auto-install example
    if !is_tty() {
        println!("No catalog found. Installing example catalog...");
        catalog_run(cfg, CatalogAction::UseExample)?;
        return Ok(());
    }

    // Interactive: prompt for setup
    println!("No dj catalog found at {}\n", root.display());
    println!("How would you like to set up your catalog?\n");
    println!("  [1] Install the example starter catalog (recommended for new users)");
    println!("  [2] Fetch a catalog from a GitHub repository (e.g. thinkjones/dj-catalog-example)");
    println!("  [3] Use an existing local folder");
    println!("  [q] Quit\n");

    print!("Choice: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let result = match input.trim() {
        "1" => catalog_run(cfg, CatalogAction::UseExample),
        "2" => {
            print!("GitHub repo (owner/repo): ");
            io::stdout().flush()?;
            let mut repo = String::new();
            io::stdin().read_line(&mut repo)?;
            let repo = repo.trim();
            if repo.is_empty() {
                bail!("repo cannot be empty");
            }
            catalog_run(
                cfg,
                CatalogAction::Fetch {
                    repo: repo.to_string(),
                    branch: "main".to_string(),
                },
            )
        }
        "3" => {
            print!("Local folder path: ");
            io::stdout().flush()?;
            let mut path = String::new();
            io::stdin().read_line(&mut path)?;
            let path = std::path::PathBuf::from(path.trim());
            catalog_run(cfg, CatalogAction::UseLocal { path })
        }
        "q" | "Q" => {
            println!("Run `dj onboard` when ready.");
            bail!("no catalog configured");
        }
        _ => {
            bail!("invalid choice");
        }
    };

    // After successful install, show getting-started guide
    if result.is_ok() {
        println!("\n{}", "✓ Catalog installed successfully!".green().bold());
        println!();
        println!("{}", "Common commands:".bold());
        println!("  dj doctor            Check what’s installed and what’s missing");
        println!("  dj doctor --detail   Show full status for every catalog item");
        println!("  dj plugins           List all plugins and their catalog config");
        println!();
        println!("{}", "Run a plugin:".bold());
        println!("  dj <plugin> --user          Run a plugin for your user");
        println!("  dj <plugin> --folder        Run a plugin for the current folder");
        println!("  dj <plugin> --dry-run ...   Preview what a plugin would do");
        println!();
        println!("{}", "Get help for any plugin:".bold());
        println!("  dj <plugin>          Show usage, config path, and items");
        println!();
    }

    result
}

fn is_catalog_present(root: &Path) -> bool {
    root.join("workflows").is_dir() || root.join("brew").exists()
}

fn is_tty() -> bool {
    atty::is(atty::Stream::Stdin)
}
