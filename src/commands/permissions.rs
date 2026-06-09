use anyhow::{bail, Result};
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table};
use owo_colors::OwoColorize;
use similar::TextDiff;
use std::io::{self, Write};
use std::path::Path;

use crate::{
    config::{catalog_root, Config},
    permissions::{
        template::{read_template, template_path, template_sha256},
        targets::{
            all_targets, global_destination, has_project_scope, is_global_installed,
            project_destination, project_marker_exists, project_marker_path, resolve_target,
            TargetId,
        },
        translators::{antigravity, claude, codex, gemini, kimicode, opencode, TranslationResult},
        TemplatePermissions,
    },
};

#[allow(dead_code)]
pub enum PermissionsAction {
    // TargetsList and Diff are retained for the plugin-runtime sub-project;
    // not reachable via the current CLI (only Sync is used via the plugin).
    TargetsList,
    Sync { target: String, global: bool, yes: bool },
    Diff { target: String, global: bool },
}

pub fn run(cfg: &Config, action: PermissionsAction) -> Result<()> {
    match action {
        PermissionsAction::TargetsList => cmd_targets_list(cfg),
        PermissionsAction::Sync { target, global, yes } => cmd_sync(cfg, &target, global, yes),
        PermissionsAction::Diff { target, global } => cmd_diff(cfg, &target, global),
    }
}

// ── targets list ──────────────────────────────────────────────────────────────

fn cmd_targets_list(cfg: &Config) -> Result<()> {
    let tpl_path = template_path(&catalog_root(cfg));
    if tpl_path.exists() {
        let sha = template_sha256(&tpl_path).unwrap_or_else(|_| "error".to_string());
        println!(
            "TEMPLATE  {}  {} (sha256: {})",
            tpl_path.display(),
            "✓ present".green(),
            &sha[..16]
        );
    } else {
        println!(
            "TEMPLATE  {}  {}",
            tpl_path.display(),
            "✗ missing".red()
        );
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(["TARGET", "SUPPORT", "GLOBAL", "PROJECT", "NOTES"]);

    for t in all_targets() {
        let global_check = if is_global_installed(t.id) {
            "✓".green().to_string()
        } else {
            "✗".dimmed().to_string()
        };
        let project_check = if !has_project_scope(t.id) {
            "n/a".dimmed().to_string()
        } else if project_marker_exists(t.id, &cwd) {
            "✓".green().to_string()
        } else {
            "✗".dimmed().to_string()
        };
        table.add_row([t.name, &t.support.to_string(), &global_check, &project_check, t.notes]);
    }
    println!("{table}");
    Ok(())
}

// ── sync ──────────────────────────────────────────────────────────────────────

fn cmd_sync(cfg: &Config, target_str: &str, global: bool, yes: bool) -> Result<()> {
    let tpl_path = template_path(&catalog_root(cfg));
    if !tpl_path.exists() {
        bail!("template not found at {}; create it first", tpl_path.display());
    }
    let perms = read_template(&tpl_path)?;

    let targets = resolve_targets(target_str)?;
    let cwd = std::env::current_dir().unwrap_or_default();

    let mut any_error = false;
    for id in &targets {
        match sync_one(cfg, *id, &perms, global, yes, &cwd) {
            Ok(summary) => println!("{}: {}", target_name(*id), summary),
            Err(e) => {
                eprintln!("{}: error ({})", target_name(*id), e);
                if targets.len() == 1 {
                    return Err(e);
                }
                any_error = true;
            }
        }
    }
    if any_error && targets.len() > 1 {
        bail!("one or more targets failed");
    }
    Ok(())
}

fn sync_one(
    cfg: &Config,
    id: TargetId,
    perms: &TemplatePermissions,
    global: bool,
    yes: bool,
    cwd: &Path,
) -> Result<String> {
    let result = translate_target(id, perms, global, cwd)?;
    for warning in &result.warnings {
        eprintln!("warning: {}", warning);
    }

    let mut all_in_sync = true;
    for f in &result.files {
        let existing = if f.path.exists() {
            std::fs::read_to_string(&f.path).unwrap_or_default()
        } else {
            String::new()
        };
        if existing.trim() != f.content.trim() {
            all_in_sync = false;
            break;
        }
    }

    if all_in_sync {
        return Ok("in sync".to_string());
    }

    // Show diffs
    for f in &result.files {
        let existing = if f.path.exists() {
            std::fs::read_to_string(&f.path).unwrap_or_default()
        } else {
            String::new()
        };
        if existing.trim() == f.content.trim() {
            continue;
        }
        println!("--- {}", f.path.display());
        println!("+++ {}", f.path.display());
        let diff = TextDiff::from_lines(&existing, &f.content);
        for change in diff.iter_all_changes() {
            match change.tag() {
                similar::ChangeTag::Delete => print!("{}", format!("- {}", change).red()),
                similar::ChangeTag::Insert => print!("{}", format!("+ {}", change).green()),
                similar::ChangeTag::Equal => print!("  {}", change),
            }
        }
    }

    if !yes {
        print!("Apply changes for {}? [y/N] ", target_name(id));
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            return Ok("skipped (declined)".to_string());
        }
    }

    // Write files
    for f in &result.files {
        if let Some(parent) = f.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&f.path, &f.content)?;
    }

    // Write sidecar checksum for JSON targets
    write_checksum(cfg, id, global, perms)?;

    Ok("synced".to_string())
}

// ── diff ──────────────────────────────────────────────────────────────────────

fn cmd_diff(cfg: &Config, target_str: &str, global: bool) -> Result<()> {
    let tpl_path = template_path(&catalog_root(cfg));
    if !tpl_path.exists() {
        bail!("template not found at {}; create it first", tpl_path.display());
    }
    let perms = read_template(&tpl_path)?;

    let targets = resolve_targets(target_str)?;
    let cwd = std::env::current_dir().unwrap_or_default();

    for id in &targets {
        match diff_one(*id, &perms, global, &cwd) {
            Ok(()) => {}
            Err(e) => eprintln!("{}: error ({})", target_name(*id), e),
        }
    }
    Ok(())
}

fn diff_one(id: TargetId, perms: &TemplatePermissions, global: bool, cwd: &Path) -> Result<()> {
    let result = translate_target(id, perms, global, cwd)?;
    for warning in &result.warnings {
        eprintln!("warning: {}", warning);
    }

    let mut all_in_sync = true;
    for f in &result.files {
        let existing = if f.path.exists() {
            std::fs::read_to_string(&f.path).unwrap_or_default()
        } else {
            String::new()
        };
        if existing.trim() != f.content.trim() {
            all_in_sync = false;
            println!("--- {}", f.path.display());
            println!("+++ {}", f.path.display());
            let diff = TextDiff::from_lines(&existing, &f.content);
            for change in diff.iter_all_changes() {
                match change.tag() {
                    similar::ChangeTag::Delete => print!("{}", format!("- {}", change).red()),
                    similar::ChangeTag::Insert => print!("{}", format!("+ {}", change).green()),
                    similar::ChangeTag::Equal => print!("  {}", change),
                }
            }
        }
    }
    if all_in_sync {
        println!("{}: in sync", target_name(id));
    }
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn target_name(id: TargetId) -> &'static str {
    all_targets()
        .into_iter()
        .find(|t| t.id == id)
        .map(|t| t.name)
        .unwrap_or("unknown")
}

fn resolve_targets(target_str: &str) -> Result<Vec<TargetId>> {
    if target_str == "all" {
        return Ok(all_targets().into_iter().map(|t| t.id).collect());
    }
    match resolve_target(target_str) {
        Some(id) => Ok(vec![id]),
        None => bail!(
            "unknown target '{}'; run `dj ai permissions targets list` to see valid targets",
            target_str
        ),
    }
}

fn translate_target(
    id: TargetId,
    perms: &TemplatePermissions,
    global: bool,
    cwd: &Path,
) -> Result<TranslationResult> {
    // Scope validation
    if !global && !has_project_scope(id) {
        bail!(
            "{} does not support project scope; rerun with --global or pick a different target",
            target_name(id)
        );
    }
    if !global && !project_marker_exists(id, cwd) {
        bail!(
            "{} not initialized in this project; expected {}. Initialize the agent first or rerun with --global",
            target_name(id),
            project_marker_path(id, cwd)
        );
    }

    let primary_dest = if global {
        let dest = global_destination(id)
            .ok_or_else(|| anyhow::anyhow!("{}: cannot determine global destination", target_name(id)))?;
        let parent = dest.parent().unwrap_or(Path::new("/")).to_path_buf();
        if !parent.exists() {
            return Err(anyhow::anyhow!(
                "skipped (global destination parent {} does not exist; {} may not be installed)",
                parent.display(),
                target_name(id)
            ));
        }
        dest
    } else {
        project_destination(id, cwd).unwrap()
    };

    match id {
        TargetId::Claude => claude::translate(perms, &primary_dest),
        TargetId::Gemini => {
            let scope_root = primary_dest.parent().unwrap().to_path_buf();
            gemini::translate(perms, &scope_root, global)
        }
        TargetId::Antigravity => antigravity::translate(perms, &primary_dest, false),
        TargetId::AntigravityCli => antigravity::translate(perms, &primary_dest, true),
        TargetId::OpenCode => opencode::translate(perms, &primary_dest),
        TargetId::Codex => {
            let scope_root = primary_dest.parent().unwrap().to_path_buf();
            codex::translate(perms, &scope_root)
        }
        TargetId::KimiCode => kimicode::translate(perms, &primary_dest),
    }
}

fn write_checksum(cfg: &Config, id: TargetId, global: bool, _perms: &TemplatePermissions) -> Result<()> {
    let tpl_path = template_path(&catalog_root(cfg));
    if !tpl_path.exists() {
        return Ok(());
    }
    let sha = template_sha256(&tpl_path).unwrap_or_default();
    let scope_str = if global { "global" } else { "project" };
    let checksum_dir = crate::config::dj_config_dir()
        .join("state")
        .join("permissions");
    std::fs::create_dir_all(&checksum_dir)?;
    let checksum_path = checksum_dir.join(format!("{}-{}.checksum", target_name(id), scope_str));
    std::fs::write(checksum_path, sha)?;
    Ok(())
}
