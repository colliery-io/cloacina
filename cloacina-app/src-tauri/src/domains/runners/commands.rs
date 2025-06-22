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

use super::types::{LocalRunnerStatus, RunnerConfig};
use crate::domains::app::state::{APP_STATE, RUNNER_SERVICE};
use std::collections::HashMap;
use std::sync::Arc;

// Create a new local runner
#[tauri::command]
pub async fn create_runner(config: RunnerConfig) -> Result<LocalRunnerStatus, String> {
    tracing::info!("Creating new runner with name: {}", config.name);
    tracing::debug!(
        "Runner config: db_path={}, max_tasks={}, cron={}, registry={}",
        config.db_path,
        config.max_concurrent_tasks,
        config.enable_cron_scheduling,
        config.enable_registry_reconciler
    );

    // Get the service
    let runner_service = {
        let service_guard = RUNNER_SERVICE.lock().map_err(|e| {
            tracing::error!("Failed to acquire runner service lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        service_guard
            .as_ref()
            .ok_or_else(|| {
                tracing::error!("Runner service not initialized");
                "Application not initialized"
            })?
            .clone()
    };

    // Create the runner (without holding any locks)
    let (runner_id, runner, status) = runner_service.create_runner(config).await.map_err(|e| {
        tracing::error!("Failed to create runner: {}", e);
        e
    })?;

    tracing::info!("Runner created successfully with ID: {}", runner_id);

    // Now add it to the active runners
    {
        let mut state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        state.runners.insert(runner_id.clone(), runner);
        tracing::debug!(
            "Added runner {} to active runners, total active: {}",
            runner_id,
            state.runners.len()
        );
    }

    Ok(status)
}

// Get all local runners
#[tauri::command]
pub async fn get_local_runners() -> Result<Vec<LocalRunnerStatus>, String> {
    tracing::debug!("Fetching all local runners");

    // Get the service
    let runner_service = {
        let service_guard = RUNNER_SERVICE.lock().map_err(|e| {
            tracing::error!("Failed to acquire runner service lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        service_guard
            .as_ref()
            .ok_or_else(|| {
                tracing::error!("Runner service not initialized when fetching runners");
                "Application not initialized"
            })?
            .clone()
    };

    // Get the current runners
    let runners = {
        let state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        state.runners.clone()
    };

    let result = runner_service.get_all_runners(&runners).map_err(|e| {
        tracing::error!("Failed to get runners from service: {}", e);
        e
    })?;

    tracing::debug!("Retrieved {} runners", result.len());
    Ok(result)
}

// Start a specific runner
#[tauri::command]
pub async fn start_local_runner(runner_id: String) -> Result<LocalRunnerStatus, String> {
    tracing::info!("Starting runner: {}", runner_id);

    // Get the service
    let runner_service = {
        let service_guard = RUNNER_SERVICE.lock().map_err(|e| {
            tracing::error!("Failed to acquire runner service lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        service_guard
            .as_ref()
            .ok_or_else(|| {
                tracing::error!(
                    "Runner service not initialized when starting runner: {}",
                    runner_id
                );
                "Application not initialized"
            })?
            .clone()
    };

    // Get current runners for checking
    let runners = {
        let state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        state.runners.clone()
    };

    tracing::debug!("Current active runners: {}", runners.len());

    // Start the runner
    let (new_runner_id, runner, status) = runner_service
        .start_runner(runner_id.clone(), &runners)
        .await
        .map_err(|e| {
            tracing::error!("Failed to start runner {}: {}", runner_id, e);
            e
        })?;

    tracing::info!("Runner {} started successfully", new_runner_id);

    // Now add it to the active runners
    {
        let mut state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        state.runners.insert(new_runner_id.clone(), runner);
        tracing::debug!(
            "Added runner {} to active runners, total active: {}",
            new_runner_id,
            state.runners.len()
        );
    }

    Ok(status)
}

// Stop a specific runner
#[tauri::command]
pub async fn stop_local_runner(runner_id: String) -> Result<LocalRunnerStatus, String> {
    tracing::info!("Stopping runner: {}", runner_id);

    // Get the service
    let runner_service = {
        let service_guard = RUNNER_SERVICE.lock().map_err(|e| {
            tracing::error!("Failed to acquire runner service lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        service_guard
            .as_ref()
            .ok_or_else(|| {
                tracing::error!(
                    "Runner service not initialized when stopping runner: {}",
                    runner_id
                );
                "Application not initialized"
            })?
            .clone()
    };

    // Remove the runner from active runners
    let runner_arc = {
        let mut state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock: {}", e);
            format!("Lock error: {}", e)
        })?;

        tracing::debug!(
            "Current active runners: {:?}",
            state.runners.keys().collect::<Vec<_>>()
        );

        state.runners.remove(&runner_id).ok_or_else(|| {
            tracing::warn!(
                "Runner {} not found in active runners - may already be stopped",
                runner_id
            );
            "Runner is not running".to_string()
        })?
    };

    tracing::debug!(
        "Runner {} found and removed from active runners, proceeding to stop",
        runner_id
    );

    // Stop the runner (without holding any locks)
    let result = runner_service
        .stop_runner(runner_id.clone(), runner_arc)
        .await
        .map_err(|e| {
            tracing::error!("Failed to stop runner {}: {}", runner_id, e);
            e
        })?;

    tracing::info!("Runner {} stopped successfully", runner_id);
    Ok(result)
}

// Delete a runner
#[tauri::command]
pub async fn delete_runner(runner_id: String, delete_database: bool) -> Result<(), String> {
    tracing::info!(
        "Deleting runner: {}, delete_database: {}",
        runner_id,
        delete_database
    );

    // Get the service
    let runner_service = {
        let service_guard = RUNNER_SERVICE.lock().map_err(|e| {
            tracing::error!("Failed to acquire runner service lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        service_guard
            .as_ref()
            .ok_or_else(|| {
                tracing::error!(
                    "Runner service not initialized when deleting runner: {}",
                    runner_id
                );
                "Application not initialized"
            })?
            .clone()
    };

    // Check if runner is running and remove it if so
    let runner_to_stop = {
        let mut state = APP_STATE.lock().map_err(|e| {
            tracing::error!("Failed to acquire app state lock: {}", e);
            format!("Lock error: {}", e)
        })?;
        state.runners.remove(&runner_id)
    };

    if let Some(runner_arc) = runner_to_stop {
        tracing::info!(
            "Runner {} was running, stopping it before deletion",
            runner_id
        );
        // Try to stop the runner gracefully
        match Arc::try_unwrap(runner_arc) {
            Ok(runner_instance) => {
                runner_instance.shutdown().await.map_err(|e| {
                    tracing::error!("Failed to shutdown runner {}: {}", runner_id, e);
                    format!("Failed to shutdown runner: {}", e)
                })?;
                tracing::debug!("Runner {} shutdown successfully", runner_id);
            }
            Err(_) => {
                tracing::error!(
                    "Cannot delete runner {}: multiple references exist",
                    runner_id
                );
                return Err("Cannot delete runner: multiple references exist".to_string());
            }
        }
    } else {
        tracing::debug!(
            "Runner {} was not running, proceeding with deletion",
            runner_id
        );
    }

    // Get runner info before deleting (for database path if needed)
    let db_path = if delete_database {
        tracing::debug!(
            "Getting database path for runner {} before deletion",
            runner_id
        );
        let runners = runner_service
            .get_all_runners(&HashMap::new())
            .map_err(|e| {
                tracing::error!("Failed to get runners for database path lookup: {}", e);
                e
            })?;
        runners.iter().find(|r| r.id == runner_id).map(|r| {
            tracing::debug!(
                "Found database path for runner {}: {}",
                runner_id,
                r.config.db_path
            );
            r.config.db_path.clone()
        })
    } else {
        None
    };

    // Delete from database
    tracing::info!("Deleting runner {} from database", runner_id);
    runner_service
        .delete_runner(&runner_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete runner {} from database: {}", runner_id, e);
            e
        })?;

    // Delete the database file if requested
    if let Some(path) = db_path {
        if std::path::Path::new(&path).exists() {
            tracing::info!("Deleting database file: {}", path);
            std::fs::remove_file(&path).map_err(|e| {
                tracing::error!("Failed to delete database file {}: {}", path, e);
                format!("Failed to delete database file: {}", e)
            })?;
            tracing::info!("Database file deleted successfully: {}", path);
        } else {
            tracing::warn!("Database file {} does not exist, skipping deletion", path);
        }
    } else {
        tracing::debug!("Database deletion not requested for runner {}", runner_id);
    }

    tracing::info!("Runner {} deleted successfully", runner_id);
    Ok(())
}
