use anyhow::{bail, Result};
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::process::Command;

use crate::config::{catalog_root, Config};

pub fn run(cfg: &Config, dry_run: bool, yes: bool) -> Result<()> {
    let root = catalog_root(cfg);
    let chezmoi_source = root.join("chezmoi");

    if !chezmoi_source.exists() {
        bail!(
            "chezmoi source directory not found: {}",
            chezmoi_source.display()
        );
    }

    let source_arg = format!("--source={}", chezmoi_source.display());

    if dry_run {
        println!("{}", "Showing diff (dry run)...".cyan());
        let status = Command::new("chezmoi")
            .args(["apply", "--dry-run", "--verbose", &source_arg])
            .status()?;
        if !status.success() {
            bail!("chezmoi dry-run failed");
        }
        return Ok(());
    }

    // Show diff first
    println!("{}", "Calculating diff...".cyan());
    let diff = Command::new("chezmoi").args(["diff", &source_arg]).output()?;
    let diff_output = String::from_utf8_lossy(&diff.stdout);

    if diff_output.trim().is_empty() {
        println!("{}", "Already in sync — nothing to apply.".green());
        return Ok(());
    }

    println!("{}", diff_output);

    if !yes {
        print!("Apply these changes? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("{}", "Applying dotfiles...".cyan());
    let status = Command::new("chezmoi")
        .args(["apply", &source_arg])
        .status()?;

    if !status.success() {
        bail!("chezmoi apply failed");
    }

    println!("{}", "✓ Dotfiles applied.".green());
    Ok(())
}
