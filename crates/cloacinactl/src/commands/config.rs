/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Configuration file support for cloacinactl.
//!
//! Reads and writes `~/.cloacina/config.toml`. Provides:
//! - `DaemonConfig` for daemon-specific settings
//! - `config get/set/list` CLI subcommands
//! - Config value lookup for commands that need database_url etc.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Full configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CloacinaConfig {
    /// Database URL for commands that need it (admin, serve).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_url: Option<String>,

    /// Daemon-specific settings.
    pub daemon: DaemonSection,

    /// Watch directory settings.
    pub watch: WatchSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonSection {
    /// Cron scheduler poll interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,
    /// Graceful shutdown timeout in seconds.
    pub shutdown_timeout_s: u64,
    /// Filesystem watcher debounce interval in milliseconds.
    pub watcher_debounce_ms: u64,
    /// Trigger scheduler base poll interval in milliseconds.
    pub trigger_poll_interval_ms: u64,
    /// Maximum cron catchup executions (None = unlimited/run_all).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_max_catchup: Option<u64>,
    /// Cron recovery check interval in seconds.
    pub cron_recovery_interval_s: u64,
    /// Cron lost task threshold in minutes.
    pub cron_lost_threshold_min: u64,
}

impl Default for DaemonSection {
    fn default() -> Self {
        Self {
            poll_interval_ms: 500,
            log_level: "info".to_string(),
            shutdown_timeout_s: 30,
            watcher_debounce_ms: 500,
            trigger_poll_interval_ms: 1000,
            cron_max_catchup: None, // None = unlimited (run_all)
            cron_recovery_interval_s: 300,
            cron_lost_threshold_min: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WatchSection {
    pub directories: Vec<String>,
}

impl CloacinaConfig {
    /// Load config from a TOML file. Returns default config if file doesn't exist.
    /// Logs and returns default on parse errors (never crashes).
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<CloacinaConfig>(&contents) {
                Ok(config) => {
                    info!("Loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    warn!(
                        "Failed to parse config file {}: {} — using defaults",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                warn!(
                    "Failed to read config file {}: {} — using defaults",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Save config to a TOML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }
        let contents = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(path, contents)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    /// Resolve watch directories from config, expanding `~` to home dir.
    pub fn resolve_watch_dirs(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        self.watch
            .directories
            .iter()
            .map(|d| {
                if let Some(stripped) = d.strip_prefix("~/") {
                    home.join(stripped)
                } else {
                    PathBuf::from(d)
                }
            })
            .collect()
    }

    /// Get a config value by dotted key path (e.g., "daemon.poll_interval_ms").
    pub fn get(&self, key: &str) -> Option<String> {
        // Serialize to toml::Value for dynamic lookup
        let value = toml::Value::try_from(self).ok()?;
        resolve_key(&value, key).map(format_value)
    }

    /// Set a config value by dotted key path.
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        // Serialize to toml::Value, update, deserialize back
        let mut toml_value =
            toml::Value::try_from(&*self).context("Failed to serialize current config")?;

        set_key(&mut toml_value, key, value)?;

        *self = toml_value
            .try_into()
            .context("Failed to apply config change — invalid value for this key")?;

        Ok(())
    }

    /// List all config key-value pairs.
    pub fn list(&self) -> Vec<(String, String)> {
        let value = match toml::Value::try_from(self) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let mut pairs = Vec::new();
        collect_pairs(&value, "", &mut pairs);
        pairs
    }
}

/// Resolve a dotted key path in a TOML value tree.
fn resolve_key<'a>(value: &'a toml::Value, key: &str) -> Option<&'a toml::Value> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = value;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

/// Set a value at a dotted key path in a TOML value tree.
fn set_key(root: &mut toml::Value, key: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = root;

    // Navigate to parent
    for part in &parts[..parts.len() - 1] {
        current = current
            .get_mut(part)
            .with_context(|| format!("Config section '{}' not found", part))?;
    }

    let leaf = parts.last().unwrap();

    // Try to preserve type: if the existing value is an integer, parse as integer, etc.
    let existing = current.get(*leaf);
    let new_value = match existing {
        Some(toml::Value::Integer(_)) => {
            let n: i64 = value
                .parse()
                .with_context(|| format!("'{}' expects an integer, got '{}'", key, value))?;
            toml::Value::Integer(n)
        }
        Some(toml::Value::Boolean(_)) => {
            let b: bool = value.parse().with_context(|| {
                format!("'{}' expects a boolean (true/false), got '{}'", key, value)
            })?;
            toml::Value::Boolean(b)
        }
        Some(toml::Value::Array(_)) => {
            // Comma-separated values for arrays, or single value
            let items: Vec<toml::Value> = value
                .split(',')
                .map(|s| toml::Value::String(s.trim().to_string()))
                .collect();
            toml::Value::Array(items)
        }
        _ => toml::Value::String(value.to_string()),
    };

    if let Some(table) = current.as_table_mut() {
        table.insert(leaf.to_string(), new_value);
    } else {
        anyhow::bail!("Cannot set value on non-table config section");
    }

    Ok(())
}

/// Collect all leaf key-value pairs with dotted paths.
fn collect_pairs(value: &toml::Value, prefix: &str, pairs: &mut Vec<(String, String)>) {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                collect_pairs(v, &key, pairs);
            }
        }
        _ => {
            pairs.push((prefix.to_string(), format_value(value)));
        }
    }
}

/// Format a TOML value for display.
fn format_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(_) => "<table>".to_string(),
        toml::Value::Datetime(dt) => dt.to_string(),
    }
}

/// Run `cloacinactl config get <key>`.
pub fn run_get(config_path: &Path, key: &str) -> Result<()> {
    let config = CloacinaConfig::load(config_path);
    match config.get(key) {
        Some(value) => {
            println!("{}", value);
            Ok(())
        }
        None => {
            anyhow::bail!("Config key '{}' not found", key);
        }
    }
}

/// Run `cloacinactl config set <key> <value>`.
pub fn run_set(config_path: &Path, key: &str, value: &str) -> Result<()> {
    let mut config = CloacinaConfig::load(config_path);
    config.set(key, value)?;
    config.save(config_path)?;
    println!("{} = {}", key, value);
    Ok(())
}

/// Run `cloacinactl config list`.
pub fn run_list(config_path: &Path) -> Result<()> {
    let config = CloacinaConfig::load(config_path);
    let pairs = config.list();
    if pairs.is_empty() {
        println!("(no configuration set)");
    } else {
        for (key, value) in pairs {
            println!("{} = {}", key, value);
        }
    }
    Ok(())
}

/// Resolve database_url from CLI arg or config file.
pub fn resolve_database_url(cli_url: Option<&str>, config_path: &Path) -> Result<String> {
    if let Some(url) = cli_url {
        return Ok(url.to_string());
    }

    let config = CloacinaConfig::load(config_path);
    config.database_url.ok_or_else(|| {
        anyhow::anyhow!(
            "Database URL is required. Either:\n  \
             - Pass --database-url <URL>\n  \
             - Set DATABASE_URL environment variable\n  \
             - Run: cloacinactl config set database_url <URL>"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn config_defaults_are_sensible() {
        let config = CloacinaConfig::default();
        assert!(config.database_url.is_none());
        assert_eq!(config.daemon.poll_interval_ms, 500);
        assert_eq!(config.daemon.log_level, "info");
        assert_eq!(config.daemon.shutdown_timeout_s, 30);
        assert_eq!(config.daemon.watcher_debounce_ms, 500);
        assert_eq!(config.daemon.trigger_poll_interval_ms, 1000);
        assert!(config.daemon.cron_max_catchup.is_none());
        assert_eq!(config.daemon.cron_recovery_interval_s, 300);
        assert_eq!(config.daemon.cron_lost_threshold_min, 10);
        assert!(config.watch.directories.is_empty());
    }

    #[test]
    fn config_load_missing_file_returns_defaults() {
        let config = CloacinaConfig::load(Path::new("/nonexistent/config.toml"));
        assert!(config.database_url.is_none());
        assert_eq!(config.daemon.poll_interval_ms, 500);
    }

    #[test]
    fn config_load_valid_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
database_url = "postgres://localhost/test"

[daemon]
poll_interval_ms = 1000
shutdown_timeout_s = 60

[watch]
directories = ["/extra/dir1", "~/workflows"]
"#,
        )
        .unwrap();

        let config = CloacinaConfig::load(&path);
        assert_eq!(
            config.database_url.as_deref(),
            Some("postgres://localhost/test")
        );
        assert_eq!(config.daemon.poll_interval_ms, 1000);
        assert_eq!(config.daemon.shutdown_timeout_s, 60);
        // Unspecified fields should still have defaults
        assert_eq!(config.daemon.log_level, "info");
        assert_eq!(config.watch.directories.len(), 2);
    }

    #[test]
    fn config_load_invalid_toml_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is { not valid toml !!!").unwrap();

        let config = CloacinaConfig::load(&path);
        // Should return defaults, not crash
        assert!(config.database_url.is_none());
        assert_eq!(config.daemon.poll_interval_ms, 500);
    }

    #[test]
    fn config_load_partial_toml_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "database_url = \"pg://localhost/db\"\n").unwrap();

        let config = CloacinaConfig::load(&path);
        assert_eq!(config.database_url.as_deref(), Some("pg://localhost/db"));
        // All daemon fields should be defaults
        assert_eq!(config.daemon.poll_interval_ms, 500);
        assert_eq!(config.daemon.shutdown_timeout_s, 30);
    }

    #[test]
    fn config_resolve_watch_dirs_expands_tilde() {
        let mut config = CloacinaConfig::default();
        config.watch.directories = vec!["~/workflows".to_string(), "/absolute/path".to_string()];

        let dirs = config.resolve_watch_dirs();
        assert_eq!(dirs.len(), 2);
        // First dir should be expanded (not start with ~)
        assert!(!dirs[0].to_str().unwrap().starts_with('~'));
        assert!(dirs[0].to_str().unwrap().ends_with("workflows"));
        // Second dir should be unchanged
        assert_eq!(dirs[1], PathBuf::from("/absolute/path"));
    }

    #[test]
    fn config_resolve_watch_dirs_empty() {
        let config = CloacinaConfig::default();
        assert!(config.resolve_watch_dirs().is_empty());
    }

    #[test]
    fn config_save_and_reload_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = CloacinaConfig {
            database_url: Some("postgres://test".to_string()),
            ..CloacinaConfig::default()
        };
        config.daemon.poll_interval_ms = 2000;
        config.watch.directories = vec!["/dir1".to_string()];
        config.save(&path).unwrap();

        let reloaded = CloacinaConfig::load(&path);
        assert_eq!(reloaded.database_url.as_deref(), Some("postgres://test"));
        assert_eq!(reloaded.daemon.poll_interval_ms, 2000);
        assert_eq!(reloaded.watch.directories, vec!["/dir1".to_string()]);
    }

    #[test]
    fn config_get_dotted_key() {
        let config = CloacinaConfig::default();
        assert_eq!(
            config.get("daemon.poll_interval_ms"),
            Some("500".to_string())
        );
        assert_eq!(config.get("daemon.log_level"), Some("info".to_string()));
        assert!(config.get("nonexistent.key").is_none());
    }

    #[test]
    fn config_set_dotted_key() {
        let mut config = CloacinaConfig::default();
        config.set("daemon.poll_interval_ms", "2000").unwrap();
        assert_eq!(config.daemon.poll_interval_ms, 2000);
    }

    #[test]
    fn config_list_returns_all_keys() {
        let config = CloacinaConfig::default();
        let pairs = config.list();
        assert!(!pairs.is_empty());
        // Should contain daemon keys
        assert!(pairs.iter().any(|(k, _)| k == "daemon.poll_interval_ms"));
        assert!(pairs.iter().any(|(k, _)| k == "daemon.log_level"));
    }
}
