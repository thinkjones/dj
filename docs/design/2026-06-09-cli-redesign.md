# DJ Public Alpha — CLI & Architecture Redesign

> Brainstorming design doc for separating the `dj` tool from the personal catalog and moving to a public plugin architecture.

## 1. Goals

1. **Separate concerns**: `dj` = the CLI engine + official plugins. `dj-catalog` = user configuration (can be private or example).
2. **Public alpha**: Anyone can install `dj` and get a working example catalog, or point to their own.
3. **Plugin architecture**: Current hard-coded features become *official plugins* (prefixed `official-*`, shipped with core).
4. **Versioned plugins**: Catalog declares plugin versions; `dj` installs/runs the matching plugin binary.
5. **Workflows**: Ordered arrays of plugin invocations, stored in catalog, supporting the same verbs as plugins.
6. **Improved UX**: Consistent `dj <name> (doctor | run | version | list | info)`, `--dry-run`, and clearer onboarding.

## 2. Repo Layout

```
thinkjones/dj           ← this repo (public CLI + official plugins)
thinkjones/me-dj-catalog ← private personal catalog
thinkjones/dj-alpha      ← legacy (current repo renamed)
```

Inside `thinkjones/dj`:

```
Cargo.toml
src/
  main.rs                 ← thin CLI shell
  lib.rs
  config.rs               ← dj global config (~/.config/dj/config.toml)
  cli/
    mod.rs                ← reserved names, resolution enums
    parse.rs              ← token parser (name, verb, scope, args, dry-run)
    dispatch.rs           ← route to plugin or workflow
    scope.rs              ← Scope / ScopeKind
    verb.rs               ← Verb enum + resolution rules
    workflow.rs           ← workflow expand + conflict resolution
  plugins/
    mod.rs                ← Plugin trait, Manifest, Health, PlanStep, Cadence
    runtime.rs            ← load plugins, resolve names, execute
    registry.rs           ← built-in official plugins
    official/             ← official plugin implementations
      brew/
      runtimes/
      custom/
      dotfiles/
      symlinks/
      shell/
      claude/
      apm/
      permissions/
    workflows.rs          ← parse workflows.md
    lastrun.rs            ← last-run tracking
  commands/               ← top-level commands (self, catalog, onboard, completions)
    mod.rs
    self.rs               ← rebuild, reinstall, uninstall, completions
    catalog.rs            ← info, use, fetch, list
    onboard.rs            ← first-time setup wizard
    completions.rs
  resolver/               ← (future) remote plugin resolver from GH releases
    mod.rs
```

## 3. Catalog Layout (Example vs Personal)

A catalog is just a folder. Default location: `~/.config/dj/catalog` (or `$DJ_CATALOG_ROOT`).

```
~/.config/dj/catalog/
  dj-catalog.toml         ← catalog manifest (name, version, plugin versions)
  workflows.md            ← workflows definition
  brew/
    config.md
  runtimes/
    config.md
  custom/
    config.md
  dotfiles/
    config.md
  symlinks/
    config.md
  shell/
    config.md
  claude/
    config.md             ← or plugin.json for folder scope
  apm/
    core/
      apm.yml
  permissions/
    template.json
```

### `dj-catalog.toml`

```toml
name = "thinkjones-personal"
version = "1.0.0"

[plugins]
brew = "0.2.0"
runtimes = "0.2.0"
custom = "0.2.0"
dotfiles = "0.2.0"
symlinks = "0.2.0"
shell = "0.2.0"
claude = "0.2.0"
apm = "0.2.0"
permissions = "0.2.0"
```

In **Phase 1** (alpha), all official plugins ship inside the `dj` binary, so the version lock is mainly documentation. In **Phase 2**, the runtime will download/validate the exact plugin version.

## 4. CLI Command Structure

### 4.1 Top-level built-ins (never plugins/workflows)

```
dj --help
dj --version

dj doctor                 ← global doctor across all plugins (all scopes)
dj list                   ← global list across all plugins
dj version                ← show dj + all plugin versions
dj info                   ← show installed plugins + workflows summary

dj self rebuild
dj self reinstall -y
dj self uninstall -y
dj self completions bash|fish|zsh|install [-y]

dj catalog info           ← show current catalog source + path
dj catalog use --example  ← install example catalog
dj catalog use /path      ← use local catalog
dj catalog fetch owner/repo [--branch main]
dj catalog list           ← list catalog entries

dj onboard                ← first-time setup wizard
```

### 4.2 Plugin / Workflow Invocation

Every plugin and workflow supports the same five verbs:

```
dj <name> info            ← default if no verb given
dj <name> version
dj <name> doctor [--user|--folder [path]]
dj <name> list  [--user|--folder [path]]
dj <name> run   [--user|--folder [path]] [args…] [--dry-run] [-- -pass through]
```

Scope rules (same as today):
- `info`, `version` → no scope required.
- `doctor`, `list` → optional scope; defaults to `user` if plugin supports it.
- `run` → scope required (`--user` or `--folder [path]`).
- If a scope flag is present without an explicit verb, verb defaults to `run`.

Examples:
```bash
dj brew info
dj brew doctor --user
dj brew run --user --dry-run
dj symlinks run --folder ./my-project
dj setup run --user          # workflow
```

### 4.3 Conflict Resolution (Plugin vs Workflow)

A name can be **both** a plugin and a workflow.

| Scenario | Behavior |
|---|---|
| Non-mutating verb (`info`, `version`, `list`, `doctor`) | Run for **both** plugin and workflow with a prominent yellow warning: `"⚠ 'foo' is both a plugin and a workflow — showing both."` |
| Mutating verb (`run`) | **Error** with disambiguation instructions: `"'foo' is both a plugin and a workflow. Disambiguate: dj plugin:foo run … or dj workflow:foo run …"` |
| Prefixed name (`plugin:foo`, `workflow:foo`) | Skip resolution, go directly to the requested kind. |

This is identical to current behavior; we keep it.

## 5. Plugin Architecture

### 5.1 Official Plugins

- Prefix: `official-` (e.g., `official-brew`).
- Shipped inside the `dj` binary.
- Installed automatically when `dj` is installed.
- In the CLI, users type the short name (`brew`, not `official-brew`). The runtime maps short names → official plugins.

### 5.2 Plugin Trait (unchanged core)

```rust
pub trait Plugin {
    fn manifest(&self) -> &Manifest;
    fn plan(&self, ctx: &PluginContext) -> Result<Vec<PlanStep>>;
    fn run(&self, ctx: &PluginContext) -> Result<()>;
    fn doctor(&self, ctx: &PluginContext) -> Result<Health>;
    fn list(&self, ctx: &PluginContext) -> Result<Vec<String>>;
    fn example_config(&self, scope: ScopeKind) -> Option<String>;
}
```

Each plugin reads its config from `<catalog_root>/<name>/<config_file>`.

### 5.3 Plugin Manifest (per plugin `plugin.toml`)

```toml
name = "brew"
summary = "Homebrew package management"
version = "0.2.0"
scopes = ["user"]
cadence = "regular"

[config]
user = "config.md"
```

### 5.4 Runtime Loading

1. Read `dj-catalog.toml` from catalog root.
2. Load all **official** plugins from the built-in registry.
3. (Future) Load external plugins from `~/.local/share/dj/plugins/`.
4. Parse `workflows.md`.
5. Validate: no workflow cycles; every workflow step resolves to a known plugin/workflow.

## 6. Workflows

Stored in `workflows.md` (same format as today):

```markdown
# Workflows
## setup
### user
- brew
- runtimes
- custom
- dotfiles
- symlinks
- shell
- dev-setup
### folder
- dev-setup
## dev-setup
### user
- claude
### folder
- claude
- apm core
- permissions
```

Execution:
- `dj setup run --user` → expands `setup.user` steps recursively → runs each plugin with the given scope.
- `--dry-run` prints the expanded list without executing.
- Workflows themselves do not have versions; they are catalog-level configuration.

## 7. Onboarding (First-Time Install)

If `dj` cannot find a catalog at `~/.config/dj/catalog`:

1. Print welcome message.
2. Prompt:
   - **(a)** Install the **example catalog** (ships a minimal `dj-catalog.toml` + configs inside the binary, extracted to disk).
   - **(b)** **Fetch** a catalog from a GitHub repo (`owner/repo`).
   - **(c)** **Use** a local path.
3. Write `~/.config/dj/config.toml` with `catalog_root` pointing to the chosen location.
4. Run `dj info` to confirm.

Example prompt:
```
No dj catalog found.

How would you like to set up your catalog?
  [1] Install the beginner example catalog
  [2] Fetch from a GitHub repo (e.g., thinkjones/me-dj-catalog)
  [3] Use a local folder
  [q] Quit
> 1
```

## 8. Global Commands Details

### `dj doctor`

Runs `doctor` on every plugin for every scope it supports. Prints a summary table.

```
brew@user       — Ok
runtimes@user   — Missing   Node 20 not installed
custom@user     — Ok
```

### `dj list`

Runs `list` on every plugin (using a default scope). Prints grouped output.

### `dj version`

```
dj 0.3.0
brew 0.2.0
runtimes 0.2.0
...
```

### `dj info`

```
Plugins:
  brew         Homebrew package management
  runtimes     Language runtime management
  ...
Workflows:
  setup
  dev-setup
```

## 9. Backwards Compatibility / Port Plan

| Current | New | Notes |
|---|---|---|
| `dj doctor` | same | Global doctor |
| `dj list` | same | Global list |
| `dj version` | same | Shows dj + plugin versions |
| `dj info` | same | Shows plugins + workflows |
| `dj self *` | same | Self-management |
| `dj catalog *` | same | Catalog management |
| `dj onboard` | same | First-time wizard |
| `dj <plugin>` | `dj <plugin> info` | Bare name now routes through dispatch (same as today) |
| `dj <plugin> run --user` | same | Explicit verb + scope |
| `dj <workflow>` | `dj <workflow> info` | Same dispatch logic |
| `dj <workflow> run --user` | same | Expands and runs steps |
| `--dry-run` | same | Works for plugin `run` and workflow `run` |

The parser (`cli/parse.rs`) and dispatch (`cli/dispatch.rs`) already implement most of this. We port them almost verbatim.

## 10. Open Questions

1. **External plugins (Phase 2)**: Should they be WASM modules, separate binaries, or shared libs? For alpha, we only need official built-ins.
2. **Plugin versioning in alpha**: Since all plugins are built-in, the catalog's version pins are advisory. Should `dj` warn if catalog pins differ from built-in versions?
3. **Example catalog bundling**: Should the example catalog be embedded in the binary (e.g., via `include_str!`) or fetched from a GitHub release asset?
4. **Rename `me-dj-catalog` → private**: Do we need a migration script for the personal catalog, or just a manual `git remote set-url`?

## 11. Implementation Order

1. Scaffold new `dj` repo (Cargo.toml, directory structure).
2. Port core modules: `config`, `cli/*`, `plugins/mod.rs`, `plugins/runtime.rs`, `plugins/registry.rs`, `plugins/workflows.rs`, `plugins/lastrun.rs`.
3. Port all official plugin implementations into `src/plugins/official/*/mod.rs`.
4. Port commands: `self`, `catalog`, `onboard`, `completions`.
5. Wire `main.rs` with the new CLI structure.
6. Add embedded example catalog + extraction logic for `dj catalog use --example`.
7. Tests + CI.

---

*Ready for review.*
