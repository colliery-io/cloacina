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

use crate::config::{types::*, ValidationError};
use crate::database::validate_backend_compatibility;

pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

impl Validate for CloacinaConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Vec::new();

        if let Err(e) = self.database.validate() {
            errors.push(e);
        }
        if let Err(e) = self.execution.validate() {
            errors.push(e);
        }
        if let Err(e) = self.registry.validate() {
            errors.push(e);
        }
        if let Err(e) = self.cron.validate() {
            errors.push(e);
        }
        if let Err(e) = self.server.validate() {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(ValidationError::Multiple { errors })
        }
    }
}

impl Validate for DatabaseConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate URL format and backend compatibility
        if let Err(e) = validate_backend_compatibility(&self.url) {
            return Err(ValidationError::BackendMismatch {
                message: e.to_string(),
            });
        }

        // Validate pool size
        if self.pool_size == 0 || self.pool_size > 100 {
            return Err(ValidationError::InvalidPoolSize {
                size: self.pool_size,
            });
        }

        Ok(())
    }
}

impl Validate for ExecutionConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate timeout
        if self.task_timeout_secs == 0 {
            return Err(ValidationError::InvalidTimeout {
                timeout: self.task_timeout_secs,
            });
        }

        // Validate max concurrent tasks
        if self.max_concurrent_tasks == 0 {
            return Err(ValidationError::InvalidTimeout {
                timeout: self.max_concurrent_tasks as u64,
            });
        }

        // Validate worker threads if specified
        if let Some(threads) = self.worker_threads {
            if threads == 0 {
                return Err(ValidationError::InvalidTimeout {
                    timeout: threads as u64,
                });
            }
        }

        Ok(())
    }
}

impl Validate for RegistryConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        self.storage.validate()
    }
}

impl Validate for RegistryStorageConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        match self.storage_type.as_str() {
            "filesystem" => {
                if self.path.is_none() {
                    return Err(ValidationError::InvalidPath {
                        path: "path is required for filesystem storage".to_string(),
                    });
                }
            }
            "database" => {
                if self.connection_string.is_none() {
                    return Err(ValidationError::InvalidPath {
                        path: "connection_string is required for database storage".to_string(),
                    });
                }
            }
            _ => {
                return Err(ValidationError::InvalidPath {
                    path: format!("unsupported storage type: {}", self.storage_type),
                });
            }
        }
        Ok(())
    }
}

impl Validate for CronConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.check_interval_secs == 0 {
            return Err(ValidationError::InvalidTimeout {
                timeout: self.check_interval_secs,
            });
        }
        Ok(())
    }
}

impl Validate for ServerConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate log level
        match self.log_level.to_lowercase().as_str() {
            "error" | "warn" | "info" | "debug" | "trace" => {}
            _ => {
                return Err(ValidationError::InvalidLogLevel {
                    level: self.log_level.clone(),
                });
            }
        }

        // Validate timeout
        if self.graceful_shutdown_timeout_secs == 0 {
            return Err(ValidationError::InvalidTimeout {
                timeout: self.graceful_shutdown_timeout_secs,
            });
        }

        // Validate API config
        self.api.validate()
    }
}

impl Validate for ApiConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        self.unix_socket.validate()?;

        if let Some(ref http) = self.http {
            http.validate()?;
        }

        Ok(())
    }
}

impl Validate for UnixSocketConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate permissions if specified
        if let Some(perms) = self.permissions {
            if perms > 0o777 {
                return Err(ValidationError::InvalidPath {
                    path: format!("invalid unix socket permissions: {:o}", perms),
                });
            }
        }
        Ok(())
    }
}

impl Validate for HttpConfig {
    fn validate(&self) -> Result<(), ValidationError> {
        // Validate port range
        if self.port == 0 {
            return Err(ValidationError::InvalidPath {
                path: "HTTP port cannot be 0".to_string(),
            });
        }

        // Basic bind address validation
        if self.bind_address.is_empty() {
            return Err(ValidationError::InvalidPath {
                path: "HTTP bind address cannot be empty".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_config_validation() {
        let mut config = DatabaseConfig::default();

        // Should pass with default config
        assert!(config.validate().is_ok());

        // Should fail with invalid pool size
        config.pool_size = 0;
        assert!(config.validate().is_err());

        config.pool_size = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_server_config_validation() {
        let mut config = ServerConfig::default();

        // Should pass with default config
        assert!(config.validate().is_ok());

        // Should fail with invalid log level
        config.log_level = "invalid".to_string();
        assert!(config.validate().is_err());

        // Should fail with zero timeout
        config.log_level = "info".to_string();
        config.graceful_shutdown_timeout_secs = 0;
        assert!(config.validate().is_err());
    }
}
