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

//! Background service management for the DefaultRunner.
//!
//! This module handles starting and managing background services including
//! the scheduler, executor, cron scheduler, cron recovery, and registry reconciler.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tracing::Instrument;

use crate::dal::FilesystemRegistryStorage;
use crate::dal::UnifiedRegistryStorage;
use crate::dal::DAL;
use crate::executor::pipeline_executor::PipelineError;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::{ReconcilerConfig, RegistryReconciler, WorkflowRegistryImpl};
use crate::{CronScheduler, CronSchedulerConfig};
use crate::{TriggerScheduler, TriggerSchedulerConfig};

use super::DefaultRunner;

impl DefaultRunner {
    /// Creates a tracing span for this runner instance with proper context
    pub(super) fn create_runner_span(&self, operation: &str) -> tracing::Span {
        if let (Some(runner_id), Some(runner_name)) =
            (&self.config.runner_id, &self.config.runner_name)
        {
            tracing::info_span!(
                "runner_task",
                runner_id = %runner_id,
                runner_name = %runner_name,
                operation = operation,
                component = "cloacina_runner"
            )
        } else {
            tracing::info_span!(
                "runner_task",
                operation = operation,
                component = "cloacina_runner"
            )
        }
    }

    /// Starts the background scheduler and executor services
    ///
    /// This method:
    /// 1. Creates shutdown channels for graceful termination
    /// 2. Spawns the scheduler background task
    /// 3. Spawns the executor background task
    /// 4. Stores the runtime handles for later shutdown
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error status
    pub(super) async fn start_background_services(&self) -> Result<(), PipelineError> {
        let mut handles = self.runtime_handles.write().await;

        tracing::info!("Starting scheduler and executor background services");

        // Create shutdown channel
        let (shutdown_tx, mut scheduler_shutdown_rx) = broadcast::channel(1);
        let executor_shutdown_rx = shutdown_tx.subscribe();

        // Start scheduler (dispatcher mode: scheduler pushes tasks to executor via dispatcher)
        let scheduler = self.scheduler.clone();
        let scheduler_span = self.create_runner_span("task_scheduler");
        let scheduler_handle = tokio::spawn(
            async move {
                let mut scheduler_future = Box::pin(scheduler.run_scheduling_loop());

                tokio::select! {
                    result = &mut scheduler_future => {
                        if let Err(e) = result {
                            tracing::error!("Scheduler loop failed: {}", e);
                        } else {
                            tracing::info!("Scheduler loop completed");
                        }
                    }
                    _ = scheduler_shutdown_rx.recv() => {
                        tracing::info!("Scheduler shutdown requested");
                    }
                }
            }
            .instrument(scheduler_span),
        );

        // Note: executor polling loop is NOT started - dispatcher pushes tasks directly
        // to executor via TaskExecutor::execute(). The executor_shutdown_rx is kept for
        // potential future use with hybrid modes.
        drop(executor_shutdown_rx);

        // Store handles
        handles.scheduler_handle = Some(scheduler_handle);
        handles.executor_handle = None; // No polling loop in dispatcher mode
        handles.shutdown_sender = Some(shutdown_tx.clone());

        // Start cron services if enabled
        if self.config.enable_cron_scheduling {
            self.start_cron_services(&mut handles, &shutdown_tx).await?;
        }

        // Start registry reconciler if enabled
        if self.config.enable_registry_reconciler {
            self.start_registry_reconciler(&mut handles, &shutdown_tx)
                .await?;
        }

        // Start trigger scheduler if enabled
        if self.config.enable_trigger_scheduling {
            self.start_trigger_services(&mut handles, &shutdown_tx)
                .await?;
        }

        Ok(())
    }

    /// Starts cron scheduler and recovery services
    async fn start_cron_services(
        &self,
        handles: &mut super::RuntimeHandles,
        shutdown_tx: &broadcast::Sender<()>,
    ) -> Result<(), PipelineError> {
        tracing::info!("Starting cron scheduler");

        // Create watch channel for cron scheduler shutdown
        let (cron_shutdown_tx, cron_shutdown_rx) = watch::channel(false);

        // Create cron scheduler config
        let cron_config = CronSchedulerConfig {
            poll_interval: self.config.cron_poll_interval,
            max_catchup_executions: self.config.cron_max_catchup_executions,
            max_acceptable_delay: Duration::from_secs(300), // 5 minutes
        };

        // Create CronScheduler with DefaultRunner as PipelineExecutor
        let dal = DAL::new(self.database.clone());
        let cron_scheduler = CronScheduler::new(
            Arc::new(dal),
            Arc::new(self.clone()), // self implements PipelineExecutor!
            cron_config,
            cron_shutdown_rx,
        );

        // Start cron background service
        let mut cron_scheduler_clone = cron_scheduler.clone();
        let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
        let cron_span = self.create_runner_span("cron_scheduler");
        let cron_handle = tokio::spawn(
            async move {
                tokio::select! {
                    result = cron_scheduler_clone.run_polling_loop() => {
                        if let Err(e) = result {
                            tracing::error!("Cron scheduler failed: {}", e);
                        } else {
                            tracing::info!("Cron scheduler completed");
                        }
                    }
                    _ = broadcast_shutdown_rx.recv() => {
                        tracing::info!("Cron scheduler shutdown requested via broadcast");
                        // Send shutdown signal to cron scheduler
                        let _ = cron_shutdown_tx.send(true);
                    }
                }
            }
            .instrument(cron_span),
        );

        // Store cron scheduler and handle
        *self.cron_scheduler.write().await = Some(Arc::new(cron_scheduler));
        handles.cron_scheduler_handle = Some(cron_handle);

        // Start cron recovery service if enabled
        if self.config.cron_enable_recovery {
            self.start_cron_recovery(handles, shutdown_tx).await?;
        }

        Ok(())
    }

    /// Starts the cron recovery service
    async fn start_cron_recovery(
        &self,
        handles: &mut super::RuntimeHandles,
        shutdown_tx: &broadcast::Sender<()>,
    ) -> Result<(), PipelineError> {
        tracing::info!("Starting cron recovery service");

        // Create watch channel for recovery service shutdown
        let (recovery_shutdown_tx, recovery_shutdown_rx) = watch::channel(false);

        // Create recovery config
        let recovery_config = crate::CronRecoveryConfig {
            check_interval: self.config.cron_recovery_interval,
            lost_threshold_minutes: self.config.cron_lost_threshold_minutes,
            max_recovery_age: self.config.cron_max_recovery_age,
            max_recovery_attempts: self.config.cron_max_recovery_attempts,
            recover_disabled_schedules: false,
        };

        // Create CronRecoveryService
        let dal = DAL::new(self.database.clone());
        let recovery_service = crate::CronRecoveryService::new(
            Arc::new(dal),
            Arc::new(self.clone()), // self implements PipelineExecutor!
            recovery_config,
            recovery_shutdown_rx,
        );

        // Start recovery background service
        let mut recovery_service_clone = recovery_service.clone();
        let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
        let recovery_span = self.create_runner_span("cron_recovery");
        let recovery_handle = tokio::spawn(
            async move {
                tokio::select! {
                    result = recovery_service_clone.run_recovery_loop() => {
                        if let Err(e) = result {
                            tracing::error!("Cron recovery service failed: {}", e);
                        } else {
                            tracing::info!("Cron recovery service completed");
                        }
                    }
                    _ = broadcast_shutdown_rx.recv() => {
                        tracing::info!("Cron recovery service shutdown requested via broadcast");
                        // Send shutdown signal to recovery service
                        let _ = recovery_shutdown_tx.send(true);
                    }
                }
            }
            .instrument(recovery_span),
        );

        // Store recovery service and handle
        *self.cron_recovery.write().await = Some(Arc::new(recovery_service));
        handles.cron_recovery_handle = Some(recovery_handle);

        Ok(())
    }

    /// Starts the registry reconciler service
    async fn start_registry_reconciler(
        &self,
        handles: &mut super::RuntimeHandles,
        shutdown_tx: &broadcast::Sender<()>,
    ) -> Result<(), PipelineError> {
        tracing::info!("Starting registry reconciler");

        // Create watch channel for registry reconciler shutdown
        let (reconciler_shutdown_tx, reconciler_shutdown_rx) = watch::channel(false);

        // Create reconciler config
        let reconciler_config = ReconcilerConfig {
            reconcile_interval: self.config.registry_reconcile_interval,
            enable_startup_reconciliation: self.config.registry_enable_startup_reconciliation,
            package_operation_timeout: Duration::from_secs(30),
            continue_on_package_error: true,
            default_tenant_id: "public".to_string(),
        };

        // Create storage backend based on configuration
        let workflow_registry_result = match self.config.registry_storage_backend.as_str() {
            "filesystem" => {
                let storage_path = self
                    .config
                    .registry_storage_path
                    .clone()
                    .unwrap_or_else(|| std::env::temp_dir().join("cloacina_registry"));

                match FilesystemRegistryStorage::new(storage_path) {
                    Ok(storage) => WorkflowRegistryImpl::new(storage, self.database.clone())
                        .map(|registry| Arc::new(registry) as Arc<dyn WorkflowRegistry>)
                        .map_err(|e| {
                            format!("Failed to create filesystem workflow registry: {}", e)
                        }),
                    Err(e) => Err(format!("Failed to create filesystem storage: {}", e)),
                }
            }
            "sqlite" | "postgres" | "database" => {
                let dal = crate::dal::DAL::new(self.database.clone());
                let storage = UnifiedRegistryStorage::new(self.database.clone());
                let registry_dal = dal.workflow_registry(storage);
                Ok(Arc::new(registry_dal) as Arc<dyn WorkflowRegistry>)
            }
            backend => Err(format!(
                "Unknown registry storage backend: {}. Valid options: filesystem, sqlite, postgres, database",
                backend
            )),
        };

        match workflow_registry_result {
            Ok(workflow_registry_arc) => {
                // Create Registry Reconciler
                let registry_reconciler = RegistryReconciler::new(
                    workflow_registry_arc.clone(),
                    reconciler_config,
                    reconciler_shutdown_rx,
                )
                .map_err(|e| PipelineError::Configuration {
                    message: format!("Failed to create registry reconciler: {}", e),
                })?;

                // Start reconciler background service
                let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
                let reconciler_span = self.create_runner_span("registry_reconciler");
                let reconciler_handle = tokio::spawn(
                    async move {
                        tokio::select! {
                            result = registry_reconciler.start_reconciliation_loop() => {
                                if let Err(e) = result {
                                    tracing::error!("Registry reconciler failed: {}", e);
                                } else {
                                    tracing::info!("Registry reconciler completed");
                                }
                            }
                            _ = broadcast_shutdown_rx.recv() => {
                                tracing::info!("Registry reconciler shutdown requested via broadcast");
                                // Send shutdown signal to reconciler
                                let _ = reconciler_shutdown_tx.send(true);
                            }
                        }
                    }
                    .instrument(reconciler_span),
                );

                // Store workflow registry and reconciler
                *self.workflow_registry.write().await = Some(workflow_registry_arc);
                handles.registry_reconciler_handle = Some(reconciler_handle);
            }
            Err(e) => {
                tracing::error!("Failed to create workflow registry: {}", e);
            }
        }

        Ok(())
    }

    /// Starts the trigger scheduler service
    async fn start_trigger_services(
        &self,
        handles: &mut super::RuntimeHandles,
        shutdown_tx: &broadcast::Sender<()>,
    ) -> Result<(), PipelineError> {
        tracing::info!("Starting trigger scheduler");

        // Create watch channel for trigger scheduler shutdown
        let (trigger_shutdown_tx, trigger_shutdown_rx) = watch::channel(false);

        // Create trigger scheduler config
        let trigger_config = TriggerSchedulerConfig {
            base_poll_interval: self.config.trigger_base_poll_interval,
            poll_timeout: self.config.trigger_poll_timeout,
        };

        // Create TriggerScheduler with DefaultRunner as PipelineExecutor
        let dal = DAL::new(self.database.clone());
        let trigger_scheduler = TriggerScheduler::new(
            Arc::new(dal),
            Arc::new(self.clone()), // self implements PipelineExecutor!
            trigger_config,
            trigger_shutdown_rx,
        );

        // Start trigger scheduler background service
        let mut trigger_scheduler_clone = trigger_scheduler.clone();
        let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
        let trigger_span = self.create_runner_span("trigger_scheduler");
        let trigger_handle = tokio::spawn(
            async move {
                tokio::select! {
                    result = trigger_scheduler_clone.run_polling_loop() => {
                        if let Err(e) = result {
                            tracing::error!("Trigger scheduler failed: {}", e);
                        } else {
                            tracing::info!("Trigger scheduler completed");
                        }
                    }
                    _ = broadcast_shutdown_rx.recv() => {
                        tracing::info!("Trigger scheduler shutdown requested via broadcast");
                        // Send shutdown signal to trigger scheduler
                        let _ = trigger_shutdown_tx.send(true);
                    }
                }
            }
            .instrument(trigger_span),
        );

        // Store trigger scheduler and handle
        *self.trigger_scheduler.write().await = Some(Arc::new(trigger_scheduler));
        handles.trigger_scheduler_handle = Some(trigger_handle);

        Ok(())
    }
}
