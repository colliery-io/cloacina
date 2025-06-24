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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppSettings {
    pub data_directory: String,
    pub app_database_path: String,
    pub log_directory: String,
    pub log_level: String,
    pub max_log_files: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        let data_dir = get_default_data_directory();
        Self {
            data_directory: data_dir.clone(),
            app_database_path: format!("{}/cloacina-app.db", data_dir),
            log_directory: format!("{}/logs", data_dir),
            log_level: "info".to_string(),
            max_log_files: 10,
        }
    }
}

impl AppSettings {
    pub fn load() -> Result<Self> {
        tracing::debug!("Loading application settings");
        let config_path = get_config_file_path()?;
        tracing::debug!("Settings file path: {:?}", config_path);

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;

            // Try to parse as current format
            match serde_json::from_str::<AppSettings>(&content) {
                Ok(settings) => {
                    tracing::debug!("Settings loaded successfully from {:?}", config_path);
                    Ok(settings)
                }
                Err(_) => {
                    // Try to parse as legacy format and migrate
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(legacy_json) => {
                            // Extract what we can from the legacy format
                            let data_dir = get_default_data_directory();
                            let app_database_path = legacy_json
                                .get("app_database_path")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&format!("{}/cloacina-app.db", data_dir))
                                .to_string();

                            // Create settings with legacy data + new defaults
                            let migrated_settings = AppSettings {
                                data_directory: data_dir.clone(),
                                app_database_path,
                                log_directory: format!("{}/logs", data_dir),
                                log_level: "info".to_string(),
                                max_log_files: 10,
                            };

                            // Save the migrated settings
                            migrated_settings.save()?;

                            tracing::info!("Migrated settings from legacy format");
                            Ok(migrated_settings)
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to parse settings file. The file may be corrupted. \
                                Please check: {}\nError: {}",
                                config_path.display(),
                                e
                            ));
                        }
                    }
                }
            }
        } else {
            // Create default settings and save them
            tracing::info!("Settings file not found, creating default settings");
            let settings = AppSettings::default();
            settings.save()?;
            tracing::debug!("Default settings created and saved");
            Ok(settings)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = get_config_file_path()?;
        tracing::debug!("Saving settings to {:?}", config_path);

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        tracing::debug!("Settings saved successfully");
        Ok(())
    }
}

fn get_default_data_directory() -> String {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Cloacina");

    // Ensure directory exists
    let _ = fs::create_dir_all(&data_dir);

    data_dir.to_string_lossy().to_string()
}

fn get_config_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Cloacina");

    Ok(config_dir.join("settings.json"))
}

#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    tracing::debug!("Getting application settings via command");
    AppSettings::load().map_err(|e| {
        tracing::error!("Failed to load settings: {}", e);
        format!("Failed to load settings: {}", e)
    })
}

#[tauri::command]
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    tracing::info!(
        "Saving application settings via command - log_level: {}, max_log_files: {}",
        settings.log_level,
        settings.max_log_files
    );
    settings.save().map_err(|e| {
        tracing::error!("Failed to save settings: {}", e);
        format!("Failed to save settings: {}", e)
    })
}

#[tauri::command]
pub async fn get_data_directory() -> Result<String, String> {
    tracing::debug!("Getting data directory via command");
    Ok(get_default_data_directory())
}

#[tauri::command]
pub async fn select_database_folder(
    app_handle: tauri::AppHandle,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app_handle
        .dialog()
        .file()
        .set_title("Select Database Location")
        .set_directory(&get_default_data_directory())
        .pick_folder(move |folder_path| {
            let _ = tx.send(folder_path);
        });

    // Wait for the dialog result
    match rx.await {
        Ok(Some(folder_path)) => {
            let db_path = folder_path.as_path().unwrap().join("cloacina-app.db");
            Ok(Some(db_path.to_string_lossy().to_string()))
        }
        Ok(None) => Ok(None), // User cancelled
        Err(_) => Err("Dialog was cancelled or failed".to_string()),
    }
}

#[tauri::command]
pub async fn change_database_location(new_path: String) -> Result<(), String> {
    // Load current settings to get the old path
    let current_settings =
        AppSettings::load().map_err(|e| format!("Failed to load current settings: {}", e))?;

    let old_path = current_settings.app_database_path;

    // If paths are the same, nothing to do
    if old_path == new_path {
        return Ok(());
    }

    // Check if old database exists
    if std::path::Path::new(&old_path).exists() {
        // Ensure parent directory exists for new path
        if let Some(parent) = std::path::Path::new(&new_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory for new database: {}", e))?;
        }

        // Copy the database file
        fs::copy(&old_path, &new_path).map_err(|e| {
            format!(
                "Failed to copy database from {} to {}: {}",
                old_path, new_path, e
            )
        })?;
    }

    // Save new settings (preserve other settings)
    let new_settings = AppSettings {
        data_directory: current_settings.data_directory,
        app_database_path: new_path.clone(),
        log_directory: current_settings.log_directory,
        log_level: current_settings.log_level,
        max_log_files: current_settings.max_log_files,
    };

    new_settings
        .save()
        .map_err(|e| format!("Failed to save new settings: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn generate_reset_confirmation() -> Result<String, String> {
    use rand::Rng;

    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();

    let confirmation: String = (0..7)
        .map(|_| {
            let idx = rng.gen_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect();

    Ok(confirmation)
}

#[tauri::command]
pub async fn full_system_reset(_app_handle: tauri::AppHandle) -> Result<(), String> {
    use std::path::Path;

    // Load current settings to know what to delete
    let settings = AppSettings::load().map_err(|e| format!("Failed to load settings: {}", e))?;

    // Stop all runners first by clearing the state
    {
        use crate::domains::app::state::APP_STATE;
        let mut state = APP_STATE.lock().map_err(|e| format!("Lock error: {}", e))?;
        state.runners.clear();
    }

    // Delete application database
    if Path::new(&settings.app_database_path).exists() {
        std::fs::remove_file(&settings.app_database_path)
            .map_err(|e| format!("Failed to delete app database: {}", e))?;
    }

    // Delete log directory
    if Path::new(&settings.log_directory).exists() {
        std::fs::remove_dir_all(&settings.log_directory)
            .map_err(|e| format!("Failed to delete log directory: {}", e))?;
    }

    // Delete settings file
    let config_path =
        get_config_file_path().map_err(|e| format!("Failed to get config path: {}", e))?;
    if config_path.exists() {
        std::fs::remove_file(&config_path)
            .map_err(|e| format!("Failed to delete settings file: {}", e))?;
    }

    // Attempt to restart the application
    // Note: In development mode, restart may not work - just exit cleanly
    tracing::info!("Initiating application restart after system reset");

    // Use process exit for now - this will trigger dev server restart in development
    // and clean shutdown in production (user will need to manually restart)
    std::process::exit(0);
}

#[tauri::command]
pub async fn open_log_directory() -> Result<(), String> {
    tracing::debug!("Opening log directory in system file manager");

    // Load settings to get log directory path
    let settings = AppSettings::load().map_err(|e| {
        tracing::error!("Failed to load settings for opening log directory: {}", e);
        format!("Failed to load settings: {}", e)
    })?;

    let log_dir = std::path::Path::new(&settings.log_directory);

    // Ensure the directory exists
    if !log_dir.exists() {
        tracing::info!(
            "Log directory {} doesn't exist, creating it",
            settings.log_directory
        );
        std::fs::create_dir_all(log_dir).map_err(|e| {
            tracing::error!(
                "Failed to create log directory {}: {}",
                settings.log_directory,
                e
            );
            format!("Failed to create log directory: {}", e)
        })?;
    }

    // Open the directory in the system file manager
    tracing::info!("Opening log directory: {}", settings.log_directory);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&settings.log_directory)
            .spawn()
            .map_err(|e| {
                tracing::error!("Failed to open log directory on macOS: {}", e);
                format!("Failed to open log directory: {}", e)
            })?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&settings.log_directory)
            .spawn()
            .map_err(|e| {
                tracing::error!("Failed to open log directory on Windows: {}", e);
                format!("Failed to open log directory: {}", e)
            })?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&settings.log_directory)
            .spawn()
            .map_err(|e| {
                tracing::error!("Failed to open log directory on Linux: {}", e);
                format!("Failed to open log directory: {}", e)
            })?;
    }

    tracing::debug!("Log directory opened successfully");
    Ok(())
}

#[tauri::command]
pub async fn get_runner_db_path(runner_name: String) -> Result<String, String> {
    tracing::debug!("Getting runner database path for: {}", runner_name);

    // Convert runner name to snake_case for filename
    let snake_case_name = runner_name
        .to_lowercase()
        .replace(" ", "_")
        .replace("-", "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();

    // Use a default name if the result is empty
    let filename = if snake_case_name.is_empty() {
        "unnamed_runner".to_string()
    } else {
        snake_case_name
    };

    // Return just the relative path from data directory
    let relative_path = format!("runners/{}.db", filename);

    tracing::debug!("Runner database relative path: {}", relative_path);
    Ok(relative_path)
}

#[tauri::command]
pub async fn get_full_path(relative_path: String) -> Result<String, String> {
    tracing::debug!("Converting relative path to full path: {}", relative_path);

    let data_dir = get_default_data_directory();
    let full_path = format!("{}/{}", data_dir, relative_path);

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&full_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    tracing::debug!("Full path: {}", full_path);
    Ok(full_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();

        assert!(!settings.data_directory.is_empty());
        assert!(settings.app_database_path.contains("cloacina-app.db"));
        assert!(settings.log_directory.contains("logs"));
        assert_eq!(settings.log_level, "info");
        assert_eq!(settings.max_log_files, 10);
    }

    #[test]
    fn test_app_settings_serialization() {
        let settings = AppSettings {
            data_directory: "/test/data".to_string(),
            app_database_path: "/test/data/app.db".to_string(),
            log_directory: "/test/data/logs".to_string(),
            log_level: "debug".to_string(),
            max_log_files: 5,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.data_directory, "/test/data");
        assert_eq!(deserialized.log_level, "debug");
        assert_eq!(deserialized.max_log_files, 5);
    }

    #[tokio::test]
    async fn test_get_runner_db_path() {
        let result = get_runner_db_path("Test Runner Name".to_string()).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path, "runners/test_runner_name.db");
    }

    #[tokio::test]
    async fn test_get_runner_db_path_special_chars() {
        let result = get_runner_db_path("My-Cool Runner #1!".to_string()).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path, "runners/my_cool_runner_1.db");
    }

    #[tokio::test]
    async fn test_get_runner_db_path_empty() {
        let result = get_runner_db_path("".to_string()).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path, "runners/unnamed_runner.db");
    }

    #[tokio::test]
    async fn test_get_full_path() {
        let result = get_full_path("runners/test.db".to_string()).await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(path.ends_with("/runners/test.db"));
        assert!(path.contains("Cloacina"));
    }

    #[tokio::test]
    async fn test_get_data_directory() {
        let result = get_data_directory().await;
        assert!(result.is_ok());

        let path = result.unwrap();
        assert!(!path.is_empty());
        assert!(path.contains("Cloacina"));
    }
}
