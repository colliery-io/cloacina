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

//! Default runner for workflow execution.
//!
//! This module provides the DefaultRunner which coordinates workflow scheduling
//! and task execution. It combines the functionality of the TaskScheduler and
//! TaskExecutor into a unified interface.
//!
//! ## Components
//!
//! - `DefaultRunner`: Main runner struct
//! - `DefaultRunnerConfig`: Configuration options
//! - `DefaultRunnerBuilder`: Builder for creating runners with custom settings

mod config;
mod cron_api;
mod pipeline_executor_impl;
mod pipeline_result;
mod services;

pub use config::{DefaultRunnerBuilder, DefaultRunnerConfig};

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::dal::DAL;
use crate::dispatcher::{DefaultDispatcher, Dispatcher, RoutingConfig, TaskExecutor};
use crate::executor::pipeline_executor::PipelineError;
use crate::executor::types::ExecutorConfig;
use crate::executor::ThreadTaskExecutor;
use crate::registry::traits::WorkflowRegistry;
use crate::registry::RegistryReconciler;
use crate::CronScheduler;
use crate::Database;
use crate::TaskScheduler;
use crate::TriggerScheduler;

/// Default runner that coordinates workflow scheduling and task execution
///
/// This struct provides a unified interface for managing workflow executions,
/// combining the functionality of the TaskScheduler and TaskExecutor. It handles:
/// - Workflow scheduling and execution
/// - Task execution and monitoring
/// - Background service management
/// - Execution status tracking and reporting
///
/// The runner maintains its own runtime state and manages the lifecycle of
/// background services for scheduling and task execution.
pub struct DefaultRunner {
    /// Database connection for persistence and state management
    pub(super) database: Database,
    /// Configuration parameters for the runner
    pub(super) config: DefaultRunnerConfig,
    /// Task scheduler for managing workflow execution scheduling
    pub(super) scheduler: Arc<TaskScheduler>,
    /// Runtime handles for managing background services
    pub(super) runtime_handles: Arc<RwLock<RuntimeHandles>>,
    /// Optional cron scheduler for time-based workflow execution
    pub(super) cron_scheduler: Arc<RwLock<Option<Arc<CronScheduler>>>>,
    /// Optional cron recovery service for handling lost executions
    pub(super) cron_recovery: Arc<RwLock<Option<Arc<crate::CronRecoveryService>>>>,
    /// Optional workflow registry for packaged workflows
    pub(super) workflow_registry: Arc<RwLock<Option<Arc<dyn WorkflowRegistry>>>>,
    /// Optional registry reconciler for packaged workflows
    pub(super) registry_reconciler: Arc<RwLock<Option<Arc<RegistryReconciler>>>>,
    /// Optional trigger scheduler for event-based workflow execution
    pub(super) trigger_scheduler: Arc<RwLock<Option<Arc<TriggerScheduler>>>>,
}

/// Internal structure for managing runtime handles of background services
///
/// This struct maintains references to the running background tasks and
/// shutdown channels used to coordinate graceful shutdown of services.
pub(super) struct RuntimeHandles {
    /// Handle to the scheduler background task
    pub(super) scheduler_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the executor background task
    pub(super) executor_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the cron scheduler background task (if enabled)
    pub(super) cron_scheduler_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the cron recovery service background task (if enabled)
    pub(super) cron_recovery_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the registry reconciler background task (if enabled)
    pub(super) registry_reconciler_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the trigger scheduler background task (if enabled)
    pub(super) trigger_scheduler_handle: Option<tokio::task::JoinHandle<()>>,
    /// Channel sender for broadcasting shutdown signals
    pub(super) shutdown_sender: Option<broadcast::Sender<()>>,
}

impl DefaultRunner {
    /// Creates a new default runner with default configuration
    ///
    /// # Arguments
    /// * `database_url` - Connection string for the database
    ///
    /// # Returns
    /// * `Result<Self, PipelineError>` - The initialized executor or an error
    ///
    /// # Example
    /// ```rust,ignore
    /// let runner = DefaultRunner::new("postgres://localhost/db").await?;
    /// ```
    pub async fn new(database_url: &str) -> Result<Self, PipelineError> {
        Self::with_config(database_url, DefaultRunnerConfig::default()).await
    }

    /// Creates a builder for configuring the executor
    ///
    /// # Returns
    /// * `DefaultRunnerBuilder` - Builder for configuring the runner
    ///
    /// # Example
    /// ```rust,ignore
    /// let runner = DefaultRunner::builder()
    ///     .database_url("postgres://localhost/db")
    ///     .build()
    ///     .await?;
    /// ```
    pub fn builder() -> DefaultRunnerBuilder {
        DefaultRunnerBuilder::new()
    }

    /// Creates a new executor with PostgreSQL schema-based multi-tenancy
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    /// * `schema` - Schema name for tenant isolation
    ///
    /// # Returns
    /// * `Result<Self, PipelineError>` - The initialized executor or an error
    ///
    /// # Example
    /// ```rust,ignore
    /// let runner = DefaultRunner::with_schema(
    ///     "postgresql://user:pass@localhost/cloacina",
    ///     "tenant_123"
    /// ).await?;
    /// ```
    pub async fn with_schema(database_url: &str, schema: &str) -> Result<Self, PipelineError> {
        Self::builder()
            .database_url(database_url)
            .schema(schema)
            .build()
            .await
    }

    /// Creates a new unified executor with custom configuration
    ///
    /// # Arguments
    /// * `database_url` - Connection string for the database
    /// * `config` - Custom configuration for the executor
    ///
    /// # Returns
    /// * `Result<Self, PipelineError>` - The initialized executor or an error
    ///
    /// This method:
    /// 1. Initializes the database connection
    /// 2. Runs any pending database migrations
    /// 3. Creates the task scheduler with optional recovery
    /// 4. Creates the task executor
    /// 5. Starts background services
    pub async fn with_config(
        database_url: &str,
        config: DefaultRunnerConfig,
    ) -> Result<Self, PipelineError> {
        // Initialize database
        let database = Database::new(database_url, "cloacina", config.db_pool_size);

        // Run migrations
        database
            .run_migrations()
            .await
            .map_err(|e| PipelineError::DatabaseConnection { message: e })?;

        // Create scheduler with global workflow registry (always dynamic)
        let scheduler =
            TaskScheduler::with_poll_interval(database.clone(), config.scheduler_poll_interval)
                .await
                .map_err(|e| PipelineError::Executor(e.into()))?;

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: config.max_concurrent_tasks,
            task_timeout: config.task_timeout,
        };

        let executor = ThreadTaskExecutor::with_global_registry(database.clone(), executor_config)
            .map_err(|e| PipelineError::Configuration {
                message: e.to_string(),
            })?;

        // Configure dispatcher for push-based task execution
        let dal = DAL::new(database.clone());
        let routing_config = config
            .routing_config
            .clone()
            .unwrap_or_else(RoutingConfig::default);
        let dispatcher = DefaultDispatcher::new(dal, routing_config);

        // Register the executor with the dispatcher
        dispatcher.register_executor("default", Arc::new(executor) as Arc<dyn TaskExecutor>);

        let scheduler = scheduler.with_dispatcher(Arc::new(dispatcher));

        let default_runner = Self {
            database,
            config,
            scheduler: Arc::new(scheduler),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                cron_scheduler_handle: None,
                cron_recovery_handle: None,
                registry_reconciler_handle: None,
                trigger_scheduler_handle: None,
                shutdown_sender: None,
            })),
            cron_scheduler: Arc::new(RwLock::new(None)), // Initially empty
            cron_recovery: Arc::new(RwLock::new(None)),  // Initially empty
            workflow_registry: Arc::new(RwLock::new(None)), // Initially empty
            registry_reconciler: Arc::new(RwLock::new(None)), // Initially empty
            trigger_scheduler: Arc::new(RwLock::new(None)), // Initially empty
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
    }

    /// Returns a reference to the database.
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Returns the DAL for database operations.
    pub fn dal(&self) -> DAL {
        DAL::new(self.database.clone())
    }

    /// Returns the trigger scheduler if enabled.
    ///
    /// Returns `None` if trigger scheduling is disabled or not yet initialized.
    pub async fn trigger_scheduler(&self) -> Option<Arc<crate::TriggerScheduler>> {
        self.trigger_scheduler.read().await.clone()
    }

    /// Gracefully shuts down the executor and its background services
    ///
    /// This method:
    /// 1. Sends shutdown signals to background services
    /// 2. Waits for services to complete
    /// 3. Cleans up runtime handles
    /// 4. Closes the database connection pool
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error status
    pub async fn shutdown(&self) -> Result<(), PipelineError> {
        let mut handles = self.runtime_handles.write().await;

        // Send shutdown signal
        if let Some(sender) = handles.shutdown_sender.take() {
            let _ = sender.send(());
        }

        // Wait for scheduler to finish
        if let Some(handle) = handles.scheduler_handle.take() {
            let _ = handle.await;
        }

        // Wait for executor to finish
        if let Some(handle) = handles.executor_handle.take() {
            let _ = handle.await;
        }

        // Wait for cron scheduler to finish (if enabled)
        if let Some(handle) = handles.cron_scheduler_handle.take() {
            let _ = handle.await;
        }

        // Wait for cron recovery service to finish (if enabled)
        if let Some(handle) = handles.cron_recovery_handle.take() {
            let _ = handle.await;
        }

        // Wait for registry reconciler to finish (if enabled)
        if let Some(handle) = handles.registry_reconciler_handle.take() {
            let _ = handle.await;
        }

        // Wait for trigger scheduler to finish (if enabled)
        if let Some(handle) = handles.trigger_scheduler_handle.take() {
            let _ = handle.await;
        }

        // Close the database connection pool to release all connections
        self.database.close();

        Ok(())
    }
}

impl Clone for DefaultRunner {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            config: self.config.clone(),
            scheduler: self.scheduler.clone(),
            runtime_handles: self.runtime_handles.clone(),
            cron_scheduler: self.cron_scheduler.clone(),
            cron_recovery: self.cron_recovery.clone(),
            workflow_registry: self.workflow_registry.clone(),
            registry_reconciler: self.registry_reconciler.clone(),
            trigger_scheduler: self.trigger_scheduler.clone(),
        }
    }
}

// Implement Drop for graceful shutdown
impl Drop for DefaultRunner {
    fn drop(&mut self) {
        // Note: Can't use async in Drop, but we can attempt shutdown
        // Users should call shutdown() explicitly for graceful shutdown
        tracing::info!("DefaultRunner dropping - consider calling shutdown() explicitly");
    }
}
