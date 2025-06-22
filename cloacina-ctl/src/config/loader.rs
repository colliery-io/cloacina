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

use crate::config::{CloacinaConfig, ConfigError};
use regex::Regex;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ConfigLoader {
    search_paths: Vec<PathBuf>,
}

impl ConfigLoader {
    /// Create a new config loader with default search paths
    pub fn new() -> Self {
        let mut search_paths = Vec::new();

        // 1. Current directory
        search_paths.push(PathBuf::from("./cloacina.yaml"));
        search_paths.push(PathBuf::from("./cloacina.yml"));
        search_paths.push(PathBuf::from("./cloacina.toml"));

        // 2. User config directory
        if let Some(config_dir) = dirs::config_dir() {
            let user_config = config_dir.join("cloacina");
            search_paths.push(user_config.join("config.yaml"));
            search_paths.push(user_config.join("config.yml"));
            search_paths.push(user_config.join("config.toml"));
        }

        // 3. System config directory
        search_paths.push(PathBuf::from("/etc/cloacina/config.yaml"));
        search_paths.push(PathBuf::from("/etc/cloacina/config.yml"));
        search_paths.push(PathBuf::from("/etc/cloacina/config.toml"));

        Self { search_paths }
    }

    /// Create a config loader with custom search paths
    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }

    /// Load configuration from the specified file or auto-discover
    pub fn load_config(&self, config_file: Option<&Path>) -> Result<CloacinaConfig, ConfigError> {
        let config_path = if let Some(path) = config_file {
            path.to_path_buf()
        } else if let Ok(env_config) = env::var("CLOACINA_CONFIG") {
            PathBuf::from(env_config)
        } else {
            self.find_config_file().ok_or(ConfigError::ConfigNotFound)?
        };

        self.load_config_from_file(&config_path)
    }

    /// Load configuration from a specific file
    pub fn load_config_from_file(&self, path: &Path) -> Result<CloacinaConfig, ConfigError> {
        let content = fs::read_to_string(path).map_err(|source| ConfigError::ReadError {
            path: path.to_path_buf(),
            source,
        })?;

        // Apply environment variable substitution
        let substituted_content = self.substitute_env_vars(&content)?;

        // Parse based on file extension
        let config = match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => toml::from_str::<CloacinaConfig>(&substituted_content)?,
            Some(ext) => {
                return Err(ConfigError::UnsupportedFormat {
                    extension: ext.to_string(),
                })
            }
            None => {
                // Try TOML for files without extension
                toml::from_str::<CloacinaConfig>(&substituted_content)?
            }
        };

        Ok(config)
    }

    /// Find the first existing configuration file in search paths
    pub fn find_config_file(&self) -> Option<PathBuf> {
        for path in &self.search_paths {
            if path.exists() && path.is_file() {
                return Some(path.clone());
            }
        }
        None
    }

    /// Substitute environment variables in configuration content
    fn substitute_env_vars(&self, content: &str) -> Result<String, ConfigError> {
        // Regex to match ${VAR}, ${VAR:-default}, ${VAR:?error}
        let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
        let mut result = content.to_string();

        for cap in re.captures_iter(content) {
            let full_match = &cap[0];
            let var_expr = &cap[1];

            let replacement = self.process_var_expression(var_expr)?;
            result = result.replace(full_match, &replacement);
        }

        Ok(result)
    }

    /// Process a variable expression like "VAR", "VAR:-default", or "VAR:?error"
    fn process_var_expression(&self, expr: &str) -> Result<String, ConfigError> {
        if let Some(default_pos) = expr.find(":-") {
            // ${VAR:-default} syntax
            let var_name = &expr[..default_pos];
            let default_value = &expr[default_pos + 2..];
            Ok(env::var(var_name).unwrap_or_else(|_| default_value.to_string()))
        } else if let Some(error_pos) = expr.find(":?") {
            // ${VAR:?error} syntax
            let var_name = &expr[..error_pos];
            let error_msg = &expr[error_pos + 2..];
            env::var(var_name).map_err(|_| {
                ConfigError::EnvSubstitutionError(format!(
                    "Required environment variable '{}' is not set: {}",
                    var_name, error_msg
                ))
            })
        } else {
            // ${VAR} syntax - required variable
            env::var(expr).map_err(|_| {
                ConfigError::EnvSubstitutionError(format!(
                    "Required environment variable '{}' is not set",
                    expr
                ))
            })
        }
    }

    /// Get all search paths for debugging
    pub fn get_search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_env_substitution_with_default() {
        let loader = ConfigLoader::new();
        env::remove_var("TEST_VAR_DEFAULT");

        let content = "database_url: ${TEST_VAR_DEFAULT:-postgresql://localhost/test}";
        let result = loader.substitute_env_vars(content).unwrap();
        assert_eq!(result, "database_url: postgresql://localhost/test");
    }

    #[test]
    fn test_env_substitution_with_existing_var() {
        let loader = ConfigLoader::new();
        env::set_var("TEST_VAR", "custom_value");

        let content = "database_url: ${TEST_VAR:-postgresql://localhost/test}";
        let result = loader.substitute_env_vars(content).unwrap();
        assert_eq!(result, "database_url: custom_value");

        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_env_substitution_required_var_missing() {
        let loader = ConfigLoader::new();
        env::remove_var("REQUIRED_VAR");

        let content = "database_url: ${REQUIRED_VAR}";
        let result = loader.substitute_env_vars(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_env_substitution_custom_error() {
        let loader = ConfigLoader::new();
        env::remove_var("REQUIRED_VAR");

        let content = "database_url: ${REQUIRED_VAR:?Database URL must be provided}";
        let result = loader.substitute_env_vars(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Database URL must be provided"));
    }
}
