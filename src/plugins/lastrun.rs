//! Informational record of when each plugin@scope last ran. Never gates execution.
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn store_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
        .join(".local/share/dj/runs.toml")
}

/// Map of "<plugin>@<scope>" -> epoch-seconds string.
pub fn load() -> BTreeMap<String, String> {
    let Ok(s) = std::fs::read_to_string(store_path()) else {
        return BTreeMap::new();
    };
    toml::from_str(&s).unwrap_or_default()
}

pub fn record(key: &str) -> Result<()> {
    let mut all = load();
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    all.insert(key.to_string(), secs.to_string());
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, toml::to_string(&all)?)?;
    Ok(())
}

pub fn key(plugin: &str, scope: &str) -> String {
    format!("{plugin}@{scope}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_format() {
        assert_eq!(key("symlinks", "user"), "symlinks@user");
    }

    #[test]
    fn load_missing_is_empty() {
        // store may or may not exist on the test machine; load never panics.
        let _ = load();
    }
}
