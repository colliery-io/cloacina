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
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::Instrument;
use uuid::Uuid;

use super::dal::RunnerDAL;
use super::types::{LocalRunnerStatus, RunnerConfig, RunnerRecord, StoredRunnerConfig};
use crate::domains::app::logging::{create_runner_logger, remove_runner_logger};

pub struct RunnerService {
    dal: RunnerDAL,
}

impl RunnerService {
    pub fn new(dal: RunnerDAL) -> Self {
        tracing::debug!("Creating new RunnerService");
        Self { dal }
    }

    pub async fn create_runner(
        &self,
        config: RunnerConfig,
    ) -> Result<(String, Arc<DefaultRunner>, LocalRunnerStatus), String> {
        // Generate unique ID first
        let runner_id = Uuid::new_v4().to_string();

        let span = tracing::info_span!(
            "create_runner",
            runner_id = %runner_id,
            runner_name = %config.name,
            operation = "create"
        );

        async move {
            tracing::info!("Creating new runner");
            tracing::debug!("Generated runner ID: {}", runner_id);

            // Convert to stored config
            let stored_config: StoredRunnerConfig = config.into();

            // Save to database first
            let record = RunnerRecord {
                id: runner_id.clone(),
                config: stored_config.clone(),
                is_paused: false,
            };

            tracing::debug!("Saving runner to database");
            self.dal.save_runner(&record).map_err(|e| {
                tracing::error!("Failed to save runner to database: {}", e);
                format!("Failed to save runner: {}", e)
            })?;

            // Create runner-specific logger
            if let Err(e) = create_runner_logger(&runner_id, &stored_config.name) {
                tracing::warn!("Failed to create runner logger: {}", e);
            }

            // Start the runner
            tracing::debug!("Starting runner instance");
            let runner = Self::start_runner_instance(&stored_config, &runner_id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to start runner instance: {}", e);
                    e
                })?;

            let status = LocalRunnerStatus {
                id: runner_id.clone(),
                running: true,
                is_paused: false,
                config: stored_config,
                message: "Runner created and started successfully".to_string(),
            };

            tracing::info!("Runner created and started successfully");
            Ok((runner_id, runner, status))
        }
        .instrument(span)
        .await
    }

    pub fn get_all_runners(
        &self,
        active_runners: &HashMap<String, Arc<DefaultRunner>>,
    ) -> Result<Vec<LocalRunnerStatus>, String> {
        tracing::debug!("Getting all runners from database and active runners map");
        let records = self.dal.get_all_runners().map_err(|e| {
            tracing::error!("Failed to load runners from database: {}", e);
            format!("Failed to load runners: {}", e)
        })?;

        let mut statuses = Vec::new();
        for record in records {
            let running = active_runners.contains_key(&record.id);
            tracing::debug!(
                "Runner {} status: running={}, is_paused={}",
                record.id,
                running,
                record.is_paused
            );
            statuses.push(LocalRunnerStatus {
                id: record.id,
                running,
                is_paused: record.is_paused,
                config: record.config,
                message: if running {
                    "Running"
                } else if record.is_paused {
                    "Paused"
                } else {
                    "Stopped"
                }
                .to_string(),
            });
        }

        tracing::debug!("Returning {} runner statuses", statuses.len());
        Ok(statuses)
    }

    pub async fn start_runner(
        &self,
        runner_id: String,
        active_runners: &HashMap<String, Arc<DefaultRunner>>,
    ) -> Result<(String, Arc<DefaultRunner>, LocalRunnerStatus), String> {
        let span = tracing::info_span!(
            "start_runner",
            runner_id = %runner_id,
            operation = "start"
        );

        async move {
            tracing::info!("Starting runner");

            // Check if already running
            if active_runners.contains_key(&runner_id) {
                tracing::warn!("Attempted to start runner that is already running");
                return Err("Runner is already running".to_string());
            }

            // Get runner config from database
            tracing::debug!("Loading runner config from database");
            let records = self.dal.get_all_runners().map_err(|e| {
                tracing::error!(
                    "Failed to load runners from database for start operation: {}",
                    e
                );
                format!("Failed to load runners: {}", e)
            })?;

            let record = records
                .iter()
                .find(|r| r.id == runner_id)
                .ok_or_else(|| {
                    tracing::error!("Runner not found in database");
                    "Runner not found".to_string()
                })?
                .clone();

            // Ensure runner logger exists
            if let Err(e) = create_runner_logger(&runner_id, &record.config.name) {
                tracing::warn!("Failed to create runner logger: {}", e);
            }

            // Start the runner
            tracing::debug!("Starting runner instance");
            let runner = Self::start_runner_instance(&record.config, &runner_id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to start runner instance: {}", e);
                    e
                })?;

            // Update database to mark as not paused
            tracing::debug!("Updating runner status to not paused");
            self.dal
                .update_runner_status(&runner_id, false)
                .map_err(|e| {
                    tracing::error!("Failed to update runner status: {}", e);
                    format!("Failed to update status: {}", e)
                })?;

            let status = LocalRunnerStatus {
                id: runner_id.clone(),
                running: true,
                is_paused: false,
                config: record.config,
                message: "Runner started successfully".to_string(),
            };

            tracing::info!("Runner started successfully");
            Ok((runner_id, runner, status))
        }
        .instrument(span)
        .await
    }

    pub async fn stop_runner(
        &self,
        runner_id: String,
        runner_arc: Arc<DefaultRunner>,
    ) -> Result<LocalRunnerStatus, String> {
        let span = tracing::info_span!(
            "stop_runner",
            runner_id = %runner_id,
            operation = "stop"
        );

        async move {
            tracing::info!("Stopping runner");

            // Get config first
            let record = {
                tracing::debug!("Loading runner config for stop operation");
                let records = self.dal.get_all_runners().map_err(|e| {
                    tracing::error!(
                        "Failed to load runners from database for stop operation: {}",
                        e
                    );
                    format!("Failed to load runners: {}", e)
                })?;

                records
                    .iter()
                    .find(|r| r.id == runner_id)
                    .ok_or_else(|| {
                        tracing::error!("Runner not found in database for stop operation");
                        "Runner not found in database".to_string()
                    })?
                    .clone()
            };

            // Shutdown the runner
            tracing::debug!("Shutting down runner instance");
            match Arc::try_unwrap(runner_arc) {
                Ok(runner_instance) => {
                    runner_instance.shutdown().await.map_err(|e| {
                        tracing::error!("Failed to shutdown runner: {}", e);
                        format!("Shutdown error: {}", e)
                    })?;
                    tracing::debug!("Runner shutdown completed");
                }
                Err(_) => {
                    tracing::error!("Cannot stop runner: multiple references exist");
                    return Err("Cannot stop runner: multiple references exist".to_string());
                }
            }

            // Update database to mark as paused
            tracing::debug!("Updating runner status to paused");
            self.dal
                .update_runner_status(&runner_id, true)
                .map_err(|e| {
                    tracing::error!("Failed to update runner status to paused: {}", e);
                    format!("Failed to update status: {}", e)
                })?;

            tracing::info!("Runner stopped successfully");
            Ok(LocalRunnerStatus {
                id: runner_id,
                running: false,
                is_paused: true,
                config: record.config,
                message: "Runner stopped successfully".to_string(),
            })
        }
        .instrument(span)
        .await
    }

    pub async fn load_and_start_runners(
        &self,
    ) -> Result<(HashMap<String, Arc<DefaultRunner>>, usize, usize, usize), String> {
        tracing::info!("Loading and starting all non-paused runners from database");
        let runner_records = self.dal.get_all_runners().map_err(|e| {
            tracing::error!("Failed to load runners from database during startup: {}", e);
            format!("Failed to load runners: {}", e)
        })?;

        tracing::debug!("Found {} total runners in database", runner_records.len());

        let mut running_count = 0;
        let mut paused_count = 0;
        let mut active_runners = HashMap::new();

        for record in &runner_records {
            let runner_span = tracing::info_span!(
                "startup_runner",
                runner_id = %record.id,
                runner_name = %record.config.name,
                operation = "startup"
            );

            let _guard = runner_span.enter();

            // Ensure runner logger exists for all runners (active and paused)
            if let Err(e) = create_runner_logger(&record.id, &record.config.name) {
                tracing::warn!("Failed to create runner logger: {}", e);
            }

            if !record.is_paused {
                tracing::info!("Starting runner during startup");
                match Self::start_runner_instance(&record.config, &record.id).await {
                    Ok(runner) => {
                        active_runners.insert(record.id.clone(), runner);
                        running_count += 1;
                        tracing::debug!("Runner started successfully during startup");
                    }
                    Err(e) => {
                        tracing::error!("Failed to start runner during startup: {}", e);
                    }
                }
            } else {
                paused_count += 1;
                tracing::debug!("Skipping paused runner during startup");
            }
        }

        tracing::info!(
            "Startup complete: {} total runners, {} running, {} paused",
            runner_records.len(),
            running_count,
            paused_count
        );
        Ok((
            active_runners,
            runner_records.len(),
            running_count,
            paused_count,
        ))
    }

    pub async fn delete_runner(&self, runner_id: &str) -> Result<(), String> {
        tracing::info!("Deleting runner from database: {}", runner_id);

        // Remove runner logger
        if let Err(e) = remove_runner_logger(runner_id) {
            tracing::warn!("Failed to remove runner logger for {}: {}", runner_id, e);
        }

        self.dal.delete_runner(runner_id).map_err(|e| {
            tracing::error!("Failed to delete runner {} from database: {}", runner_id, e);
            format!("Failed to delete runner: {}", e)
        })
    }

    async fn start_runner_instance(
        config: &StoredRunnerConfig,
        runner_id: &str,
    ) -> Result<Arc<DefaultRunner>, String> {
        let span = tracing::info_span!(
            "runner",
            runner_id = %runner_id,
            runner_name = %config.name,
            db_path = %config.db_path,
            component = "cloacina_runner"
        );

        // Use instrument() to ensure ALL async operations inherit this span
        async move {
            let db_url = format!(
                "sqlite://{}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
                config.db_path
            );

            tracing::info!(
                "Creating runner instance - max_tasks: {}, cron: {}, registry: {}",
                config.max_concurrent_tasks,
                config.enable_cron_scheduling,
                config.enable_registry_reconciler
            );

            let mut runner_config = DefaultRunnerConfig::default();
            runner_config.max_concurrent_tasks = config.max_concurrent_tasks as usize;
            runner_config.enable_cron_scheduling = config.enable_cron_scheduling;
            runner_config.enable_registry_reconciler = config.enable_registry_reconciler;
            // Always use SQLite registry storage for cloacina-app
            runner_config.registry_storage_backend = "sqlite".to_string();

            // Pass runner context for logging
            runner_config.runner_id = Some(runner_id.to_string());
            runner_config.runner_name = Some(config.name.clone());

            // This is the critical part - DefaultRunner creation and ALL its internal
            // async operations should inherit the current span context
            let runner = DefaultRunner::with_config(&db_url, runner_config)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create runner instance: {}", e);
                    format!("Failed to create runner: {}", e)
                })?;

            tracing::info!("Runner instance created successfully");
            Ok(Arc::new(runner))
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_dal() -> (RunnerDAL, tempfile::TempDir) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let dal = RunnerDAL::new(db_path.to_str().unwrap()).unwrap();
        (dal, temp_dir)
    }

    fn create_test_config() -> RunnerConfig {
        RunnerConfig {
            name: "Test Runner".to_string(),
            db_path: "./test.db".to_string(),
            max_concurrent_tasks: 2,
            enable_cron_scheduling: false,
            enable_registry_reconciler: true,
        }
    }

    #[tokio::test]
    async fn test_runner_service_creation() {
        let (dal, _temp_dir) = create_test_dal();
        let _service = RunnerService::new(dal);

        // Service should be created successfully
        // This is a simple smoke test
        assert!(true);
    }

    #[tokio::test]
    async fn test_get_all_runners_empty() {
        let (dal, _temp_dir) = create_test_dal();
        let service = RunnerService::new(dal);
        let active_runners = HashMap::new();

        let result = service.get_all_runners(&active_runners);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_runner() {
        let (dal, _temp_dir) = create_test_dal();
        let service = RunnerService::new(dal);

        let result = service.delete_runner("nonexistent").await;
        // Should succeed even if runner doesn't exist
        assert!(result.is_ok());
    }

    #[test]
    fn test_runner_config_conversion() {
        let config = create_test_config();
        let stored_config: StoredRunnerConfig = config.into();

        assert_eq!(stored_config.name, "Test Runner");
        assert_eq!(stored_config.db_path, "./test.db");
        assert_eq!(stored_config.max_concurrent_tasks, 2);
        assert!(!stored_config.enable_cron_scheduling);
        assert!(stored_config.enable_registry_reconciler);
    }
}
