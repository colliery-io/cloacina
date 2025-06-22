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

use crate::config::types::*;
use std::path::PathBuf;

impl Default for CloacinaConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            execution: ExecutionConfig::default(),
            registry: RegistryConfig::default(),
            cron: CronConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

#[cfg(feature = "postgres")]
impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "${CLOACINA_DATABASE_URL:-postgresql://cloacina:cloacina@localhost:5432/cloacina}"
                .to_string(),
            pool_size: 10,
            schema: None,
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "${CLOACINA_DATABASE_URL:-sqlite:///var/lib/cloacina/cloacina.db}".to_string(),
            pool_size: 1,
            schema: None,
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            task_timeout_secs: 300,
            worker_threads: None,      // Auto-detect based on CPU cores
            polling_interval_ms: 1000, // 1 second default polling
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage: RegistryStorageConfig::default(),
        }
    }
}

impl Default for RegistryStorageConfig {
    fn default() -> Self {
        Self {
            storage_type: "filesystem".to_string(),
            path: Some(PathBuf::from("/var/lib/cloacina/registry")),
            connection_string: None,
        }
    }
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 60,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            pid_file: PathBuf::from("/var/run/cloacina.pid"),
            log_file: PathBuf::from("/var/log/cloacina/cloacina.log"),
            log_level: "info".to_string(),
            graceful_shutdown_timeout_secs: 30,
            api: ApiConfig::default(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            unix_socket: UnixSocketConfig::default(),
            http: None, // Disabled by default
        }
    }
}

impl Default for UnixSocketConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: PathBuf::from("/var/run/cloacina/api.sock"),
            permissions: Some(0o660),
        }
    }
}

/// Generate a complete default configuration as TOML string
pub fn generate_default_config_toml() -> Result<String, toml::ser::Error> {
    let config = CloacinaConfig::default();
    toml::to_string_pretty(&config)
}
