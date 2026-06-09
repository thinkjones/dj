use anyhow::{bail, Result};
use clap_complete::{generate, Shell};
use std::io;

pub fn print_completions(shell: Shell, cmd: &mut clap::Command) {
    generate(shell, cmd, "dj", &mut io::stdout());
}

pub fn install(yes: bool, cmd: &mut clap::Command) -> Result<()> {
    let shell_path = std::env::var("SHELL").unwrap_or_default();
    let shell_name = shell_path.rsplit('/').next().unwrap_or("").to_string();

    let (shell, dest, hint) = match shell_name.as_str() {
        "zsh" => (
            Shell::Zsh,
            dirs::home_dir().unwrap_or_default().join(".zfunc").join("_dj"),
            None,
        ),
        "bash" => (
            Shell::Bash,
            dirs::home_dir()
                .unwrap_or_default()
                .join(".bash_completion.d")
                .join("dj"),
            Some("source ~/.bash_completion.d/dj"),
        ),
        "fish" => (
            Shell::Fish,
            dirs::home_dir()
                .unwrap_or_default()
                .join(".config/fish/completions/dj.fish"),
            None,
        ),
        other => bail!(
            "unsupported shell '{}' — use `dj completions <bash|fish|zsh>` to print manually",
            other
        ),
    };

    println!("Installing {} completions → {}", shell_name, dest.display());

    if !yes {
        use std::io::Write as _;
        print!("Continue? [y/N] ");
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

    let mut file = std::fs::File::create(&dest)?;
    generate(shell, cmd, "dj", &mut file);

    println!("✅ Completions installed to {}", dest.display());
    if let Some(hint) = hint {
        println!("   To activate, add to your shell rc: {}", hint);
    } else {
        println!("   Restart your shell or open a new tab to activate.");
    }

    Ok(())
}
