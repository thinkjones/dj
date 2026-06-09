use anyhow::Result;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};
use owo_colors::OwoColorize;
use std::collections::HashSet;
use std::process::Command;

use crate::{
    catalog::{arch_compatible, BrewKind},
    config::{catalog_root, Config},
    plugins::brew::config::parse as parse_brew,
    plugins::custom::config::parse as parse_custom,
    plugins::runtimes::config::{parse_package_managers, parse_runtimes},
    plugins::shell::config::parse as parse_shell,
    plugins::symlinks::config::parse as parse_symlinks,
};

pub fn run(cfg: &Config) -> Result<()> {
    let root = catalog_root(cfg);

    let brew = parse_brew(&root.join("brew").join("config.md")).unwrap_or_default();
    let runtimes = parse_runtimes(&root.join("runtimes").join("config.md")).unwrap_or_default();
    let package_managers = parse_package_managers(&root.join("runtimes").join("config.md")).unwrap_or_default();
    let custom_installs = parse_custom(&root.join("custom").join("config.md")).unwrap_or_default();
    let shell_functions = parse_shell(&root.join("shell").join("config.md")).unwrap_or_default();
    let symlinks = parse_symlinks(&root.join("symlinks").join("config.md")).unwrap_or_default();

    let formulae: Vec<_> = brew.iter().filter(|e| e.kind == BrewKind::Formula).collect();
    let casks: Vec<_> = brew.iter().filter(|e| e.kind == BrewKind::Cask).collect();
    let taps: Vec<_> = brew.iter().filter(|e| e.kind == BrewKind::Tap).collect();

    let installed_casks = installed_cask_set();

    if !formulae.is_empty() {
        section("Formulae", "CLI tools and libraries installed by Homebrew into /opt/homebrew/bin");
        let mut t = table();
        for e in &formulae {
            let check_name = e.bin.as_deref().unwrap_or(&e.name);
            let status = arch_status(&e.arch, || yn(check_binary(check_name)));
            let desc = desc_with_plugin(&e.description, &e.zsh_plugin);
            t.add_row([&e.name, &desc, &status]);
        }
        println!("{t}");
    }

    if !casks.is_empty() {
        section("Casks", "macOS GUI apps installed as .app bundles by Homebrew Cask");
        let mut t = table();
        for e in &casks {
            let status = arch_status(&e.arch, || yn(installed_casks.contains(&e.name)));
            let desc = desc_with_plugin(&e.description, &e.zsh_plugin);
            t.add_row([&e.name, &desc, &status]);
        }
        println!("{t}");
    }

    if !taps.is_empty() {
        section("Taps", "Third-party Homebrew repositories that unlock extra formulae and casks");
        let mut t = table();
        for e in &taps {
            t.add_row([&e.name, &e.description, &format!("{}", "tapped".dimmed())]);
        }
        println!("{t}");
    }

    if !runtimes.is_empty() {
        section("Runtimes", "Language version pins managed by mise — `mise use -g <name>@<version>`");
        let mut t = table();
        for r in &runtimes {
            let pin = format!("{}@{}", r.name, r.version);
            t.add_row([r.name.as_str(), pin.as_str(), &yn(check_binary(&r.name))]);
        }
        println!("{t}");
    }

    if !package_managers.is_empty() {
        section("Package Managers", "Language-specific package installers bootstrapped outside Homebrew");
        let mut t = table();
        for pm in &package_managers {
            t.add_row([pm.name.as_str(), pm.description.as_str(), &yn(check_binary(&pm.name))]);
        }
        println!("{t}");
    }

    if !custom_installs.is_empty() {
        section("Custom Installs", "Tools with non-Homebrew install scripts — skipped if binary already on PATH");
        let mut t = table();
        for c in &custom_installs {
            t.add_row([c.name.as_str(), c.description.as_str(), &yn(check_binary(&c.name))]);
        }
        println!("{t}");
    }

    if !shell_functions.is_empty() {
        section("Shell Functions", "zsh helpers injected into .zshrc as a sentinel block by `dj rebuild`");
        let mut t = table();
        for f in &shell_functions {
            t.add_row([f.name.as_str(), f.description.as_str(), &format!("{}", "catalog".dimmed())]);
        }
        println!("{t}");
    }

    if !symlinks.is_empty() {
        section("Symlinks", "paths symlinked into their destination directories (see catalog/symlinks/config.md)");
        let (linked, missing): (Vec<_>, Vec<_>) = symlinks
            .iter()
            .partition(|s| crate::catalog::expand_tilde(&s.destination).join(&s.name).exists());
        let names = symlinks.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ");
        let status = if missing.is_empty() {
            format!("{}", format!("✅ all {} linked", linked.len()).green())
        } else {
            let m: Vec<_> = missing.iter().map(|s| s.name.as_str()).collect();
            format!("{}", format!("⚠️  missing: {}", m.join(", ")).yellow())
        };
        let mut t = table();
        t.add_row(["symlinks", &names, &status]);
        println!("{t}");
    }

    Ok(())
}

fn desc_with_plugin(description: &str, zsh_plugin: &Option<String>) -> String {
    match zsh_plugin {
        Some(p) => format!("{} {}", description, format!("[plugin: {p}]").dimmed()),
        None => description.to_string(),
    }
}

fn section(title: &str, subtitle: &str) {
    println!("\n{}", title.bold());
    println!("{}\n", subtitle.dimmed());
}

fn table() -> Table {
    let mut t = Table::new();
    t.load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(["Name", "Description", "Status"]);
    t
}

fn arch_status(arch: &Option<String>, check: impl FnOnce() -> String) -> String {
    if arch_compatible(arch) {
        check()
    } else {
        format!("{}", format!("skipped ({})", arch.as_deref().unwrap_or("unknown")).dimmed())
    }
}

fn yn(ok: bool) -> String {
    if ok {
        format!("{}", "✅ installed".green())
    } else {
        format!("{}", "❌ missing".red())
    }
}

fn check_binary(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn installed_cask_set() -> HashSet<String> {
    Command::new("brew")
        .args(["list", "--cask"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}
