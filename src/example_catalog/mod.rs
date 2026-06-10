//! Embedded example catalog for `dj catalog use --example` when the binary
//! is not running from the source tree.
#![allow(dead_code)]
use anyhow::Result;
use std::path::Path;

struct EmbeddedFile {
    path: &'static str,
    content: &'static str,
}

const FILES: &[EmbeddedFile] = &[
    EmbeddedFile {
        path: "dj-catalog.toml",
        content: include_str!("../../examples/catalog/dj-catalog.toml"),
    },
    EmbeddedFile {
        path: "workflows/setup.md",
        content: include_str!("../../examples/catalog/workflows/setup.md"),
    },
    EmbeddedFile {
        path: "workflows/dev-setup.md",
        content: include_str!("../../examples/catalog/workflows/dev-setup.md"),
    },
    EmbeddedFile {
        path: "brew/config.md",
        content: include_str!("../../examples/catalog/brew/config.md"),
    },
    EmbeddedFile {
        path: "runtimes/config.md",
        content: include_str!("../../examples/catalog/runtimes/config.md"),
    },
    EmbeddedFile {
        path: "custom/config.md",
        content: include_str!("../../examples/catalog/custom/config.md"),
    },
    EmbeddedFile {
        path: "shell/config.md",
        content: include_str!("../../examples/catalog/shell/config.md"),
    },
    EmbeddedFile {
        path: "symlinks/config.md",
        content: include_str!("../../examples/catalog/symlinks/config.md"),
    },
    EmbeddedFile {
        path: "dotfiles/config.md",
        content: include_str!("../../examples/catalog/dotfiles/config.md"),
    },
    EmbeddedFile {
        path: "claude/user.md",
        content: include_str!("../../examples/catalog/claude/user.md"),
    },
    EmbeddedFile {
        path: "permissions/template.json",
        content: include_str!("../../examples/catalog/permissions/template.json"),
    },
];

pub fn extract(dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for file in FILES {
        let path = dest.join(file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, file.content)?;
    }
    Ok(())
}
