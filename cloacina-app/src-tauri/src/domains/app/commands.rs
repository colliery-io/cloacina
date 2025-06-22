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
