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

use serde::Serialize;
use tauri::command;

use super::settings::AppSettings;
use super::state::{APP_STATE, RUNNER_SERVICE};
use crate::domains::runners::{RunnerDAL, RunnerService};
use std::sync::Arc;

#[derive(Serialize)]
pub struct AppStatus {
    pub total_runners: usize,
    pub running_runners: usize,
    pub paused_runners: usize,
}

// Initialize the application database and load existing runners
#[tauri::command]
pub async fn initialize_app() -> Result<AppStatus, String> {
    tracing::info!("Initializing application");

    // Load settings to get database path
    let settings = AppSettings::load().map_err(|e| {
        tracing::error!("Failed to load settings: {}", e);
        format!("Failed to load settings: {}", e)
    })?;

    tracing::debug!(
        "Loaded settings - database path: {}",
        settings.app_database_path
    );

    // Initialize runner DAL and service
    let runner_dal = RunnerDAL::new(&settings.app_database_path).map_err(|e| {
        tracing::error!(
            "Failed to initialize app database at {}: {}",
            settings.app_database_path,
            e
        );
        format!("Failed to initialize app database: {}", e)
    })?;

    let runner_service = Arc::new(RunnerService::new(runner_dal));
    tracing::debug!("Created runner service");

    // Store the service globally
    {
        let mut service_guard = RUNNER_SERVICE.lock().map_err(|e| {
            tracing::error!(
                "Failed to acquire runner service lock for initialization: {}",
                e
            );
            format!("Lock error: {}", e)
        })?;
        *service_guard = Some(runner_service.clone());
        tracing::debug!("Stored runner service globally");
    }

    // Load and start existing runners
    tracing::info!("Loading and starting existing runners from database");
    let (active_runners, total_runners, running_runners, paused_runners) =
        runner_service.load_and_start_runners().await.map_err(|e| {
            tracing::error!("Failed to load and start runners: {}", e);
            e
        })?;

    tracing::info!(
        "Loaded {} total runners, {} running, {} paused",
        total_runners,
        running_runners,
        paused_runners
    );

    // Store the active runners
    {
        let mut state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock for runner storage: {}", e);
            format!("Lock error: {}", e)
        })?;
        state.runners = active_runners;
        tracing::debug!("Stored {} active runners in app state", state.runners.len());
    };

    tracing::info!("Application initialization complete");

    Ok(AppStatus {
        total_runners,
        running_runners,
        paused_runners,
    })
}

/// Open directory selection dialog
#[command]
pub async fn select_directory_dialog(
    app_handle: tauri::AppHandle,
    title: String,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app_handle
        .dialog()
        .file()
        .set_title(&title)
        .pick_folder(move |folder_path| {
            let _ = tx.send(folder_path);
        });

    // Wait for the dialog result
    match rx.await {
        Ok(Some(folder_path)) => {
            let path_str = folder_path
                .as_path()
                .ok_or("Invalid path")?
                .to_string_lossy()
                .to_string();
            Ok(Some(path_str))
        }
        Ok(None) => Ok(None), // User cancelled
        Err(e) => Err(format!("Dialog error: {}", e)),
    }
}

/// Open save file dialog
#[command]
pub async fn save_file_dialog(
    app_handle: tauri::AppHandle,
    title: String,
    default_filename: Option<String>,
    _filters: Option<Vec<serde_json::Value>>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    let mut dialog = app_handle.dialog().file().set_title(&title);

    if let Some(filename) = default_filename {
        dialog = dialog.set_file_name(&filename);
    }

    dialog.save_file(move |file_path| {
        let _ = tx.send(file_path);
    });

    // Wait for the dialog result
    match rx.await {
        Ok(Some(file_path)) => {
            let path_str = file_path
                .as_path()
                .ok_or("Invalid path")?
                .to_string_lossy()
                .to_string();
            Ok(Some(path_str))
        }
        Ok(None) => Ok(None), // User cancelled
        Err(e) => Err(format!("Dialog error: {}", e)),
    }
}

/// Open file picker dialog for selecting existing files
#[command]
pub async fn select_file_dialog(
    app_handle: tauri::AppHandle,
    title: String,
    _filters: Option<Vec<serde_json::Value>>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();

    app_handle
        .dialog()
        .file()
        .set_title(&title)
        .pick_file(move |file_path| {
            let _ = tx.send(file_path);
        });

    // Wait for the dialog result
    match rx.await {
        Ok(Some(file_path)) => {
            let path_str = file_path
                .as_path()
                .ok_or("Invalid path")?
                .to_string_lossy()
                .to_string();
            Ok(Some(path_str))
        }
        Ok(None) => Ok(None), // User cancelled
        Err(e) => Err(format!("Dialog error: {}", e)),
    }
}

/// Open file location in system file manager
#[command]
pub async fn open_file_location(path: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .args(["/select,", &path])
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(
                std::path::Path::new(&path)
                    .parent()
                    .unwrap_or(std::path::Path::new(".")),
            )
            .spawn()
            .map_err(|e| format!("Failed to open file location: {}", e))?;
    }

    Ok(())
}

/// Get the user's desktop directory path with defensive checks
#[command]
pub async fn get_desktop_path() -> Result<String, String> {
    use dirs;

    // Log the OS for debugging
    let os = std::env::consts::OS;
    tracing::debug!("Detecting desktop path for OS: {}", os);

    // First try to get the desktop directory
    if let Some(desktop_path) = dirs::desktop_dir() {
        let path_str = desktop_path.to_string_lossy().to_string();

        // Defensive check: verify the desktop directory actually exists
        if desktop_path.exists() {
            tracing::debug!("Found and verified desktop directory: {}", path_str);
            return Ok(path_str);
        } else {
            tracing::warn!("Desktop directory detected but doesn't exist: {}", path_str);
        }
    } else {
        tracing::warn!("Could not detect desktop directory from system");
    }

    // Fallback 1: Try home/Desktop
    if let Some(home_path) = dirs::home_dir() {
        let desktop_fallback = home_path.join("Desktop");
        if desktop_fallback.exists() {
            let path_str = desktop_fallback.to_string_lossy().to_string();
            tracing::debug!("Using existing home/Desktop fallback: {}", path_str);
            return Ok(path_str);
        } else {
            tracing::debug!(
                "Home/Desktop fallback doesn't exist: {}",
                desktop_fallback.display()
            );
        }

        // Fallback 2: Just use home directory
        let home_str = home_path.to_string_lossy().to_string();
        tracing::debug!("Using home directory as final fallback: {}", home_str);
        return Ok(home_str);
    }

    // Last resort error
    tracing::error!("Could not determine any suitable directory (desktop, home/Desktop, or home)");
    Err("Could not determine a suitable output directory".to_string())
}

/// Get system path information for debugging
#[command]
pub async fn get_system_paths() -> Result<serde_json::Value, String> {
    use dirs;
    use serde_json::json;

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let home_dir = dirs::home_dir().map(|p| p.to_string_lossy().to_string());
    let desktop_dir = dirs::desktop_dir().map(|p| p.to_string_lossy().to_string());
    let documents_dir = dirs::document_dir().map(|p| p.to_string_lossy().to_string());

    Ok(json!({
        "os": os,
        "arch": arch,
        "home_dir": home_dir,
        "desktop_dir": desktop_dir,
        "documents_dir": documents_dir,
        "expected_desktop_paths": {
            "macos": "/Users/[username]/Desktop",
            "windows": "C:\\Users\\[username]\\Desktop",
            "linux": "/home/[username]/Desktop"
        }
    }))
}
