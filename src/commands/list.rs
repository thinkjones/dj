use anyhow::Result;
use comfy_table::Table;
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

/// Per-plugin install counts: (total, installed, missing).
#[derive(Default, Copy, Clone)]
pub struct PluginCounts {
    pub total: usize,
    pub installed: usize,
    pub missing: usize,
}

/// Return install counts for a single plugin by name, if the catalog config exists.
pub fn plugin_counts(cfg: &Config, name: &str) -> Option<PluginCounts> {
    let root = catalog_root(cfg);
    match name {
        "brew" => {
            let brew = parse_brew(&root.join("brew").join("config.md")).ok()?;
            let installed_casks = installed_cask_set();
            let mut counts = PluginCounts::default();
            for e in brew {
                if !arch_compatible(&e.arch) {
                    continue;
                }
                let check_name = e.bin.as_deref().unwrap_or(&e.name);
                let installed = check_binary(check_name)
                    || (e.kind == BrewKind::Cask && installed_casks.contains(&e.name));
                counts.total += 1;
                if installed {
                    counts.installed += 1;
                } else {
                    counts.missing += 1;
                }
            }
            Some(counts)
        }
        "runtimes" => {
            let path = root.join("runtimes").join("config.md");
            let runtimes = parse_runtimes(&path).ok()?;
            let pms = parse_package_managers(&path).ok()?;
            let mut counts = PluginCounts::default();
            for r in runtimes {
                counts.total += 1;
                if check_binary(&r.name) {
                    counts.installed += 1;
                } else {
                    counts.missing += 1;
                }
            }
            for pm in pms {
                counts.total += 1;
                if check_binary(&pm.name) {
                    counts.installed += 1;
                } else {
                    counts.missing += 1;
                }
            }
            Some(counts)
        }
        "custom" => {
            let entries = parse_custom(&root.join("custom").join("config.md")).ok()?;
            let mut counts = PluginCounts::default();
            for c in entries {
                counts.total += 1;
                if check_binary(&c.name) {
                    counts.installed += 1;
                } else {
                    counts.missing += 1;
                }
            }
            Some(counts)
        }
        "symlinks" => {
            let entries = parse_symlinks(&root.join("symlinks").join("config.md")).ok()?;
            let mut counts = PluginCounts::default();
            for s in entries {
                counts.total += 1;
                if crate::catalog::expand_tilde(&s.destination)
                    .join(&s.name)
                    .exists()
                {
                    counts.installed += 1;
                } else {
                    counts.missing += 1;
                }
            }
            Some(counts)
        }
        _ => None,
    }
}

pub fn run(cfg: &Config, detail: bool) -> Result<()> {
    let root = catalog_root(cfg);

    let brew = parse_brew(&root.join("brew").join("config.md")).unwrap_or_default();
    let runtimes = parse_runtimes(&root.join("runtimes").join("config.md")).unwrap_or_default();
    let package_managers =
        parse_package_managers(&root.join("runtimes").join("config.md")).unwrap_or_default();
    let custom_installs = parse_custom(&root.join("custom").join("config.md")).unwrap_or_default();
    let shell_functions = parse_shell(&root.join("shell").join("config.md")).unwrap_or_default();
    let symlinks = parse_symlinks(&root.join("symlinks").join("config.md")).unwrap_or_default();

    let formulae: Vec<_> = brew
        .iter()
        .filter(|e| e.kind == BrewKind::Formula)
        .collect();
    let casks: Vec<_> = brew.iter().filter(|e| e.kind == BrewKind::Cask).collect();
    let taps: Vec<_> = brew.iter().filter(|e| e.kind == BrewKind::Tap).collect();

    let installed_casks = installed_cask_set();

    let mut total_items = 0usize;
    let mut installed_items = 0usize;
    let mut missing_items = 0usize;

    println!("Catalog: {}\n", root.display());

    let mut sections: Vec<(String, Table)> = Vec::new();

    if !formulae.is_empty() {
        let (title, t, totals) =
            build_brew_section("Formulae", &formulae, detail, &installed_casks);
        if detail || totals.missing > 0 {
            sections.push((title, t));
        }
        total_items += totals.total;
        installed_items += totals.installed;
        missing_items += totals.missing;
    }

    if !casks.is_empty() {
        let (title, t, totals) = build_brew_section("Casks", &casks, detail, &installed_casks);
        if detail || totals.missing > 0 {
            sections.push((title, t));
        }
        total_items += totals.total;
        installed_items += totals.installed;
        missing_items += totals.missing;
    }

    if !taps.is_empty() && detail {
        let mut t = plain_table();
        for e in &taps {
            t.add_row([&e.name, &e.description, &format!("{}", "tapped".dimmed())]);
        }
        sections.push(("Taps".to_string(), t));
    }

    if !runtimes.is_empty() {
        let mut t = plain_table();
        let mut totals = Totals::default();
        for r in &runtimes {
            let ok = check_binary(&r.name);
            totals.add(ok);
            if detail || !ok {
                let pin = format!("{}@{}", r.name, r.version);
                t.add_row([r.name.as_str(), pin.as_str(), &yn(ok)]);
            }
        }
        if detail || totals.missing > 0 {
            sections.push(("Runtimes".to_string(), t));
        }
        total_items += totals.total;
        installed_items += totals.installed;
        missing_items += totals.missing;
    }

    if !package_managers.is_empty() {
        let mut t = plain_table();
        let mut totals = Totals::default();
        for pm in &package_managers {
            let ok = check_binary(&pm.name);
            totals.add(ok);
            if detail || !ok {
                t.add_row([pm.name.as_str(), pm.description.as_str(), &yn(ok)]);
            }
        }
        if detail || totals.missing > 0 {
            sections.push(("Package Managers".to_string(), t));
        }
        total_items += totals.total;
        installed_items += totals.installed;
        missing_items += totals.missing;
    }

    if !custom_installs.is_empty() {
        let mut t = plain_table();
        let mut totals = Totals::default();
        for c in &custom_installs {
            let ok = check_binary(&c.name);
            totals.add(ok);
            if detail || !ok {
                t.add_row([c.name.as_str(), c.description.as_str(), &yn(ok)]);
            }
        }
        if detail || totals.missing > 0 {
            sections.push(("Custom Installs".to_string(), t));
        }
        total_items += totals.total;
        installed_items += totals.installed;
        missing_items += totals.missing;
    }

    if !shell_functions.is_empty() && detail {
        let mut t = plain_table();
        for f in &shell_functions {
            t.add_row([
                f.name.as_str(),
                f.description.as_str(),
                &format!("{}", "catalog".dimmed()),
            ]);
        }
        sections.push(("Shell Functions".to_string(), t));
    }

    if !symlinks.is_empty() {
        let mut t = plain_table();
        let mut totals = Totals::default();
        for s in &symlinks {
            let linked = crate::catalog::expand_tilde(&s.destination)
                .join(&s.name)
                .exists();
            totals.add(linked);
            if detail || !linked {
                let status = if linked {
                    format!("{}", "✅ linked".green())
                } else {
                    format!("{}", "❌ missing".red())
                };
                let target = format!("{} → {}", s.name, s.target);
                t.add_row([s.name.as_str(), target.as_str(), status.as_str()]);
            }
        }
        if detail || totals.missing > 0 {
            sections.push(("Symlinks".to_string(), t));
        }
        total_items += totals.total;
        installed_items += totals.installed;
        missing_items += totals.missing;
    }

    for (title, table) in sections {
        println!("{}", title.bold());
        println!("{table}\n");
    }

    if missing_items == 0 {
        println!(
            "{}",
            format!("✅ All {total_items} catalog items installed").green()
        );
    } else {
        println!(
            "{}",
            format!(
                "⚠️  {missing_items} of {total_items} catalog items missing ({installed_items} installed)"
            )
            .yellow()
        );
    }

    Ok(())
}

#[derive(Default)]
struct Totals {
    total: usize,
    installed: usize,
    missing: usize,
}

impl Totals {
    fn add(&mut self, ok: bool) {
        self.total += 1;
        if ok {
            self.installed += 1;
        } else {
            self.missing += 1;
        }
    }
}

fn build_brew_section(
    title: &str,
    entries: &[&crate::catalog::BrewEntry],
    detail: bool,
    installed_casks: &HashSet<String>,
) -> (String, Table, Totals) {
    let mut t = plain_table();
    let mut totals = Totals::default();
    for e in entries {
        let check_name = e.bin.as_deref().unwrap_or(&e.name);
        let compat = arch_compatible(&e.arch);
        let ok = if compat {
            let installed = check_binary(check_name)
                || (e.kind == BrewKind::Cask && installed_casks.contains(&e.name));
            totals.add(installed);
            installed
        } else {
            totals.total += 1;
            true // treat skipped as not a problem
        };
        if detail || !ok {
            let status = if compat {
                yn(ok)
            } else {
                format!(
                    "{}",
                    format!("skipped ({})", e.arch.as_deref().unwrap_or("unknown")).dimmed()
                )
            };
            let desc = desc_with_plugin(&e.description, &e.zsh_plugin);
            t.add_row([&e.name, &desc, &status]);
        }
    }
    (title.to_string(), t, totals)
}

fn desc_with_plugin(description: &str, zsh_plugin: &Option<String>) -> String {
    match zsh_plugin {
        Some(p) => format!("{} {}", description, format!("[plugin: {p}]").dimmed()),
        None => description.to_string(),
    }
}

fn plain_table() -> Table {
    let mut t = Table::new();
    t.set_header(["Name", "Description", "Status"]);
    t.set_width(100);
    t
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
