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

//! Daemon configuration file support.
//!
//! Reads `~/.cloacina/config.toml` for daemon settings. CLI args
//! override config file values. Invalid config is logged and ignored
//! — the daemon continues with previous/default settings.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Daemon configuration from `config.toml`.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DaemonConfig {
    pub daemon: DaemonSection,
    pub watch: WatchSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DaemonSection {
    pub poll_interval_ms: u64,
    pub log_level: String,
}

impl Default for DaemonSection {
    fn default() -> Self {
        Self {
            poll_interval_ms: 50,
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct WatchSection {
    pub directories: Vec<String>,
}

impl DaemonConfig {
    /// Load config from a TOML file. Returns default config if file doesn't exist.
    /// Logs and returns default on parse errors (never crashes).
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(path) {
            Ok(contents) => match toml::from_str::<DaemonConfig>(&contents) {
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

    /// Resolve watch directories from config, expanding `~` to home dir.
    pub fn resolve_watch_dirs(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        self.watch
            .directories
            .iter()
            .map(|d| {
                if d.starts_with("~/") {
                    home.join(&d[2..])
                } else {
                    PathBuf::from(d)
                }
            })
            .collect()
    }
}
