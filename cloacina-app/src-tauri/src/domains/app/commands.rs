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

// Import cloacina DAL for direct database operations
use cloacina::dal::DAL;
use cloacina::Database;

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

/// Register a workflow package to a specific runner's registry
#[command]
pub async fn register_workflow_package(
    runner_id: String,
    package_path: String,
) -> Result<serde_json::Value, String> {
    use cloacina::dal::{SqliteRegistryStorage, DAL};
    use cloacina::Database;
    use serde_json::json;
    use std::{fs, sync::Arc};

    // Read the package file
    let package_data =
        fs::read(&package_path).map_err(|e| format!("Failed to read package file: {}", e))?;

    // Get the specific runner to access its database connection
    let runner_records = {
        let runner_service_guard = RUNNER_SERVICE
            .lock()
            .map_err(|e| format!("Failed to acquire runner service lock: {}", e))?;
        let runner_service = runner_service_guard
            .as_ref()
            .ok_or_else(|| "Runner service not initialized".to_string())?;
        runner_service
            .get_all_runners(&{
                let state = APP_STATE
                    .lock()
                    .map_err(|e| format!("Failed to acquire app state lock: {}", e))?;
                state.runners.clone()
            })
            .map_err(|e| format!("Failed to get runner configurations: {}", e))?
    };

    let runner_config = runner_records
        .iter()
        .find(|r| r.id == runner_id)
        .ok_or_else(|| "Runner configuration not found".to_string())?
        .clone();

    // Create database connection using the same URL pattern as the runner
    let db_url = format!(
        "sqlite://{}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        runner_config.config.db_path
    );

    let database = Database::new(&db_url, "", 5);

    // Create the storage backend and DAL
    let storage = Arc::new(SqliteRegistryStorage::new(database.clone()));
    let dal = DAL::new(database);
    let mut workflow_registry = dal.workflow_registry(storage);

    // Use the high-level register_workflow_package method
    let package_id = workflow_registry
        .register_workflow_package(package_data)
        .await
        .map_err(|e| format!("Failed to register workflow package: {}", e))?;

    // Get package info from the registered package
    let package_info = workflow_registry
        .get_workflow_package_by_id(package_id)
        .await
        .map_err(|e| format!("Failed to get package info: {}", e))?;

    let (package_metadata, _) =
        package_info.ok_or_else(|| "Package was registered but cannot be retrieved".to_string())?;

    Ok(json!({
        "success": true,
        "package_id": package_id,
        "runner_id": runner_id,
        "package_name": package_metadata.package_name,
        "version": package_metadata.version,
        "message": format!("Workflow package '{}' v{} registered successfully",
                          package_metadata.package_name, package_metadata.version)
    }))
}

/// List all registered workflow packages for a specific runner
#[command]
pub async fn list_workflow_packages(runner_id: String) -> Result<serde_json::Value, String> {
    use cloacina::dal::{SqliteRegistryStorage, DAL};
    use cloacina::Database;
    use serde_json::json;
    use std::sync::Arc;

    // Get runner config to create database connection
    let runner_records = {
        let runner_service_guard = RUNNER_SERVICE
            .lock()
            .map_err(|e| format!("Failed to acquire runner service lock: {}", e))?;
        let runner_service = runner_service_guard
            .as_ref()
            .ok_or_else(|| "Runner service not initialized".to_string())?;
        runner_service
            .get_all_runners(&{
                let state = APP_STATE
                    .lock()
                    .map_err(|e| format!("Failed to acquire app state lock: {}", e))?;
                state.runners.clone()
            })
            .map_err(|e| format!("Failed to get runner configurations: {}", e))?
    };

    let runner_config = runner_records
        .iter()
        .find(|r| r.id == runner_id)
        .ok_or_else(|| "Runner configuration not found".to_string())?;

    // Create database connection using the same URL pattern as the runner
    let db_url = format!(
        "sqlite://{}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        runner_config.config.db_path
    );

    let database = Database::new(&db_url, "", 5);

    // Create the storage backend and DAL
    let storage = Arc::new(SqliteRegistryStorage::new(database.clone()));
    let dal = DAL::new(database);
    let workflow_registry = dal.workflow_registry(storage);

    // List packages using the high-level workflow registry
    let workflows = workflow_registry
        .list_packages()
        .await
        .map_err(|e| format!("Failed to list packages: {}", e))?;

    // Convert WorkflowPackageInfo to a serializable format
    let serializable_workflows: Vec<serde_json::Value> = workflows
        .into_iter()
        .map(|info| {
            json!({
                "id": info.id,
                "registry_id": info.registry_id,
                "package_name": info.package_name,
                "version": info.version,
                "description": info.description,
                "author": info.author,
                "created_at": info.created_at,
                "updated_at": info.updated_at
            })
        })
        .collect();

    Ok(json!({
        "success": true,
        "runner_id": runner_id,
        "workflows": serializable_workflows
    }))
}

/// Unregister a workflow package from a specific runner's registry
#[command]
pub async fn unregister_workflow_package(
    runner_id: String,
    package_name: String,
    version: String,
) -> Result<serde_json::Value, String> {
    use cloacina::dal::{SqliteRegistryStorage, DAL};
    use cloacina::Database;
    use serde_json::json;
    use std::sync::Arc;

    // Get runner config to create database connection
    let runner_records = {
        let runner_service_guard = RUNNER_SERVICE
            .lock()
            .map_err(|e| format!("Failed to acquire runner service lock: {}", e))?;
        let runner_service = runner_service_guard
            .as_ref()
            .ok_or_else(|| "Runner service not initialized".to_string())?;
        runner_service
            .get_all_runners(&{
                let state = APP_STATE
                    .lock()
                    .map_err(|e| format!("Failed to acquire app state lock: {}", e))?;
                state.runners.clone()
            })
            .map_err(|e| format!("Failed to get runner configurations: {}", e))?
    };

    let runner_config = runner_records
        .iter()
        .find(|r| r.id == runner_id)
        .ok_or_else(|| "Runner configuration not found".to_string())?;

    // Create database connection using the same URL pattern as the runner
    let db_url = format!(
        "sqlite://{}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        runner_config.config.db_path
    );

    let database = Database::new(&db_url, "", 5);

    // Create the storage backend and DAL
    let storage = Arc::new(SqliteRegistryStorage::new(database.clone()));
    let dal = DAL::new(database);
    let mut workflow_registry = dal.workflow_registry(storage);

    // First get the package by name and version to get its ID
    let package_info = workflow_registry
        .get_workflow_package_by_name(&package_name, &version)
        .await
        .map_err(|e| format!("Failed to get package info: {}", e))?;

    let (package_metadata, _) =
        package_info.ok_or_else(|| format!("Package {}:{} not found", package_name, version))?;

    // Unregister using the package name instead of ID
    workflow_registry
        .unregister_workflow_package_by_name(
            &package_metadata.package_name,
            &package_metadata.version,
        )
        .await
        .map_err(|e| format!("Failed to unregister package: {}", e))?;

    Ok(json!({
        "success": true,
        "runner_id": runner_id,
        "message": format!("Workflow {}/{} unregistered successfully from runner {}", package_name, version, runner_id)
    }))
}

/// Execute a workflow from a registered package
#[command]
pub async fn execute_workflow(
    runner_id: String,
    package_name: String,
    workflow_name: String,
    context: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    use cloacina::{Context, PipelineExecutor};
    use serde_json::json;

    // Get the specific runner to access its execution functionality
    let runner = {
        let state = APP_STATE
            .lock()
            .map_err(|e| format!("Failed to acquire app state lock: {}", e))?;
        state
            .runners
            .get(&runner_id)
            .ok_or_else(|| format!("Runner {} not found", runner_id))?
            .clone()
    };

    // Execute the workflow
    let mut exec_context = Context::<serde_json::Value>::new();
    let context_data = context.unwrap_or(json!({}));

    // Insert the provided context data
    if let serde_json::Value::Object(map) = context_data {
        for (key, value) in map {
            exec_context
                .insert(key, value)
                .map_err(|e| format!("Failed to insert context data: {}", e))?;
        }
    }

    match runner.execute(&workflow_name, exec_context).await {
        Ok(result) => Ok(json!({
            "success": true,
            "runner_id": runner_id,
            "result": {
                "execution_id": result.execution_id,
                "status": format!("{:?}", result.status),
                "workflow_name": result.workflow_name
            },
            "package_name": package_name,
            "workflow_name": workflow_name,
            "message": format!("Workflow '{}' from package '{}' executed successfully", workflow_name, package_name)
        })),
        Err(e) => Err(format!("Failed to execute workflow: {}", e)),
    }
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
