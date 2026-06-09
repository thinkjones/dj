use anyhow::{bail, Result};
use dirs::home_dir;
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{
    catalog::ai::{parse_apm_stack, parse_claude_settings},
    config::{catalog_root, Config},
};

pub fn run(cfg: &Config, name: &str, scope: &str, path: Option<&str>, yes: bool) -> Result<()> {
    let root = catalog_root(cfg);
    let stack_dir = root.join(name);

    if !stack_dir.exists() {
        bail!("ai stack '{}' not found in catalog ({})", name, stack_dir.display());
    }

    // Dispatch: claude settings
    if name == "claude" {
        apply_claude_settings(&stack_dir, scope, path, yes)?;
    }

    // Dispatch: apm.yml
    if let Some(stack) = parse_apm_stack(name, &stack_dir)? {
        apply_apm(&stack.apm_yml_content, &stack_dir, scope, path, yes)?;
    }

    Ok(())
}

pub fn run_apm(cfg: &Config, stack: &str, scope: &str, path: Option<&str>, yes: bool) -> Result<()> {
    let root = catalog_root(cfg);
    let stack_dir = root.join("apm").join(stack);

    if !stack_dir.exists() {
        bail!("ai apm stack '{}' not found in catalog ({})", stack, stack_dir.display());
    }

    if let Some(apm_stack) = parse_apm_stack(stack, &stack_dir)? {
        apply_apm(&apm_stack.apm_yml_content, &stack_dir, scope, path, yes)?;
    }

    Ok(())
}

fn apply_claude_settings(stack_dir: &Path, scope: &str, path: Option<&str>, yes: bool) -> Result<()> {
    let md_file = match scope {
        "user" => stack_dir.join("user.md"),
        _ => stack_dir.join("project.md"),
    };

    if !md_file.exists() {
        return Ok(());
    }

    let settings = parse_claude_settings(&md_file)?;
    let dest = resolve_settings_path(&settings.settings_path, scope, path)?;

    println!("Writing Claude settings to {}", dest.display().cyan());

    if !yes {
        if dest.exists() {
            let existing = std::fs::read_to_string(&dest).unwrap_or_default();
            if existing.trim() == settings.json_body.trim() {
                println!("{}", "Already in sync.".green());
                return Ok(());
            }
            // Show diff
            let diff = similar::TextDiff::from_lines(&existing, &settings.json_body);
            for change in diff.iter_all_changes() {
                match change.tag() {
                    similar::ChangeTag::Delete => print!("{}", format!("- {}", change).red()),
                    similar::ChangeTag::Insert => print!("{}", format!("+ {}", change).green()),
                    similar::ChangeTag::Equal => print!("  {}", change),
                }
            }
        }

        print!("Apply? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&dest, &settings.json_body)?;
    println!("{}", "✓ Settings written.".green());
    Ok(())
}

fn resolve_settings_path(settings_path: &str, scope: &str, path: Option<&str>) -> Result<PathBuf> {
    if scope == "user" {
        let raw = settings_path.replace('~', &home_dir().unwrap_or_default().to_string_lossy());
        return Ok(PathBuf::from(raw));
    }

    // Project scope
    if path.is_some() && scope == "user" {
        bail!("--path is not allowed with --scope=user");
    }

    let base = path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    Ok(base.join(settings_path))
}

pub fn run_setup_project(cfg: &Config, path: Option<&str>, yes: bool) -> Result<()> {
    use crate::permissions::{
        targets::{has_project_scope, project_destination, project_marker_exists, TargetId},
        template::{read_template, template_path},
        translators::claude as claude_translator,
    };

    let root = catalog_root(cfg);
    let claude_dir = root.join("claude");

    // Apply Claude project settings
    apply_claude_settings(&claude_dir, "project", path, yes)?;

    // Sync permissions for claude target if already initialized
    let cwd = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let tpl_path = template_path(&root);
    if tpl_path.exists() && has_project_scope(TargetId::Claude) {
        if let Some(dest) = project_destination(TargetId::Claude, &cwd) {
            if project_marker_exists(TargetId::Claude, &cwd) {
                let perms = read_template(&tpl_path)?;
                let result = claude_translator::translate(&perms, &dest)?;
                for f in &result.files {
                    if let Some(parent) = f.path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&f.path, &f.content)?;
                }
                println!("{}", "✓ Permissions synced.".green());
            }
        }
    }

    Ok(())
}

fn apply_apm(apm_yml: &str, stack_dir: &Path, scope: &str, path: Option<&str>, yes: bool) -> Result<()> {
    if !yes {
        println!("APM manifest:\n{}", apm_yml.cyan());
        print!("Run apm install? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let abs_stack_dir = stack_dir.canonicalize()?;

    if scope == "user" {
        // Global mode: continue as normal
        let mut cmd = Command::new("apm");
        cmd.arg("install").arg("-g").arg(&abs_stack_dir);

        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap_or("")).collect();
        println!("  {} apm {}", "→".cyan(), args.join(" "));

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        print!("{}", stdout);
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }
        if !output.status.success() || stdout.contains("Install interrupted") || stdout.contains("failed validation") {
            bail!("apm install failed");
        }
    } else {
        // Folder mode: run in temp dir, then copy back
        let tmp_dir = apm_tmp_dir()?;
        std::fs::create_dir_all(&tmp_dir)?;

        let dest_dir = path
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let mut cmd = Command::new("apm");
        cmd.arg("install").arg(&abs_stack_dir).arg("-t").arg("claude");
        cmd.current_dir(&tmp_dir);

        let args: Vec<&str> = cmd.get_args().map(|a| a.to_str().unwrap_or("")).collect();
        println!("  {} apm {} (in {})", "→".cyan(), args.join(" "), tmp_dir.display());

        let output = cmd.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        print!("{}", stdout);
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }
        if !output.status.success() || stdout.contains("Install interrupted") || stdout.contains("failed validation") {
            bail!("apm install failed");
        }

        // Copy files from tmp_dir to dest_dir, skipping APM metadata
        copy_apm_output(&tmp_dir, &dest_dir)?;
    }

    println!("{}", "✓ APM stack applied.".green());
    Ok(())
}

fn apm_tmp_dir() -> Result<PathBuf> {
    let output = Command::new("date").arg("+%Y-%m-%d-%H-%M-%S").output()?;
    let timestamp = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(std::env::temp_dir().join(format!("apm-{}", timestamp)))
}

fn copy_apm_output(src: &Path, dest: &Path) -> Result<()> {
    const SKIP: &[&str] = &["apm.yml", "apm_modules", "apmlock.yml", "apm.lock.yml"];

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if SKIP.iter().any(|s| *s == name_str.as_ref()) {
            continue;
        }

        copy_recursively(&entry.path(), &dest.join(&name))?;
    }
    Ok(())
}

fn copy_recursively(src: &Path, dest: &Path) -> Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursively(&entry.path(), &dest.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dest)?;
    }
    Ok(())
}
