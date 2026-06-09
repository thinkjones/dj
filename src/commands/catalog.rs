use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{catalog_root, Config};
use crate::plugins::runtime::Runtime;

pub enum CatalogAction {
    Info,
    UseExample,
    UseLocal { path: PathBuf },
    Fetch { repo: String, branch: String },
    List,
}

pub fn run(cfg: &Config, action: CatalogAction) -> Result<()> {
    match action {
        CatalogAction::Info => cmd_info(cfg),
        CatalogAction::UseExample => cmd_use_example(cfg),
        CatalogAction::UseLocal { path } => cmd_use_local(cfg, &path),
        CatalogAction::Fetch { repo, branch } => cmd_fetch(cfg, &repo, &branch),
        CatalogAction::List => cmd_list(cfg),
    }
}

fn cmd_info(cfg: &Config) -> Result<()> {
    let root = catalog_root(cfg);
    println!("Catalog root: {}", root.display());

    if let Some(meta) = &cfg.catalog {
        if let Some(src) = &meta.source {
            println!("Source: {}", src);
        }
    }

    if !root.exists() {
        println!("Status: not installed");
        return Ok(());
    }

    // Count plugin configs present
    let plugins = ["brew", "runtimes", "custom", "symlinks", "shell", "apm", "claude", "permissions"];
    let mut present = 0;
    for name in &plugins {
        if root.join(name).exists() {
            present += 1;
        }
    }
    println!("Plugins with config: {}/{}", present, plugins.len());

    // Load runtime and show workflow count
    match Runtime::load(cfg) {
        Ok(rt) => {
            let workflows = rt.workflows().len();
            println!("Workflows: {}", workflows);
        }
        Err(_) => {
            println!("Workflows: unable to load");
        }
    }

    Ok(())
}

fn cmd_use_example(cfg: &Config) -> Result<()> {
    let dest = catalog_root(cfg);

    // Try local source first (when running from source)
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join("catalog");

    if example.exists() {
        copy_dir_all(&example, &dest)?;
    } else {
        // Extract embedded example catalog shipped inside the binary
        crate::example_catalog::extract(&dest)?;
    }

    // Update config.toml with source
    write_catalog_source(cfg, "example")?;

    println!("Example catalog installed to {}", dest.display());
    Ok(())
}

fn cmd_use_local(cfg: &Config, path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("path does not exist: {}", path.display());
    }

    if !is_valid_catalog(path) {
        bail!("path does not look like a dj catalog (missing plugin configs or workflows.md)");
    }

    // Write config.toml with new catalog_root
    let config_path = crate::config::config_path();
    let content = fs::read_to_string(&config_path).unwrap_or_default();
    let mut doc: toml::Value = content.parse().unwrap_or(toml::Value::Table(toml::Table::new()));

    if let Some(table) = doc.as_table_mut() {
        table.insert("catalog_root".to_string(), toml::Value::String(path.to_string_lossy().to_string()));
    }

    fs::write(&config_path, doc.to_string())?;
    write_catalog_source(cfg, "local")?;

    println!("Catalog root set to {}", path.display());
    Ok(())
}

fn cmd_fetch(cfg: &Config, repo: &str, branch: &str) -> Result<()> {
    let dest = catalog_root(cfg);

    // Ensure parent exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // Remove existing catalog if present
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }

    // Try gh first, fallback to git
    let has_gh = Command::new("gh").arg("auth").arg("status").output().map(|o| o.status.success()).unwrap_or(false);

    let status = if has_gh {
        Command::new("gh")
            .args(["repo", "clone", repo, &dest.to_string_lossy(), "--", "--branch", branch])
            .status()
    } else {
        let url = format!("https://github.com/{}.git", repo);
        Command::new("git")
            .args(["clone", "--branch", branch, &url, &dest.to_string_lossy()])
            .status()
    };

    match status {
        Ok(s) if s.success() => {
            write_catalog_source(cfg, &format!("github:{}", repo))?;
            println!("Catalog fetched from {} to {}", repo, dest.display());
            Ok(())
        }
        _ => {
            bail!("failed to fetch catalog from {}. Ensure gh is authenticated or the repo is public.", repo);
        }
    }
}

fn cmd_list(cfg: &Config) -> Result<()> {
    // Delegate to existing list command
    crate::commands::list::run(cfg)
}

fn is_valid_catalog(path: &Path) -> bool {
    path.join("workflows.md").exists() || path.join("brew").exists()
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

fn write_catalog_source(_cfg: &Config, source: &str) -> Result<()> {
    let config_path = crate::config::config_path();
    let content = fs::read_to_string(&config_path).unwrap_or_default();
    let mut doc: toml::Value = content.parse().unwrap_or(toml::Value::Table(toml::Table::new()));

    if let Some(table) = doc.as_table_mut() {
        let catalog_table = table.entry("catalog").or_insert_with(|| toml::Value::Table(toml::Table::new()));
        if let Some(cat) = catalog_table.as_table_mut() {
            cat.insert("source".to_string(), toml::Value::String(source.to_string()));
        }
    }

    fs::write(&config_path, doc.to_string())?;
    Ok(())
}
