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

//! # Pipeline Engine
//!
//! The PipelineEngine provides a unified orchestrator that can run both scheduler and executor
//! components in a single process or distribute them across multiple instances.
//!
//! ## Architecture
//!
//! The PipelineEngine consists of several key components:
//!
//! - **Database**: Provides persistence for task state and workflow definitions
//! - **TaskRegistry**: Maintains a registry of available tasks and their implementations
//! - **TaskScheduler**: Manages workflow scheduling and task queuing
//! - **ThreadTaskExecutor**: Executes tasks and manages their lifecycle
//!
//! ## Deployment Modes
//!
//! The engine supports three deployment modes:
//!
//! - **Unified Mode**: Runs both scheduler and executor in the same process, ideal for
//!   single-node deployments or development environments
//! - **Scheduler Only**: Runs only the scheduler component, useful for distributed deployments
//!   where executors run on separate nodes
//! - **Executor Only**: Runs only the executor component, typically used in worker nodes
//!   that receive tasks from a central scheduler
//!
//! ## Error Handling
//!
//! The engine uses the `ExecutorError` type for error handling, which can occur in several
//! scenarios:
//!
//! - Database connection failures
//! - Task execution errors
//! - Workflow validation errors
//! - Recovery failures
//! - Component panic or unexpected termination
//!
//! ## Recovery
//!
//! The engine supports automatic recovery of orphaned tasks through the `new_with_recovery`
//! constructor. This is particularly useful for handling system interruptions and ensuring
//! task consistency.

use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use super::{EngineMode, ExecutorConfig, ThreadTaskExecutor};
use crate::error::ExecutorError;
use crate::{Database, TaskRegistry, TaskScheduler};

/// Pipeline Engine for unified scheduler and executor orchestration.
///
/// The PipelineEngine provides a high-level interface for running task pipelines
/// with configurable deployment modes. It manages the lifecycle of both scheduler
/// and executor components, handling their coordination and error recovery.
///
/// # Components
///
/// - **Database**: Provides persistence for task state and workflow definitions
/// - **TaskRegistry**: Maintains a registry of available tasks and their implementations
/// - **TaskScheduler**: Manages workflow scheduling and task queuing
/// - **ThreadTaskExecutor**: Executes tasks and manages their lifecycle
///
/// # Deployment Modes
///
/// - **Unified Mode**: Runs both scheduler and executor in the same process
/// - **Scheduler Only**: Runs only the scheduler component
/// - **Executor Only**: Runs only the executor component
///
/// # Error Handling
///
/// The engine uses the `ExecutorError` type for error handling. Errors can occur during:
///
/// - Initialization and recovery
/// - Task execution
/// - Workflow scheduling
/// - Component coordination
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust,ignore
/// use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let database = Database::new("postgresql://localhost/test")?;
/// let task_registry = Arc::new(TaskRegistry::new());
/// let workflows = vec![]; // Add your compiled workflows here
/// let executor_config = ExecutorConfig::default();
///
/// let engine = PipelineEngine::new(
///     database,
///     task_registry,
///     workflows,
///     executor_config,
///     EngineMode::Unified
/// );
/// # Ok(())
/// # }
/// ```
///
/// ## With Recovery
///
/// ```rust,ignore
/// use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let database = Database::new("postgresql://localhost/test")?;
/// let task_registry = Arc::new(TaskRegistry::new());
/// let workflows = vec![]; // Add your compiled workflows here
/// let executor_config = ExecutorConfig::default();
///
/// let engine = PipelineEngine::new_with_recovery(
///     database,
///     task_registry,
///     workflows,
///     executor_config,
///     EngineMode::Unified
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct PipelineEngine {
    database: Database,
    task_registry: Arc<TaskRegistry>,
    scheduler: TaskScheduler,
    executor: ThreadTaskExecutor,
    mode: EngineMode,
}

impl PipelineEngine {
    /// Creates a new PipelineEngine with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `database` - Database connection for persistence
    /// * `task_registry` - Registry containing available tasks
    /// * `workflows` - Vector of compiled workflows for the scheduler
    /// * `executor_config` - Configuration for the executor component
    /// * `mode` - Deployment mode (unified, scheduler-only, or executor-only)
    ///
    /// # Panics
    ///
    /// This method will panic if:
    /// - The database connection is invalid
    /// - The task registry is empty
    /// - The workflows contain invalid configurations
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let database = Database::new("postgresql://localhost/test")?;
    /// let task_registry = Arc::new(TaskRegistry::new());
    /// let workflows = vec![]; // Add your compiled workflows here
    /// let executor_config = ExecutorConfig::default();
    ///
    /// let engine = PipelineEngine::new(
    ///     database,
    ///     task_registry,
    ///     workflows,
    ///     executor_config,
    ///     EngineMode::Unified
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        _workflows: Vec<crate::Workflow>,
        executor_config: ExecutorConfig,
        mode: EngineMode,
    ) -> Self {
        let scheduler = TaskScheduler::with_poll_interval_sync(
            database.clone(),
            std::time::Duration::from_millis(100),
        );
        let executor = ThreadTaskExecutor::new(
            database.clone(),
            Arc::clone(&task_registry),
            executor_config,
        );

        Self {
            database,
            task_registry,
            scheduler,
            executor,
            mode,
        }
    }

    /// Creates a new PipelineEngine with recovery-enabled scheduler.
    ///
    /// This method performs recovery during initialization by automatically
    /// detecting and recovering orphaned tasks from previous system interruptions.
    /// It will attempt to resume any tasks that were in progress when the system
    /// was interrupted.
    ///
    /// # Arguments
    ///
    /// * `database` - Database connection for persistence
    /// * `task_registry` - Registry containing available tasks
    /// * `workflows` - Vector of compiled workflows for the scheduler
    /// * `executor_config` - Configuration for the executor component
    /// * `mode` - Deployment mode (unified, scheduler-only, or executor-only)
    ///
    /// # Returns
    ///
    /// * `Ok(PipelineEngine)` - Engine with recovery completed
    /// * `Err(ExecutorError)` - If recovery fails due to:
    ///   - Database connection issues
    ///   - Invalid task states
    ///   - Workflow validation errors
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let database = Database::new("postgresql://localhost/test")?;
    /// let task_registry = Arc::new(TaskRegistry::new());
    /// let workflows = vec![]; // Add your compiled workflows here
    /// let executor_config = ExecutorConfig::default();
    ///
    /// let engine = PipelineEngine::new_with_recovery(
    ///     database,
    ///     task_registry,
    ///     workflows,
    ///     executor_config,
    ///     EngineMode::Unified
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new_with_recovery(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        _workflows: Vec<crate::Workflow>,
        executor_config: ExecutorConfig,
        mode: EngineMode,
    ) -> Result<Self, ExecutorError> {
        // Create scheduler with recovery
        let scheduler = TaskScheduler::new(database.clone())
            .await
            .map_err(ExecutorError::Validation)?;

        let executor = ThreadTaskExecutor::new(
            database.clone(),
            Arc::clone(&task_registry),
            executor_config,
        );

        Ok(Self {
            database,
            task_registry,
            scheduler,
            executor,
            mode,
        })
    }

    /// Starts the pipeline engine in the configured mode.
    ///
    /// This method will run indefinitely, processing workflows and executing tasks
    /// according to the configured deployment mode. It handles the coordination
    /// between scheduler and executor components, including error recovery and
    /// graceful shutdown.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the engine shuts down gracefully
    /// * `Err(ExecutorError)` - If an unrecoverable error occurs, such as:
    ///   - Component panic
    ///   - Database connection loss
    ///   - Task execution failure
    ///   - Workflow scheduling error
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let database = Database::new("postgresql://localhost/test")?;
    /// # let task_registry = Arc::new(TaskRegistry::new());
    /// # let executor_config = ExecutorConfig::default();
    /// # let engine = PipelineEngine::new(database, task_registry, executor_config, EngineMode::Unified);
    /// // This will block until shutdown
    /// engine.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(self) -> Result<(), ExecutorError> {
        match self.mode {
            EngineMode::Unified => {
                info!("Starting Pipeline Engine in Unified mode");
                self.run_unified().await
            }
            EngineMode::SchedulerOnly => {
                info!("Starting Pipeline Engine in Scheduler-Only mode");
                self.run_scheduler_only().await
            }
            EngineMode::ExecutorOnly => {
                info!("Starting Pipeline Engine in Executor-Only mode");
                self.run_executor_only().await
            }
        }
    }

    /// Runs both scheduler and executor components concurrently.
    async fn run_unified(self) -> Result<(), ExecutorError> {
        let scheduler_handle: JoinHandle<Result<(), ExecutorError>> = tokio::spawn(async move {
            self.scheduler
                .run_scheduling_loop()
                .await
                .map_err(ExecutorError::Validation)
        });

        let executor_handle = tokio::spawn(async move { self.executor.run().await });

        // Wait for either component to complete (or fail)
        tokio::select! {
            scheduler_result = scheduler_handle => {
                match scheduler_result {
                    Ok(Ok(())) => {
                        info!("Scheduler completed successfully");
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!("Scheduler failed: {}", e);
                        Err(e)
                    }
                    Err(join_error) => {
                        error!("Scheduler task panicked: {}", join_error);
                        Err(ExecutorError::InvalidScope(format!("Scheduler panic: {}", join_error)))
                    }
                }
            }
            executor_result = executor_handle => {
                match executor_result {
                    Ok(Ok(())) => {
                        info!("Executor completed successfully");
                        Ok(())
                    }
                    Ok(Err(e)) => {
                        error!("Executor failed: {}", e);
                        Err(e)
                    }
                    Err(join_error) => {
                        error!("Executor task panicked: {}", join_error);
                        Err(ExecutorError::InvalidScope(format!("Executor panic: {}", join_error)))
                    }
                }
            }
        }
    }

    /// Runs only the scheduler component.
    async fn run_scheduler_only(self) -> Result<(), ExecutorError> {
        self.scheduler
            .run_scheduling_loop()
            .await
            .map_err(ExecutorError::Validation)
    }

    /// Runs only the executor component.
    async fn run_executor_only(self) -> Result<(), ExecutorError> {
        self.executor.run().await
    }

    /// Gets a reference to the task registry.
    ///
    /// # Returns
    ///
    /// A reference to the task registry containing all registered tasks.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let database = Database::new("postgresql://localhost/test")?;
    /// # let task_registry = Arc::new(TaskRegistry::new());
    /// # let executor_config = ExecutorConfig::default();
    /// # let engine = PipelineEngine::new(database, task_registry, executor_config, EngineMode::Unified);
    /// let registry = engine.task_registry();
    /// # Ok(())
    /// # }
    /// ```
    pub fn task_registry(&self) -> &Arc<TaskRegistry> {
        &self.task_registry
    }

    /// Gets a reference to the database.
    ///
    /// # Returns
    ///
    /// A reference to the database connection used for persistence.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let database = Database::new("postgresql://localhost/test")?;
    /// # let task_registry = Arc::new(TaskRegistry::new());
    /// # let executor_config = ExecutorConfig::default();
    /// # let engine = PipelineEngine::new(database, task_registry, executor_config, EngineMode::Unified);
    /// let db = engine.database();
    /// # Ok(())
    /// # }
    /// ```
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Gets the current engine mode.
    ///
    /// # Returns
    ///
    /// A reference to the current deployment mode of the engine.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use cloacina::{Database, TaskRegistry, PipelineEngine, ExecutorConfig, EngineMode};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let database = Database::new("postgresql://localhost/test")?;
    /// # let task_registry = Arc::new(TaskRegistry::new());
    /// # let executor_config = ExecutorConfig::default();
    /// # let engine = PipelineEngine::new(database, task_registry, executor_config, EngineMode::Unified);
    /// let mode = engine.mode();
    /// # Ok(())
    /// # }
    /// ```
    pub fn mode(&self) -> &EngineMode {
        &self.mode
    }
}

/// Debug implementation for PipelineEngine.
///
/// Provides a debug representation of the engine that includes:
/// - The current deployment mode
/// - The number of registered tasks
///
/// This implementation is useful for logging and debugging purposes.
impl std::fmt::Debug for PipelineEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineEngine")
            .field("mode", &self.mode)
            .field("task_registry_size", &self.task_registry.task_count())
            .finish()
    }
}
