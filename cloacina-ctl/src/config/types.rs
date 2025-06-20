/*
 *  Copyright 2025 Colliery Software
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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloacinaConfig {
    pub database: DatabaseConfig,
    pub execution: ExecutionConfig,
    pub registry: RegistryConfig,
    pub cron: CronConfig,
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub max_concurrent_tasks: u32,
    pub task_timeout_secs: u64,
    pub worker_threads: Option<u32>,
    pub polling_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub enabled: bool,
    pub storage: RegistryStorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStorageConfig {
    #[serde(rename = "type")]
    pub storage_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    pub enabled: bool,
    pub check_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub pid_file: PathBuf,
    pub log_file: PathBuf,
    pub log_level: String,
    pub graceful_shutdown_timeout_secs: u64,
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub unix_socket: UnixSocketConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnixSocketConfig {
    pub enabled: bool,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
}

impl CloacinaConfig {
    /// Create a new configuration with defaults for the compiled backend
    pub fn with_defaults() -> Self {
        Self {
            database: DatabaseConfig::default(),
            execution: ExecutionConfig::default(),
            registry: RegistryConfig::default(),
            cron: CronConfig::default(),
            server: ServerConfig::default(),
        }
    }
}
