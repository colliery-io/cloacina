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

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Process environment variables and merge them into the context
pub fn process_environment_variables(
    context: &mut serde_json::Value,
    env_vars: &[String],
    env_file: &Option<PathBuf>,
    include_env: &bool,
    env_prefix: &Option<String>,
) -> Result<()> {
    // Ensure context is an object
    if !context.is_object() {
        anyhow::bail!("Context must be a JSON object to merge environment variables");
    }

    let context_obj = context.as_object_mut().unwrap();

    // 1. Load from .env file if provided
    if let Some(env_file_path) = env_file {
        let env_content = std::fs::read_to_string(env_file_path)
            .with_context(|| format!("Failed to read env file: {:?}", env_file_path))?;

        for line in env_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                context_obj.insert(
                    format!("env_{}", key),
                    serde_json::Value::String(value.to_string()),
                );
            }
        }
    }

    // 2. Include current environment variables if requested
    if *include_env {
        for (key, value) in std::env::vars() {
            let should_include = if let Some(prefix) = env_prefix {
                key.starts_with(prefix)
            } else {
                true
            };

            if should_include {
                context_obj.insert(format!("env_{}", key), serde_json::Value::String(value));
            }
        }
    }

    // 3. Process explicit environment variables (these override others)
    for env_var in env_vars {
        if let Some((key, value)) = env_var.split_once('=') {
            context_obj.insert(
                format!("env_{}", key),
                serde_json::Value::String(value.to_string()),
            );
        } else {
            anyhow::bail!(
                "Invalid environment variable format: '{}'. Expected KEY=VALUE",
                env_var
            );
        }
    }

    Ok(())
}

pub fn get_current_architecture() -> String {
    // Get the current target triple
    if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
        "x86_64-unknown-linux-gnu".to_string()
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "macos") {
        "x86_64-apple-darwin".to_string()
    } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        "aarch64-apple-darwin".to_string()
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc".to_string()
    } else {
        "unknown".to_string()
    }
}
