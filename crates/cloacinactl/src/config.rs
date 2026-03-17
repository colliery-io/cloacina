/*
 *  Copyright 2025-2026 Colliery Software
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

//! Server configuration with layered loading: defaults → TOML file → env vars → CLI flags.

use crate::commands::serve::ServeArgs;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use tracing::info;

/// Top-level server configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub database: DatabaseSection,
    pub scheduler: SchedulerSection,
    pub worker: WorkerSection,
    pub logging: LoggingSection,
    pub observability: ObservabilitySection,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSection::default(),
            database: DatabaseSection::default(),
            scheduler: SchedulerSection::default(),
            worker: WorkerSection::default(),
            logging: LoggingSection::default(),
            observability: ObservabilitySection::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerSection {
    pub bind: String,
    pub port: u16,
    pub mode: String,
}

impl Default for ServerSection {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 8080,
            mode: "all".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseSection {
    pub url: String,
    pub pool_size: u32,
}

impl Default for DatabaseSection {
    fn default() -> Self {
        Self {
            url: String::new(),
            pool_size: 10,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSection {
    pub poll_interval_ms: u64,
    pub enable_continuous: bool,
    pub continuous_poll_interval_ms: u64,
}

impl Default for SchedulerSection {
    fn default() -> Self {
        Self {
            poll_interval_ms: 100,
            enable_continuous: false,
            continuous_poll_interval_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WorkerSection {
    pub max_concurrent_tasks: usize,
    pub task_timeout_seconds: u64,
}

impl Default for WorkerSection {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 10,
            task_timeout_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingSection {
    pub level: String,
    pub format: String,
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ObservabilitySection {
    /// OpenTelemetry OTLP endpoint. Empty string disables OTLP export.
    pub otlp_endpoint: String,
    /// Service name reported to the OTLP collector.
    pub otlp_service_name: String,
}

impl Default for ObservabilitySection {
    fn default() -> Self {
        Self {
            otlp_endpoint: String::new(),
            otlp_service_name: "cloacina".to_string(),
        }
    }
}

/// Load configuration with layered precedence: defaults → TOML → env vars → CLI flags.
pub fn load_config(cli_config_path: Option<&str>, cli_args: &ServeArgs) -> Result<ServerConfig> {
    // Start with defaults
    let mut config = ServerConfig::default();

    // Layer 1: TOML file
    let config_path = discover_config_file(cli_config_path);
    if let Some(ref path) = config_path {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;
        config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse TOML config: {}", path))?;
        info!("Loaded config from {}", path);
    }

    // Layer 2: Environment variables (CLOACINA_ prefix)
    apply_env_overrides(&mut config);

    // Layer 3: CLI flags (highest precedence)
    apply_cli_overrides(&mut config, cli_args);

    Ok(config)
}

/// Discover the config file path from CLI flag, CWD, or user config dir.
fn discover_config_file(cli_path: Option<&str>) -> Option<String> {
    // Explicit --config flag
    if let Some(path) = cli_path {
        return Some(path.to_string());
    }

    // Current directory
    let cwd_path = "cloacina.toml";
    if Path::new(cwd_path).exists() {
        return Some(cwd_path.to_string());
    }

    // User config directory
    if let Some(config_dir) = dirs::config_dir() {
        let user_path = config_dir.join("cloacina").join("cloacina.toml");
        if user_path.exists() {
            return Some(user_path.to_string_lossy().to_string());
        }
    }

    None
}

/// Apply environment variable overrides with CLOACINA_ prefix.
fn apply_env_overrides(config: &mut ServerConfig) {
    if let Ok(val) = std::env::var("CLOACINA_SERVER_BIND") {
        config.server.bind = val;
    }
    if let Ok(val) = std::env::var("CLOACINA_SERVER_PORT") {
        if let Ok(port) = val.parse::<u16>() {
            config.server.port = port;
        }
    }
    if let Ok(val) = std::env::var("CLOACINA_SERVER_MODE") {
        config.server.mode = val;
    }
    if let Ok(val) = std::env::var("CLOACINA_DATABASE_URL") {
        config.database.url = val;
    }
    if let Ok(val) = std::env::var("CLOACINA_DATABASE_POOL_SIZE") {
        if let Ok(size) = val.parse::<u32>() {
            config.database.pool_size = size;
        }
    }
    if let Ok(val) = std::env::var("CLOACINA_SCHEDULER_POLL_INTERVAL_MS") {
        if let Ok(ms) = val.parse::<u64>() {
            config.scheduler.poll_interval_ms = ms;
        }
    }
    if let Ok(val) = std::env::var("CLOACINA_WORKER_MAX_CONCURRENT") {
        if let Ok(n) = val.parse::<usize>() {
            config.worker.max_concurrent_tasks = n;
        }
    }
    if let Ok(val) = std::env::var("CLOACINA_LOG_LEVEL") {
        config.logging.level = val;
    }
    if let Ok(val) = std::env::var("CLOACINA_OTLP_ENDPOINT") {
        config.observability.otlp_endpoint = val;
    }
    if let Ok(val) = std::env::var("CLOACINA_OTLP_SERVICE_NAME") {
        config.observability.otlp_service_name = val;
    }
}

/// Apply CLI flag overrides (highest precedence).
fn apply_cli_overrides(config: &mut ServerConfig, args: &ServeArgs) {
    config.server.bind = args.bind.clone();
    config.server.port = args.port;
    config.server.mode = args.mode.to_string();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::serve::ServeMode;

    fn default_args() -> ServeArgs {
        ServeArgs {
            mode: ServeMode::All,
            config: None,
            bind: "0.0.0.0".to_string(),
            port: 8080,
        }
    }

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.server.bind, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.mode, "all");
        assert_eq!(config.database.pool_size, 10);
        assert_eq!(config.scheduler.poll_interval_ms, 100);
        assert_eq!(config.worker.max_concurrent_tasks, 10);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_toml_parsing() {
        let toml_str = r#"
[server]
bind = "127.0.0.1"
port = 9090
mode = "api"

[database]
url = "postgres://user:pass@localhost/db"
pool_size = 20

[scheduler]
poll_interval_ms = 500
enable_continuous = true

[worker]
max_concurrent_tasks = 50
task_timeout_seconds = 600

[logging]
level = "debug"
format = "pretty"
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.database.url, "postgres://user:pass@localhost/db");
        assert_eq!(config.database.pool_size, 20);
        assert!(config.scheduler.enable_continuous);
        assert_eq!(config.worker.max_concurrent_tasks, 50);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_partial_toml_uses_defaults() {
        let toml_str = r#"
[database]
url = "postgres://localhost/mydb"
"#;
        let config: ServerConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.server.bind, "0.0.0.0"); // default
        assert_eq!(config.server.port, 8080); // default
        assert_eq!(config.database.url, "postgres://localhost/mydb"); // from file
        assert_eq!(config.database.pool_size, 10); // default
    }

    #[test]
    fn test_env_var_overlay() {
        let mut config = ServerConfig::default();
        std::env::set_var("CLOACINA_DATABASE_URL", "postgres://env-test/db");
        std::env::set_var("CLOACINA_SERVER_PORT", "3000");

        apply_env_overrides(&mut config);

        assert_eq!(config.database.url, "postgres://env-test/db");
        assert_eq!(config.server.port, 3000);

        // Clean up
        std::env::remove_var("CLOACINA_DATABASE_URL");
        std::env::remove_var("CLOACINA_SERVER_PORT");
    }

    #[test]
    fn test_cli_overrides_take_precedence() {
        let mut config = ServerConfig::default();
        config.server.port = 9090; // from TOML
        config.server.bind = "127.0.0.1".to_string(); // from TOML

        let args = ServeArgs {
            mode: ServeMode::Worker,
            config: None,
            bind: "10.0.0.1".to_string(),
            port: 4000,
        };

        apply_cli_overrides(&mut config, &args);

        assert_eq!(config.server.port, 4000); // CLI wins
        assert_eq!(config.server.bind, "10.0.0.1"); // CLI wins
        assert_eq!(config.server.mode, "worker"); // CLI wins
    }

    #[test]
    fn test_load_config_no_file() {
        let args = default_args();
        let config = load_config(None, &args).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.url, ""); // no file, no env
    }

    #[test]
    fn test_missing_config_file_is_not_error() {
        let args = default_args();
        // No cloacina.toml in CWD, no --config flag
        let result = load_config(None, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_explicit_missing_config_file_is_error() {
        let args = default_args();
        let result = load_config(Some("/nonexistent/cloacina.toml"), &args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to read config file"));
    }
}
