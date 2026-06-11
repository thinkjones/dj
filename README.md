# dj

A single Rust binary that manages a macOS developer workstation. Install tools, apply dotfiles, inject shell helpers, and stamp project directories with your preferred agentic stack.

No Node. No Python. No interpreter on the target machine.

---

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/thinkjones/dj/main/bootstrap.sh | bash
```

The bootstrap script downloads the latest `dj` binary for your architecture, installs it to `~/.local/bin/dj`, and runs `dj onboard` to set up your catalog.

**Prerequisites:** macOS, Homebrew, `gh` (GitHub CLI) authenticated.

---

## Quick start

```bash
dj onboard                    # Set up your catalog (example, GitHub repo, or local path)
dj doctor                     # Health check everything
dj setup run --user           # Full machine setup
dj setup run --user --dry-run # Preview what would run
dj plugins                    # See installed plugins and catalog actions
```

---

## Commands

Plugins and workflows share the same interface:

```text
dj <name> info                # Default when bare name is given
dj <name> version
dj <name> doctor [--user|--folder [path]]
dj <name> list  [--user|--folder [path]]
dj <name> run   [--user|--folder [path]] [args…] [--dry-run]
```

Tool management commands:

```text
dj rebuild                    # Regenerate all artifacts from catalog
dj reinstall                  # Re-download dj from latest release
dj uninstall                  # Remove the dj binary
dj completions install        # Install shell completions
dj plugins                    # Show all installed plugins and catalog actions
dj doctor                     # Health check across all plugins
dj version                    # Show dj + plugin versions
dj onboard                    # First-time catalog setup
```

---

## The Catalog

The catalog is a plain directory of markdown files at `~/.config/dj/catalog/`. It is the source of truth for everything `dj` manages.

```text
~/.config/dj/catalog/
  dj-catalog.toml        # Plugin version pins
  workflows.md           # Named workflows
  brew/config.md         # Homebrew formulae, casks, and taps
  runtimes/config.md     # mise runtimes + package managers
  scripts/               # Custom install scripts (one *.md file per purpose)
  shell/config.md        # zsh helper functions
  symlinks/config.md     # Symlink definitions
  claude/user.md         # Claude Code user settings
  claude/project.md      # Claude Code project settings
  apm/<stack>/apm.yml    # Agentic stack manifests
  permissions/template.json  # AI permission rules
  chezmoi/               # Dotfiles source tree
```

After editing the catalog, run `dj rebuild` to regenerate the Brewfile and `.zshrc` sentinel block.

---

## Configuration

`~/.config/dj/config.toml`:

```toml
catalog_root = "~/.config/dj/catalog"
default_agent_stack = "core"
```

Override the config location with `DJ_CONFIG_ROOT`.

---

## Development

```bash
git clone git@github.com:thinkjones/dj.git
cd dj

# Optional: enable pre-commit hooks
git config core.hooksPath .githooks

cargo test
cargo build --release
```

---

## License

MIT
