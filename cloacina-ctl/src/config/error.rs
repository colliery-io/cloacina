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

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found in any search location")]
    ConfigNotFound,

    #[error("Failed to read configuration file {path}: {source}")]
    ReadError {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse YAML configuration: {0}")]
    YamlParseError(#[from] serde_yaml::Error),

    #[error("Failed to parse TOML configuration: {0}")]
    TomlParseError(#[from] toml::de::Error),

    #[error("Environment variable substitution failed: {0}")]
    EnvSubstitutionError(String),

    #[error("Configuration validation failed: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("Unsupported configuration file format: {extension}")]
    UnsupportedFormat { extension: String },
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid database URL: {url}")]
    InvalidDatabaseUrl { url: String },

    #[error("Invalid pool size: {size} (must be between 1 and 100)")]
    InvalidPoolSize { size: u32 },

    #[error("Invalid log level: {level} (must be one of: error, warn, info, debug, trace)")]
    InvalidLogLevel { level: String },

    #[error("Invalid timeout value: {timeout} (must be positive)")]
    InvalidTimeout { timeout: u64 },

    #[error("Database backend mismatch: {message}")]
    BackendMismatch { message: String },

    #[error("Invalid file path: {path}")]
    InvalidPath { path: String },

    #[error("Multiple validation errors: {errors:?}")]
    Multiple { errors: Vec<ValidationError> },
}
