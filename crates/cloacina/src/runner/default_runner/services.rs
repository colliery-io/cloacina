/*
 *  Copyright 2025-2026 Colliery Software
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

//! Background service construction and registration for the DefaultRunner.
//!
//! Each service is built here, wrapped in a [`BackgroundService`] adapter,
//! and registered with the runner's [`ServiceManager`]. The manager owns the
//! lifecycle from that point on.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

use crate::dal::FilesystemRegistryStorage;
use crate::dal::UnifiedRegistryStorage;
use crate::dal::DAL;
use crate::executor::workflow_executor::WorkflowExecutionError;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::{ReconcilerConfig, RegistryReconciler, WorkflowRegistryImpl};
use crate::{Scheduler, SchedulerConfig};

use super::service_manager::{
    CronRecoveryServiceWrapper, RegistryReconcilerService, ServiceManager,
    StaleClaimSweeperService, TaskSchedulerService, UnifiedSchedulerService,
};
use super::DefaultRunner;

impl DefaultRunner {
    /// Creates a tracing span for this runner instance with proper context
    pub(super) fn create_runner_span(&self, operation: &str) -> tracing::Span {
        if let (Some(runner_id), Some(runner_name)) =
            (self.config.runner_id(), self.config.runner_name())
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

    /// Constructs every enabled background service, registers them with the
    /// service manager, and starts them.
    pub(super) async fn start_background_services(&self) -> Result<(), WorkflowExecutionError> {
        tracing::info!("Starting background services");

        let mut manager = self.service_manager.write().await;

        // Always: per-runner task scheduler.
        manager.register(Box::new(TaskSchedulerService::new(
            self.scheduler.clone(),
            self.create_runner_span("task_scheduler"),
        )));

        // Unified scheduler covers both cron and trigger scheduling.
        if self.config.enable_cron_scheduling() || self.config.enable_trigger_scheduling() {
            self.register_unified_scheduler(&mut manager).await?;
        }

        // Cron recovery is gated by cron + recovery flags.
        if self.config.enable_cron_scheduling() && self.config.cron_enable_recovery() {
            self.register_cron_recovery(&mut manager).await?;
        }

        // Registry reconciler must be wired before the reactive scheduler is
        // installed externally.
        if self.config.enable_registry_reconciler() {
            self.register_registry_reconciler(&mut manager).await?;
        }

        // Stale-claim sweeper runs whenever push-with-claim is enabled.
        if self.config.enable_claiming() {
            self.register_stale_claim_sweeper(&mut manager).await?;
        }

        manager.start_all().await?;

        Ok(())
    }

    async fn register_unified_scheduler(
        &self,
        manager: &mut ServiceManager,
    ) -> Result<(), WorkflowExecutionError> {
        tracing::info!("Registering unified scheduler");

        let (inner_tx, inner_rx) = watch::channel(false);

        let scheduler_config = SchedulerConfig {
            cron_poll_interval: self.config.cron_poll_interval(),
            max_catchup_executions: self.config.cron_max_catchup_executions(),
            max_acceptable_delay: Duration::from_secs(300),
            trigger_base_poll_interval: self.config.trigger_base_poll_interval(),
            trigger_poll_timeout: self.config.trigger_poll_timeout(),
        };

        let dal = DAL::new(self.database.clone());
        let unified_scheduler = Scheduler::new(
            Arc::new(dal),
            Arc::new(self.clone()),
            scheduler_config,
            inner_rx,
            self.runtime.clone(),
        );
        let unified_scheduler = Arc::new(unified_scheduler);

        manager.unified_scheduler = Some(unified_scheduler.clone());
        manager.register(Box::new(UnifiedSchedulerService::new(
            unified_scheduler,
            inner_tx,
            self.create_runner_span("unified_scheduler"),
        )));

        Ok(())
    }

    async fn register_cron_recovery(
        &self,
        manager: &mut ServiceManager,
    ) -> Result<(), WorkflowExecutionError> {
        tracing::info!("Registering cron recovery service");

        let (inner_tx, inner_rx) = watch::channel(false);

        let recovery_config = crate::CronRecoveryConfig {
            check_interval: self.config.cron_recovery_interval(),
            lost_threshold_minutes: self.config.cron_lost_threshold_minutes(),
            max_recovery_age: self.config.cron_max_recovery_age(),
            max_recovery_attempts: self.config.cron_max_recovery_attempts(),
            recover_disabled_schedules: false,
        };

        let dal = DAL::new(self.database.clone());
        let recovery_service = crate::CronRecoveryService::new(
            Arc::new(dal),
            Arc::new(self.clone()),
            recovery_config,
            inner_rx,
        );
        let recovery_service = Arc::new(recovery_service);

        manager.cron_recovery = Some(recovery_service.clone());
        manager.register(Box::new(CronRecoveryServiceWrapper::new(
            recovery_service,
            inner_tx,
            self.create_runner_span("cron_recovery"),
        )));

        Ok(())
    }

    async fn register_registry_reconciler(
        &self,
        manager: &mut ServiceManager,
    ) -> Result<(), WorkflowExecutionError> {
        tracing::info!("Registering registry reconciler");

        let (inner_tx, inner_rx) = watch::channel(false);

        let reconciler_config = ReconcilerConfig {
            reconcile_interval: self.config.registry_reconcile_interval(),
            enable_startup_reconciliation: self.config.registry_enable_startup_reconciliation(),
            package_operation_timeout: Duration::from_secs(30),
            continue_on_package_error: true,
            default_tenant_id: "public".to_string(),
        };

        let workflow_registry_result = match self.config.registry_storage_backend() {
            "filesystem" => {
                let storage_path = self
                    .config
                    .registry_storage_path()
                    .map(|p| p.to_path_buf())
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
                let dal = DAL::new(self.database.clone());
                let storage = UnifiedRegistryStorage::new(self.database.clone());
                let registry_dal = dal.workflow_registry(storage);
                Ok(Arc::new(registry_dal) as Arc<dyn WorkflowRegistry>)
            }
            backend => Err(format!(
                "Unknown registry storage backend: {}. Valid options: filesystem, sqlite, postgres, database",
                backend
            )),
        };

        let workflow_registry_arc = match workflow_registry_result {
            Ok(arc) => arc,
            Err(e) => {
                tracing::error!("Failed to create workflow registry: {}", e);
                return Ok(());
            }
        };

        let mut registry_reconciler =
            RegistryReconciler::new(workflow_registry_arc.clone(), reconciler_config, inner_rx)
                .map_err(|e| WorkflowExecutionError::Configuration {
                    message: format!("Failed to create registry reconciler: {}", e),
                })?;

        // Share the manager's reactive scheduler slot so that a later
        // `set_reactive_scheduler()` is observable to the reconciler.
        registry_reconciler.set_reactive_scheduler_slot(manager.reactive_scheduler.clone());
        registry_reconciler = registry_reconciler.with_runtime(self.runtime.clone());

        manager.workflow_registry = Some(workflow_registry_arc);

        manager.register(Box::new(RegistryReconcilerService::new(
            registry_reconciler,
            inner_tx,
            self.create_runner_span("registry_reconciler"),
        )));

        Ok(())
    }

    async fn register_stale_claim_sweeper(
        &self,
        manager: &mut ServiceManager,
    ) -> Result<(), WorkflowExecutionError> {
        use crate::execution_planner::stale_claim_sweeper::{
            StaleClaimSweeper, StaleClaimSweeperConfig,
        };

        tracing::info!("Registering stale claim sweeper");

        let (inner_tx, inner_rx) = watch::channel(false);

        let sweeper_config = StaleClaimSweeperConfig {
            sweep_interval: self.config.stale_claim_sweep_interval(),
            stale_threshold: self.config.stale_claim_threshold(),
        };

        let dal = DAL::new(self.database.clone());
        let sweeper = StaleClaimSweeper::new(Arc::new(dal), sweeper_config, inner_rx);

        manager.register(Box::new(StaleClaimSweeperService::new(
            sweeper,
            inner_tx,
            self.create_runner_span("stale_claim_sweeper"),
        )));

        Ok(())
    }
}
