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
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Full configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct CloacinaConfig {
    /// Database URL for commands that need it (admin, `server start`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_url: Option<String>,

    /// Named server-targeting profile selected by default when no `--profile`
    /// flag is supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,

    /// Named profiles keyed by name. Selected via `--profile <NAME>` or
    /// `default_profile`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, Profile>,

    /// Daemon-specific settings.
    pub daemon: DaemonSection,

    /// Compiler service settings.
    pub compiler: CompilerSection,

    /// Watch directory settings.
    pub watch: WatchSection,

    /// `cloacina-server` settings.
    pub server: ServerSection,
}

/// A named server-targeting profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Base URL of the `cloacina-server` to talk to.
    pub server: String,
    /// API key. Accepts raw literal, `env:VAR`, or `file:PATH`. `keyring:NAME`
    /// is rejected in v1 (deferred).
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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
    /// The most missed fire times that one claim of a `run_all` cron schedule
    /// takes. `None` uses the runner default, 100. It is a batch size, not a
    /// cap: the fires beyond it stay due and the next pass takes them.
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
            cron_max_catchup: None, // None = the runner default (100 per claim)
            cron_recovery_interval_s: 300,
            cron_lost_threshold_min: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CompilerSection {
    /// Local status/health address the compiler binds to (e.g. `127.0.0.1:9000`).
    /// Also used by `cloacinactl compiler status` / `health` as the probe URL.
    pub local_addr: String,
}

impl Default for CompilerSection {
    fn default() -> Self {
        Self {
            local_addr: "127.0.0.1:9000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct WatchSection {
    pub directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerSection {
    /// Executor every task is dispatched to (CLOACI-T-0640). `"fleet"` sends
    /// it to the execution-agent fleet; `"default"` runs it on the in-process
    /// thread executor and needs `single_trust` (CLOACINA-T-1075). Not set:
    /// not forwarded, and the server uses `fleet`, or `default` with single
    /// trust. `cloacinactl server start` forwards it as `--default-executor`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_executor: Option<String>,

    /// Single trust (CLOACINA-T-1075): every tenant of the server is trusted
    /// as much as the server. `true` is forwarded as `--single-trust`: the
    /// server then loads packages in its own process and allows the
    /// in-process executor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_trust: Option<bool>,

    // The settings below (CLOACINA-T-1039) have no default here: a key that
    // is not set is not forwarded, and `cloacina-server` uses its own default.
    // `cloacinactl server start` forwards a key that is set, unless the flag
    // or the env var of the same setting is given.
    /// Seconds the server waits at start for the database to accept a
    /// connection; `0` = one try (`--database-wait-timeout-s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_wait_timeout_s: Option<u64>,
    /// Days to keep a finished run; `0` keeps run history for ever
    /// (`--run-history-retention-days`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_history_retention_days: Option<u64>,
    /// Seconds to keep an acked `delivery_outbox` row; `0` keeps the rows for
    /// ever (`--outbox-acked-retention-s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbox_acked_retention_s: Option<u64>,
    /// Seconds between passes of the retention job (`--retention-interval-s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_interval_s: Option<u64>,
    /// Time limit (seconds) of a fleet task that sets none, and of a graph
    /// firing on the fleet (`--fleet-task-timeout-s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_task_timeout_s: Option<u64>,
    /// Longest time limit (seconds) a fleet task can get
    /// (`--fleet-task-timeout-max-s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fleet_task_timeout_max_s: Option<u64>,
    /// URL of the alert webhook (`CLOACINA_ALERT_WEBHOOK_URL`). The signing
    /// secret has no key here: it is the env var
    /// `CLOACINA_ALERT_WEBHOOK_SECRET` only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_webhook_url: Option<String>,
    /// Run outcomes that send an alert: `failed`, `completed`, `cancelled`,
    /// or `all` (`--alert-webhook-events`). Empty = not forwarded.
    pub alert_webhook_events: Vec<String>,
    /// Seconds allowed for one alert request (`--alert-webhook-timeout-s`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_webhook_timeout_s: Option<u64>,
    /// The most sends of one alert (`--alert-webhook-max-attempts`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_webhook_max_attempts: Option<u32>,
    /// Public URL of the server, for the links in an alert
    /// (`--public-base-url`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_base_url: Option<String>,
    /// Template of the URL of a task's logs in the log stack
    /// (`--task-log-url-template`, CLOACINA-T-1024).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_log_url_template: Option<String>,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            default_executor: None,
            single_trust: None,
            database_wait_timeout_s: None,
            run_history_retention_days: None,
            outbox_acked_retention_s: None,
            retention_interval_s: None,
            fleet_task_timeout_s: None,
            fleet_task_timeout_max_s: None,
            alert_webhook_url: None,
            alert_webhook_events: Vec::new(),
            alert_webhook_timeout_s: None,
            alert_webhook_max_attempts: None,
            public_base_url: None,
            task_log_url_template: None,
        }
    }
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
        let home = crate::home_dir().unwrap_or_else(|| PathBuf::from("."));
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

        match toml_value.clone().try_into::<CloacinaConfig>() {
            Ok(config) => *self = config,
            Err(first) => {
                // A key that is not set yet (an optional number such as
                // `server.fleet_task_timeout_s`) has no value to take the
                // type from, so `set_key` stored a string. Try the value as
                // an integer before the change is refused.
                let Ok(n) = value.parse::<i64>() else {
                    return Err(first)
                        .context("Failed to apply config change — invalid value for this key");
                };
                set_typed_key(&mut toml_value, key, toml::Value::Integer(n))?;
                *self = toml_value
                    .try_into()
                    .context("Failed to apply config change — invalid value for this key")?;
            }
        }

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

/// Put `new_value` at a dotted key path, whatever is there now.
fn set_typed_key(root: &mut toml::Value, key: &str, new_value: toml::Value) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = current
            .get_mut(part)
            .with_context(|| format!("Config section '{}' not found", part))?;
    }
    match current.as_table_mut() {
        Some(table) => {
            table.insert(parts[parts.len() - 1].to_string(), new_value);
            Ok(())
        }
        None => anyhow::bail!("Cannot set value on non-table config section"),
    }
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

/// Run `cloacinactl config profile set <NAME> <URL> --api-key <K> [--default]`.
pub fn run_profile_set(
    config_path: &Path,
    name: &str,
    server: &str,
    api_key: &str,
    default: bool,
) -> Result<()> {
    let mut config = CloacinaConfig::load(config_path);
    config.profiles.insert(
        name.to_string(),
        Profile {
            server: server.to_string(),
            api_key: api_key.to_string(),
        },
    );
    if default {
        config.default_profile = Some(name.to_string());
    }
    config.save(config_path)?;
    let marker = if default { " (default)" } else { "" };
    println!("profile '{name}' set → {server}{marker}");
    Ok(())
}

/// Run `cloacinactl config profile list`.
pub fn run_profile_list(config_path: &Path) -> Result<()> {
    let config = CloacinaConfig::load(config_path);
    if config.profiles.is_empty() {
        println!("(no profiles configured)");
        return Ok(());
    }
    let default = config.default_profile.as_deref();
    for (name, profile) in &config.profiles {
        let marker = if Some(name.as_str()) == default {
            "*"
        } else {
            " "
        };
        let redacted = redact_secret(&profile.api_key);
        println!("{marker} {name:<20} {:<40} {redacted}", profile.server);
    }
    Ok(())
}

/// Run `cloacinactl config profile use <NAME>`.
pub fn run_profile_use(config_path: &Path, name: &str) -> Result<()> {
    let mut config = CloacinaConfig::load(config_path);
    if !config.profiles.contains_key(name) {
        anyhow::bail!("no profile named '{name}'");
    }
    config.default_profile = Some(name.to_string());
    config.save(config_path)?;
    println!("default profile → {name}");
    Ok(())
}

/// Run `cloacinactl config profile delete <NAME>`.
pub fn run_profile_delete(config_path: &Path, name: &str) -> Result<()> {
    let mut config = CloacinaConfig::load(config_path);
    if config.profiles.remove(name).is_none() {
        anyhow::bail!("no profile named '{name}'");
    }
    if config.default_profile.as_deref() == Some(name) {
        config.default_profile = None;
    }
    config.save(config_path)?;
    println!("profile '{name}' deleted");
    Ok(())
}

/// Short redacted form of a secret for display.
///
/// Scheme prefixes (`env:`, `file:`, `keyring:`) pass through unchanged — they
/// don't contain the actual secret. Raw literals show first/last 4 chars.
fn redact_secret(raw: &str) -> String {
    if raw.starts_with("env:") || raw.starts_with("file:") || raw.starts_with("keyring:") {
        return raw.to_string();
    }
    if raw.len() <= 8 {
        return "<short>".to_string();
    }
    format!("{}…{}", &raw[..4], &raw[raw.len() - 4..])
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

/// Resolve the server's default executor from CLI arg or config file
/// (CLOACI-T-0640).
///
/// config.toml is the preferred surface, but an explicit `--default-executor`
/// flag (or `CLOACINA_DEFAULT_EXECUTOR` env, captured by clap) still wins for
/// ad-hoc runs — same precedence as `resolve_database_url`:
/// explicit arg > `[server].default_executor` > not set (CLOACINA-T-1075: the
/// server then picks `fleet`, or `default` with single trust).
pub fn resolve_default_executor(cli_value: Option<&str>, config_path: &Path) -> Option<String> {
    if let Some(value) = cli_value {
        return Some(value.to_string());
    }
    CloacinaConfig::load(config_path).server.default_executor
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
        assert_eq!(config.server.default_executor, None);
        assert_eq!(config.server.single_trust, None);
    }

    #[test]
    fn resolve_default_executor_precedence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        // No config, no CLI → not set: the server decides (CLOACINA-T-1075).
        assert_eq!(resolve_default_executor(None, &path), None);

        // config.toml [server].default_executor is used when no CLI override.
        std::fs::write(&path, "[server]\ndefault_executor = \"fleet\"\n").unwrap();
        assert_eq!(
            resolve_default_executor(None, &path).as_deref(),
            Some("fleet")
        );

        // Explicit CLI value wins over config.toml.
        assert_eq!(
            resolve_default_executor(Some("default"), &path).as_deref(),
            Some("default")
        );
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
    fn server_settings_are_not_set_by_default() {
        let server = CloacinaConfig::default().server;
        assert!(server.database_wait_timeout_s.is_none());
        assert!(server.run_history_retention_days.is_none());
        assert!(server.outbox_acked_retention_s.is_none());
        assert!(server.retention_interval_s.is_none());
        assert!(server.fleet_task_timeout_s.is_none());
        assert!(server.fleet_task_timeout_max_s.is_none());
        assert!(server.alert_webhook_url.is_none());
        assert!(server.alert_webhook_events.is_empty());
        assert!(server.alert_webhook_timeout_s.is_none());
        assert!(server.alert_webhook_max_attempts.is_none());
        assert!(server.public_base_url.is_none());
    }

    #[test]
    fn config_load_server_settings() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[server]
database_wait_timeout_s = 120
run_history_retention_days = 30
outbox_acked_retention_s = 600
retention_interval_s = 60
fleet_task_timeout_s = 900
fleet_task_timeout_max_s = 7200
alert_webhook_url = "https://alerts.example.com/hook"
alert_webhook_events = ["failed", "cancelled"]
alert_webhook_timeout_s = 5
alert_webhook_max_attempts = 3
public_base_url = "https://cloacina.example.com"
"#,
        )
        .unwrap();

        let server = CloacinaConfig::load(&path).server;
        assert_eq!(server.database_wait_timeout_s, Some(120));
        // A key that the file does not set keeps its default.
        assert_eq!(server.default_executor, None);
        assert_eq!(server.run_history_retention_days, Some(30));
        assert_eq!(server.outbox_acked_retention_s, Some(600));
        assert_eq!(server.retention_interval_s, Some(60));
        assert_eq!(server.fleet_task_timeout_s, Some(900));
        assert_eq!(server.fleet_task_timeout_max_s, Some(7200));
        assert_eq!(
            server.alert_webhook_url.as_deref(),
            Some("https://alerts.example.com/hook")
        );
        assert_eq!(server.alert_webhook_events, vec!["failed", "cancelled"]);
        assert_eq!(server.alert_webhook_timeout_s, Some(5));
        assert_eq!(server.alert_webhook_max_attempts, Some(3));
        assert_eq!(
            server.public_base_url.as_deref(),
            Some("https://cloacina.example.com")
        );
    }

    #[test]
    fn config_set_server_settings_that_are_not_set_yet() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = CloacinaConfig::default();
        // An optional number has no value to take the type from.
        config.set("server.fleet_task_timeout_s", "900").unwrap();
        config
            .set("server.run_history_retention_days", "0")
            .unwrap();
        config
            .set(
                "server.alert_webhook_url",
                "https://alerts.example.com/hook",
            )
            .unwrap();
        config
            .set("server.alert_webhook_events", "failed, cancelled")
            .unwrap();
        assert!(
            config
                .set("server.fleet_task_timeout_max_s", "soon")
                .is_err()
        );
        config.save(&path).unwrap();

        let server = CloacinaConfig::load(&path).server;
        assert_eq!(server.fleet_task_timeout_s, Some(900));
        assert_eq!(server.run_history_retention_days, Some(0));
        assert_eq!(
            server.alert_webhook_url.as_deref(),
            Some("https://alerts.example.com/hook")
        );
        assert_eq!(server.alert_webhook_events, vec!["failed", "cancelled"]);
        assert!(server.fleet_task_timeout_max_s.is_none());
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
