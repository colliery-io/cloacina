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
mod service_manager;
mod services;
mod workflow_executor_impl;
mod workflow_result;

pub use config::{DefaultRunnerBuilder, DefaultRunnerConfig, DefaultRunnerConfigBuilder};

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::dal::DAL;
use crate::dispatcher::{DefaultDispatcher, Dispatcher, RoutingConfig, TaskExecutor};
use crate::executor::types::ExecutorConfig;
use crate::executor::workflow_executor::WorkflowExecutionError;
use crate::executor::ThreadTaskExecutor;
use crate::Database;
use crate::Runtime;
use crate::Scheduler;
use crate::TaskScheduler;

use service_manager::ServiceManager;

/// Default runner that coordinates workflow scheduling and task execution.
///
/// Holds only top-level state — runtime, database, config, and the task
/// scheduler. All background services (cron, recovery, registry reconciler,
/// graph scheduler, stale-claim sweeper, ...) live inside the
/// [`ServiceManager`], which owns their handles and shutdown wiring.
///
/// # Shutdown
///
/// Call [`shutdown()`](Self::shutdown) before dropping to gracefully stop
/// background tasks and release database connections.
#[must_use = "DefaultRunner runs background tasks; call shutdown() before dropping"]
pub struct DefaultRunner {
    /// Scoped runtime holding isolated registries for tasks, workflows, and triggers
    pub(super) runtime: Arc<Runtime>,
    /// Database connection for persistence and state management
    pub(super) database: Database,
    /// Configuration parameters for the runner
    pub(super) config: DefaultRunnerConfig,
    /// Task scheduler for managing workflow execution scheduling
    pub(super) scheduler: Arc<TaskScheduler>,
    /// Owns the lifecycle of every background service plus typed accessor slots.
    pub(super) service_manager: Arc<RwLock<ServiceManager>>,
}

impl DefaultRunner {
    /// Creates a new default runner with default configuration
    pub async fn new(database_url: &str) -> Result<Self, WorkflowExecutionError> {
        Self::with_config(database_url, DefaultRunnerConfig::default()).await
    }

    /// Creates a builder for configuring the executor
    pub fn builder() -> DefaultRunnerBuilder {
        DefaultRunnerBuilder::new()
    }

    /// Creates a new executor with PostgreSQL schema-based multi-tenancy
    pub async fn with_schema(
        database_url: &str,
        schema: &str,
    ) -> Result<Self, WorkflowExecutionError> {
        Self::builder()
            .database_url(database_url)
            .schema(schema)
            .build()
            .await
    }

    /// Creates a new unified executor with custom configuration
    pub async fn with_config(
        database_url: &str,
        config: DefaultRunnerConfig,
    ) -> Result<Self, WorkflowExecutionError> {
        // Initialize database
        let database = Database::new(database_url, "cloacina", config.db_pool_size());

        // Run migrations
        database
            .run_migrations()
            .await
            .map_err(|e| WorkflowExecutionError::DatabaseConnection { message: e })?;

        // Fresh inventory-seeded runtime.
        let runtime = Arc::new(Runtime::new());

        // Create scheduler with the scoped runtime
        let scheduler =
            TaskScheduler::with_poll_interval(database.clone(), config.scheduler_poll_interval())
                .await
                .map_err(|e| WorkflowExecutionError::Executor(e.into()))?
                .with_runtime(runtime.clone());

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: config.max_concurrent_tasks(),
            task_timeout: config.task_timeout(),
            enable_claiming: config.enable_claiming(),
            heartbeat_interval: config.heartbeat_interval(),
        };

        let executor = ThreadTaskExecutor::with_runtime_and_registry(
            database.clone(),
            Arc::new(crate::task::TaskRegistry::new()),
            runtime.clone(),
            executor_config,
        );

        // Configure dispatcher for push-based task execution
        let dal = DAL::new(database.clone());
        let routing_config = config
            .routing_config()
            .cloned()
            .unwrap_or_else(RoutingConfig::default);
        let dispatcher = DefaultDispatcher::new(dal, routing_config);

        dispatcher.register_executor("default", Arc::new(executor) as Arc<dyn TaskExecutor>);

        let scheduler = scheduler.with_dispatcher(Arc::new(dispatcher));

        let default_runner = Self {
            runtime,
            database,
            config,
            scheduler: Arc::new(scheduler),
            service_manager: Arc::new(RwLock::new(ServiceManager::new())),
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

    /// Returns a handle to the scoped `Runtime` this runner uses.
    pub fn runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }

    /// Returns the unified scheduler if enabled.
    pub async fn unified_scheduler(&self) -> Option<Arc<Scheduler>> {
        self.service_manager.read().await.unified_scheduler.clone()
    }

    /// Set the graph scheduler for computation graph package routing.
    /// Must be called before `start_services()` so the reconciler can route CG packages.
    pub async fn set_graph_scheduler(
        &self,
        scheduler: Arc<crate::computation_graph::scheduler::ComputationGraphScheduler>,
    ) {
        let slot = self.service_manager.read().await.graph_scheduler.clone();
        *slot.write().await = Some(scheduler);
    }

    /// Gracefully shuts down the executor and its background services.
    pub async fn shutdown(&self) -> Result<(), WorkflowExecutionError> {
        self.service_manager.write().await.shutdown_all().await?;
        // Close the database connection pool to release all connections
        self.database.close();
        Ok(())
    }
}

impl Clone for DefaultRunner {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            database: self.database.clone(),
            config: self.config.clone(),
            scheduler: self.scheduler.clone(),
            service_manager: self.service_manager.clone(),
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
