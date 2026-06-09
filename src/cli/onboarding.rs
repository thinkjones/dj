use anyhow::{bail, Result};
use std::io::{self, Write};
use std::path::Path;

use crate::config::{catalog_root, Config};
use crate::commands::catalog::{CatalogAction, run as catalog_run};

pub fn ensure_catalog(cfg: &Config) -> Result<()> {
    let root = catalog_root(cfg);
    let had_catalog = is_catalog_present(&root);

    // Non-interactive: auto-install example
    if !is_tty() {
        if !had_catalog {
            println!("No catalog found. Installing example catalog...");
            catalog_run(cfg, CatalogAction::UseExample)?;
        }
        return Ok(());
    }

    // Always prompt in interactive mode
    if had_catalog {
        println!("A catalog already exists at {}\n", root.display());
    } else {
        println!("No dj catalog found at {}\n", root.display());
    }
    println!("How would you like to set up your catalog?\n");
    println!("  [1] Install the example starter catalog (recommended for new users)");
    println!("  [2] Fetch a catalog from a GitHub repository");
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
            catalog_run(cfg, CatalogAction::Fetch { repo: repo.to_string(), branch: "main".to_string() })
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

    // After successful install, run doctor
    if result.is_ok() {
        println!("\n→ Running dj doctor...\n");
        let _ = crate::commands::list::run(cfg);
    }

    result
}

fn is_catalog_present(root: &Path) -> bool {
    root.join("workflows.md").exists() || root.join("brew").exists()
}

fn is_tty() -> bool {
    atty::is(atty::Stream::Stdin)
}
