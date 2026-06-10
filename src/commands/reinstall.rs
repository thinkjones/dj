use anyhow::{bail, Result};
use dirs::home_dir;
use owo_colors::OwoColorize;
use std::fs;
use std::io::{self, Write};
use std::process::Command;

use crate::config::Config;

pub fn run(_cfg: &Config, yes: bool) -> Result<()> {
    println!(
        "{}",
        "WARNING: dj reinstall will replace your dj installation."
            .red()
            .bold()
    );
    println!("This will:");
    println!("  Back up ~/.local/bin/dj → ~/.local/bin/dj.old");
    println!("  Back up ~/.config/dj/ → ~/.config/dj.old/");
    println!("  Download and install the latest dj release.\n");

    if !yes {
        print!("Type 'yes' to confirm: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() != "yes" {
            bail!("Aborted.");
        }
    }

    let home = home_dir().ok_or_else(|| anyhow::anyhow!("cannot find home directory"))?;
    let bin = home.join(".local").join("bin").join("dj");
    let config_dir = home.join(".config").join("dj");
    let bin_old = home.join(".local").join("bin").join("dj.old");
    let config_old = home.join(".config").join("dj.old");

    // Pre-flight: check bootstrap URL is reachable
    let bootstrap_url = "https://raw.githubusercontent.com/thinkjones/dj/main/bootstrap.sh";
    let check = Command::new("curl")
        .args(["-fsSL", "-I", bootstrap_url])
        .output()?;
    if !check.status.success() {
        bail!("bootstrap URL is not reachable — check your connection and try again");
    }

    // Backup binary
    if bin.exists() {
        if bin_old.exists() {
            fs::remove_file(&bin_old)?;
        }
        fs::rename(&bin, &bin_old)?;
        println!("  Backed up {} → {}", bin.display(), bin_old.display());
    }

    // Backup config
    if config_dir.exists() {
        if config_old.exists() {
            fs::remove_dir_all(&config_old)?;
        }
        fs::rename(&config_dir, &config_old)?;
        println!(
            "  Backed up {} → {}",
            config_dir.display(),
            config_old.display()
        );
    }

    // Download and run bootstrap
    println!("\n{}", "Re-downloading dj from latest release...".cyan());
    let status = Command::new("sh")
        .args(["-c", &format!("$(curl -fsSL {})", bootstrap_url)])
        .status();

    match status {
        Ok(s) if s.success() => {
            // Clean up old backups
            if bin_old.exists() {
                fs::remove_file(&bin_old)?;
            }
            if config_old.exists() {
                fs::remove_dir_all(&config_old)?;
            }
            println!("{}", "✓ dj reinstalled.".green());
            Ok(())
        }
        _ => {
            // Rollback
            println!(
                "{}",
                "✗ Reinstall failed — restoring previous installation...".red()
            );
            if bin_old.exists() {
                if bin.exists() {
                    fs::remove_file(&bin)?;
                }
                fs::rename(&bin_old, &bin)?;
                println!("  Restored {}", bin.display());
            }
            if config_old.exists() {
                if config_dir.exists() {
                    fs::remove_dir_all(&config_dir)?;
                }
                fs::rename(&config_old, &config_dir)?;
                println!("  Restored {}", config_dir.display());
            }
            bail!("reinstall failed — previous installation restored");
        }
    }
}

pub fn uninstall(_cfg: &Config, yes: bool) -> Result<()> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("cannot find home directory"))?;
    let bin = home.join(".local").join("bin").join("dj");

    println!("This will remove: {}", bin.display());
    println!("Catalog and config are left intact.\n");

    if !yes {
        print!("Confirm? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    if bin.exists() {
        std::fs::remove_file(&bin)?;
        println!("{}", "✓ dj uninstalled.".green());
    } else {
        println!("Binary not found at {} — nothing to remove.", bin.display());
    }

    Ok(())
}
