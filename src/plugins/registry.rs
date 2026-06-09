//! Static list of built-in plugins. Plugins are appended in Phase 2.
use crate::plugins::Plugin;

pub fn built_ins() -> Vec<Box<dyn Plugin>> {
    vec![
        Box::new(crate::plugins::apm::Apm::new()),
        Box::new(crate::plugins::brew::Brew::new()),
        Box::new(crate::plugins::claude::Claude::new()),
        Box::new(crate::plugins::custom::Custom::new()),
        Box::new(crate::plugins::dotfiles::Dotfiles::new()),
        Box::new(crate::plugins::permissions::Permissions::new()),
        Box::new(crate::plugins::runtimes::Runtimes::new()),
        Box::new(crate::plugins::shell::Shell::new()),
        Box::new(crate::plugins::symlinks::Symlinks::new()),
    ]
}
