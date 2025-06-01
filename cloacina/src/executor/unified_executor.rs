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

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::pipeline_executor::*;
use crate::dal::DAL;
use crate::executor::types::ExecutorConfig;
use crate::task::TaskState;
use crate::UniversalUuid;
use crate::{Context, Database, TaskExecutor, TaskScheduler};

/// Configuration for the unified pipeline executor
///
/// This struct defines the configuration parameters that control the behavior
/// of the UnifiedExecutor. It includes settings for concurrency, timeouts,
/// polling intervals, and database connection management.
#[derive(Debug, Clone)]
pub struct UnifiedExecutorConfig {
    /// Maximum number of concurrent task executions allowed at any given time.
    /// This controls the parallelism of task processing.
    pub max_concurrent_tasks: usize,

    /// How often the task executor should poll for new tasks to execute.
    /// Lower values increase responsiveness but may increase database load.
    pub executor_poll_interval: Duration,

    /// How often the scheduler should check for ready tasks and dependencies.
    /// Lower values increase responsiveness but may increase database load.
    pub scheduler_poll_interval: Duration,

    /// Maximum time allowed for a single task to execute before timing out.
    /// Tasks that exceed this duration will be marked as failed.
    pub task_timeout: Duration,

    /// Optional maximum time allowed for an entire pipeline execution.
    /// If set, the pipeline will be marked as failed if it exceeds this duration.
    pub pipeline_timeout: Option<Duration>,

    /// Number of database connections to maintain in the connection pool.
    /// This should be tuned based on expected concurrent load.
    pub db_pool_size: u32,

    /// Whether to enable automatic recovery of in-progress workflows on startup.
    /// When enabled, the executor will attempt to resume interrupted workflows.
    pub enable_recovery: bool,
}

impl Default for UnifiedExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            executor_poll_interval: Duration::from_millis(100), // 100ms for responsive execution
            scheduler_poll_interval: Duration::from_millis(100), // 100ms for responsive scheduling
            task_timeout: Duration::from_secs(300),             // 5 minutes
            pipeline_timeout: Some(Duration::from_secs(3600)),  // 1 hour
            db_pool_size: {
                #[cfg(feature = "sqlite")]
                {
                    1
                } // SQLite works best with single connection
                #[cfg(feature = "postgres")]
                {
                    10
                }
            },
            enable_recovery: true,
        }
    }
}

/// Unified executor that coordinates workflow scheduling and task execution
///
/// This struct provides a unified interface for managing workflow executions,
/// combining the functionality of the TaskScheduler and TaskExecutor. It handles:
/// - Workflow scheduling and execution
/// - Task execution and monitoring
/// - Background service management
/// - Execution status tracking and reporting
///
/// The executor maintains its own runtime state and manages the lifecycle of
/// background services for scheduling and task execution.
pub struct UnifiedExecutor {
    /// Database connection for persistence and state management
    database: Database,
    /// Configuration parameters for the executor
    config: UnifiedExecutorConfig,
    /// Task scheduler for managing workflow execution scheduling
    scheduler: Arc<TaskScheduler>,
    /// Task executor for running individual tasks
    executor: Arc<TaskExecutor>,
    /// Runtime handles for managing background services
    runtime_handles: Arc<RwLock<RuntimeHandles>>,
}

/// Internal structure for managing runtime handles of background services
///
/// This struct maintains references to the running background tasks and
/// shutdown channels used to coordinate graceful shutdown of services.
struct RuntimeHandles {
    /// Handle to the scheduler background task
    scheduler_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the executor background task
    executor_handle: Option<tokio::task::JoinHandle<()>>,
    /// Channel sender for broadcasting shutdown signals
    shutdown_sender: Option<broadcast::Sender<()>>,
}

#[cfg(feature = "postgres")]
/// Builder for creating a UnifiedExecutor with PostgreSQL schema-based multi-tenancy
///
/// This builder supports PostgreSQL schema-based multi-tenancy for complete tenant isolation.
/// Each schema provides complete data isolation with zero collision risk.
///
/// # Example
/// ```rust
/// // Single-tenant PostgreSQL (uses public schema)
/// let executor = UnifiedExecutorBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .build()
///     .await?;
///
/// // Multi-tenant PostgreSQL with schema isolation
/// let tenant_a = UnifiedExecutorBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .schema("tenant_a")
///     .build()
///     .await?;
///
/// let tenant_b = UnifiedExecutorBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .schema("tenant_b")
///     .build()
///     .await?;
/// ```
pub struct UnifiedExecutorBuilder {
    database_url: Option<String>,
    schema: Option<String>,
    config: UnifiedExecutorConfig,
}

#[cfg(feature = "postgres")]
impl Default for UnifiedExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "postgres")]
impl UnifiedExecutorBuilder {
    /// Creates a new builder with default configuration
    pub fn new() -> Self {
        Self {
            database_url: None,
            schema: None,
            config: UnifiedExecutorConfig::default(),
        }
    }

    /// Sets the database URL
    pub fn database_url(mut self, url: &str) -> Self {
        self.database_url = Some(url.to_string());
        self
    }

    /// Sets the PostgreSQL schema for multi-tenant isolation
    ///
    /// # Arguments
    /// * `schema` - The schema name (must be alphanumeric with underscores only)
    pub fn schema(mut self, schema: &str) -> Self {
        self.schema = Some(schema.to_string());
        self
    }

    /// Sets the full configuration
    pub fn with_config(mut self, config: UnifiedExecutorConfig) -> Self {
        self.config = config;
        self
    }

    /// Validates the schema name contains only alphanumeric characters and underscores
    fn validate_schema_name(schema: &str) -> Result<(), PipelineError> {
        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(PipelineError::Configuration {
                message: "Schema name must contain only alphanumeric characters and underscores"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// Builds the UnifiedExecutor
    pub async fn build(self) -> Result<UnifiedExecutor, PipelineError> {
        let database_url = self
            .database_url
            .ok_or_else(|| PipelineError::Configuration {
                message: "Database URL is required".to_string(),
            })?;

        if let Some(ref schema) = self.schema {
            Self::validate_schema_name(schema)?;

            // Validate schema is only used with PostgreSQL
            if !database_url.starts_with("postgresql://")
                && !database_url.starts_with("postgres://")
            {
                return Err(PipelineError::Configuration {
                    message: "Schema isolation is only supported with PostgreSQL. \
                             For SQLite multi-tenancy, use separate database files instead."
                        .to_string(),
                });
            }
        }

        // Create the database with schema support
        let database = Database::new_with_schema(
            &database_url,
            "cloacina",
            self.config.db_pool_size,
            self.schema.as_deref(),
        );

        // Set up schema if specified
        if let Some(ref schema) = self.schema {
            database
                .setup_schema(schema)
                .map_err(|e| PipelineError::Configuration {
                    message: format!("Failed to set up schema '{}': {}", schema, e),
                })?;
        } else {
            // Run migrations in public schema
            let mut conn =
                database
                    .pool()
                    .get()
                    .map_err(|e| PipelineError::DatabaseConnection {
                        message: e.to_string(),
                    })?;
            crate::database::run_migrations(&mut conn).map_err(|e| {
                PipelineError::DatabaseConnection {
                    message: e.to_string(),
                }
            })?;
        }

        // Create scheduler with recovery if enabled
        let scheduler = if self.config.enable_recovery {
            TaskScheduler::with_global_workflows_and_recovery_and_poll_interval(
                database.clone(),
                self.config.scheduler_poll_interval,
            )
            .await
        } else {
            let workflows = crate::workflow::get_all_workflows();
            Ok(TaskScheduler::with_poll_interval(
                database.clone(),
                workflows,
                self.config.scheduler_poll_interval,
            ))
        }
        .map_err(|e| PipelineError::Executor(e.into()))?;

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: self.config.max_concurrent_tasks,
            poll_interval: self.config.executor_poll_interval,
            task_timeout: self.config.task_timeout,
        };

        let executor = TaskExecutor::with_global_registry(database.clone(), executor_config)
            .map_err(|e| PipelineError::Configuration {
                message: e.to_string(),
            })?;

        let unified_executor = UnifiedExecutor {
            database,
            config: self.config,
            scheduler: Arc::new(scheduler),
            executor: Arc::new(executor),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                shutdown_sender: None,
            })),
        };

        // Start the background services immediately
        unified_executor.start_background_services().await?;

        Ok(unified_executor)
    }
}

impl UnifiedExecutor {
    /// Creates a new unified executor with default configuration
    ///
    /// # Arguments
    /// * `database_url` - Connection string for the database
    ///
    /// # Returns
    /// * `Result<Self, PipelineError>` - The initialized executor or an error
    ///
    /// # Example
    /// ```rust
    /// let executor = UnifiedExecutor::new("postgres://localhost/db").await?;
    /// ```
    pub async fn new(database_url: &str) -> Result<Self, PipelineError> {
        Self::with_config(database_url, UnifiedExecutorConfig::default()).await
    }

    /// Creates a builder for configuring the executor
    ///
    /// # Returns
    /// * `UnifiedExecutorBuilder` - Builder for configuring the executor
    ///
    /// # Example
    /// ```rust
    /// let executor = UnifiedExecutor::builder()
    ///     .database_url("postgres://localhost/db")
    ///     .build()
    ///     .await?;
    /// ```
    #[cfg(feature = "postgres")]
    pub fn builder() -> UnifiedExecutorBuilder {
        UnifiedExecutorBuilder::new()
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
    /// ```rust
    /// let executor = UnifiedExecutor::with_schema(
    ///     "postgresql://user:pass@localhost/cloacina",
    ///     "tenant_123"
    /// ).await?;
    /// ```
    #[cfg(feature = "postgres")]
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
        config: UnifiedExecutorConfig,
    ) -> Result<Self, PipelineError> {
        // Initialize database
        let database = Database::new(database_url, "cloacina", config.db_pool_size);

        // Run migrations
        {
            let mut conn =
                database
                    .pool()
                    .get()
                    .map_err(|e| PipelineError::DatabaseConnection {
                        message: e.to_string(),
                    })?;
            crate::database::run_migrations(&mut conn).map_err(|e| {
                PipelineError::DatabaseConnection {
                    message: e.to_string(),
                }
            })?;
        }

        // Create scheduler with recovery if enabled
        let scheduler = if config.enable_recovery {
            TaskScheduler::with_global_workflows_and_recovery_and_poll_interval(
                database.clone(),
                config.scheduler_poll_interval,
            )
            .await
        } else {
            let workflows = crate::workflow::get_all_workflows();
            Ok(TaskScheduler::with_poll_interval(
                database.clone(),
                workflows,
                config.scheduler_poll_interval,
            ))
        }
        .map_err(|e| PipelineError::Executor(e.into()))?;

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: config.max_concurrent_tasks,
            poll_interval: config.executor_poll_interval,
            task_timeout: config.task_timeout,
        };

        let executor = TaskExecutor::with_global_registry(database.clone(), executor_config)
            .map_err(|e| PipelineError::Configuration {
                message: e.to_string(),
            })?;

        let unified_executor = Self {
            database,
            config,
            scheduler: Arc::new(scheduler),
            executor: Arc::new(executor),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                shutdown_sender: None,
            })),
        };

        // Start the background services immediately
        unified_executor.start_background_services().await?;

        Ok(unified_executor)
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
    async fn start_background_services(&self) -> Result<(), PipelineError> {
        let mut handles = self.runtime_handles.write().await;

        tracing::info!("Starting scheduler and executor background services");

        // Create shutdown channel
        let (shutdown_tx, mut scheduler_shutdown_rx) = broadcast::channel(1);
        let mut executor_shutdown_rx = shutdown_tx.subscribe();

        // Start scheduler
        let scheduler = self.scheduler.clone();
        let scheduler_handle = tokio::spawn(async move {
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
        });

        // Start executor
        let executor = self.executor.clone();
        let executor_handle = tokio::spawn(async move {
            let mut executor_future = Box::pin(executor.run());

            tokio::select! {
                result = &mut executor_future => {
                    if let Err(e) = result {
                        tracing::error!("Executor failed: {}", e);
                    } else {
                        tracing::info!("Executor completed");
                    }
                }
                _ = executor_shutdown_rx.recv() => {
                    tracing::info!("Executor shutdown requested");
                }
            }
        });

        // Store handles
        handles.scheduler_handle = Some(scheduler_handle);
        handles.executor_handle = Some(executor_handle);
        handles.shutdown_sender = Some(shutdown_tx);

        Ok(())
    }

    /// Gracefully shuts down the executor and its background services
    ///
    /// This method:
    /// 1. Sends shutdown signals to background services
    /// 2. Waits for services to complete
    /// 3. Cleans up runtime handles
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

        Ok(())
    }

    /// Builds a pipeline result from an execution ID
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<PipelineResult, PipelineError>` - The complete pipeline result or an error
    ///
    /// This method:
    /// 1. Retrieves pipeline execution details
    /// 2. Gets all task executions
    /// 3. Retrieves the final context
    /// 4. Builds task results
    /// 5. Constructs the complete pipeline result
    async fn build_pipeline_result(
        &self,
        execution_id: Uuid,
    ) -> Result<PipelineResult, PipelineError> {
        let dal = DAL::new(self.database.pool());

        let pipeline_execution = dal
            .pipeline_execution()
            .get_by_id(UniversalUuid(execution_id))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get pipeline execution: {}", e),
            })?;

        let task_executions = dal
            .task_execution()
            .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get task executions: {}", e),
            })?;

        // Get final context using DAL
        let final_context = if let Some(context_id) = pipeline_execution.context_id {
            dal.context()
                .read(context_id)
                .map_err(|e| PipelineError::ExecutionFailed {
                    message: format!("Failed to get context: {}", e),
                })?
        } else {
            Context::new()
        };

        // Build task results
        let task_results: Vec<TaskResult> = task_executions
            .into_iter()
            .map(|task_exec| {
                let status = match task_exec.status.as_str() {
                    "Pending" => TaskState::Pending,
                    "Running" => TaskState::Running {
                        start_time: task_exec
                            .started_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    "Completed" => TaskState::Completed {
                        completion_time: task_exec
                            .completed_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    "Failed" => TaskState::Failed {
                        error: task_exec
                            .error_details
                            .clone()
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        failure_time: task_exec
                            .completed_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    "Skipped" => TaskState::Skipped {
                        reason: task_exec
                            .error_details
                            .clone()
                            .unwrap_or_else(|| "Trigger rules not satisfied".to_string()),
                        skip_time: task_exec
                            .completed_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    _ => TaskState::Failed {
                        error: format!("Unknown status: {}", task_exec.status),
                        failure_time: chrono::Utc::now(),
                    },
                };

                let duration =
                    task_exec
                        .completed_at
                        .zip(task_exec.started_at)
                        .map(|(end, start)| {
                            let end_utc = end.0;
                            let start_utc = start.0;
                            (end_utc - start_utc).to_std().unwrap_or(Duration::ZERO)
                        });

                TaskResult {
                    task_name: task_exec.task_name,
                    status,
                    start_time: task_exec.started_at.map(|ts| ts.0),
                    end_time: task_exec.completed_at.map(|ts| ts.0),
                    duration,
                    attempt_count: task_exec.attempt,
                    error_message: task_exec.error_details,
                }
            })
            .collect();

        // Convert status
        let status = match pipeline_execution.status.as_str() {
            "Pending" => PipelineStatus::Pending,
            "Running" => PipelineStatus::Running,
            "Completed" => PipelineStatus::Completed,
            "Failed" => PipelineStatus::Failed,
            _ => PipelineStatus::Failed,
        };

        let duration = pipeline_execution.completed_at.map(|end| {
            let end_utc = end.0;
            let start_utc = pipeline_execution.started_at.0;
            (end_utc - start_utc).to_std().unwrap_or(Duration::ZERO)
        });

        Ok(PipelineResult {
            execution_id,
            workflow_name: pipeline_execution.pipeline_name,
            status,
            start_time: pipeline_execution.started_at.0,
            end_time: pipeline_execution.completed_at.map(|ts| ts.0),
            duration: duration,
            final_context,
            task_results,
            error_message: pipeline_execution.error_details,
        })
    }
}

/// Implementation of PipelineExecutor trait for UnifiedExecutor
///
/// This implementation provides the core workflow execution functionality:
/// - Synchronous and asynchronous execution
/// - Status monitoring and result retrieval
/// - Execution cancellation
/// - Execution listing and management
#[async_trait]
impl PipelineExecutor for UnifiedExecutor {
    /// Executes a workflow synchronously and waits for completion
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow
    ///
    /// # Returns
    /// * `Result<PipelineResult, PipelineError>` - The execution result or an error
    ///
    /// This method will block until the workflow completes or times out.
    async fn execute(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<PipelineResult, PipelineError> {
        // Schedule execution
        let execution_id = self
            .scheduler
            .schedule_workflow_execution(workflow_name, context)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to schedule workflow: {}", e),
            })?;

        // Wait for completion
        let start_time = std::time::Instant::now();
        let dal = DAL::new(self.database.pool());

        loop {
            // Check timeout
            if let Some(timeout) = self.config.pipeline_timeout {
                if start_time.elapsed() > timeout {
                    return Err(PipelineError::Timeout {
                        timeout_seconds: timeout.as_secs(),
                    });
                }
            }

            // Check status
            let pipeline = dal
                .pipeline_execution()
                .get_by_id(UniversalUuid(execution_id))
                .map_err(|e| PipelineError::ExecutionFailed {
                    message: format!("Failed to check execution status: {}", e),
                })?;

            match pipeline.status.as_str() {
                "Completed" | "Failed" => {
                    return self.build_pipeline_result(execution_id).await;
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Executes a workflow asynchronously
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow
    ///
    /// # Returns
    /// * `Result<PipelineExecution, PipelineError>` - A handle to the execution or an error
    ///
    /// This method returns immediately with an execution handle that can be used
    /// to monitor the workflow's progress.
    async fn execute_async(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<PipelineExecution, PipelineError> {
        // Schedule execution
        let execution_id = self
            .scheduler
            .schedule_workflow_execution(workflow_name, context)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to schedule workflow: {}", e),
            })?;

        Ok(PipelineExecution::new(
            execution_id,
            workflow_name.to_string(),
            self.clone(),
        ))
    }

    /// Executes a workflow with status callbacks
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow
    /// * `callback` - Callback for receiving status updates
    ///
    /// # Returns
    /// * `Result<PipelineResult, PipelineError>` - The execution result or an error
    ///
    /// This method will block until completion but provides status updates
    /// through the callback interface.
    async fn execute_with_callback(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
        callback: Box<dyn super::pipeline_executor::StatusCallback>,
    ) -> Result<PipelineResult, PipelineError> {
        // Start async execution
        let execution = self.execute_async(workflow_name, context).await?;
        let execution_id = execution.execution_id;

        // Poll for status changes and call callback
        let mut last_status = PipelineStatus::Pending;
        callback.on_status_change(last_status.clone());

        loop {
            let current_status = self.get_execution_status(execution_id).await?;

            if current_status != last_status {
                callback.on_status_change(current_status.clone());
                last_status = current_status.clone();
            }

            if current_status.is_terminal() {
                return self.get_execution_result(execution_id).await;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Gets the current status of a pipeline execution
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<PipelineStatus, PipelineError>` - The current status or an error
    async fn get_execution_status(
        &self,
        execution_id: Uuid,
    ) -> Result<PipelineStatus, PipelineError> {
        let dal = DAL::new(self.database.pool());
        let pipeline = dal
            .pipeline_execution()
            .get_by_id(UniversalUuid(execution_id))
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get execution status: {}", e),
            })?;

        let status = match pipeline.status.as_str() {
            "Pending" => PipelineStatus::Pending,
            "Running" => PipelineStatus::Running,
            "Completed" => PipelineStatus::Completed,
            "Failed" => PipelineStatus::Failed,
            _ => PipelineStatus::Failed,
        };

        Ok(status)
    }

    /// Gets the complete result of a pipeline execution
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<PipelineResult, PipelineError>` - The complete result or an error
    async fn get_execution_result(
        &self,
        execution_id: Uuid,
    ) -> Result<PipelineResult, PipelineError> {
        self.build_pipeline_result(execution_id).await
    }

    /// Cancels an in-progress pipeline execution
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the pipeline execution to cancel
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error status
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<(), PipelineError> {
        // Implementation would mark execution as cancelled in database
        // and notify scheduler/executor to stop processing
        let dal = DAL::new(self.database.pool());

        dal.pipeline_execution()
            .cancel(execution_id.into())
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to cancel execution: {}", e),
            })?;

        Ok(())
    }

    /// Lists recent pipeline executions
    ///
    /// # Returns
    /// * `Result<Vec<PipelineResult>, PipelineError>` - List of recent executions or an error
    ///
    /// Currently limited to the 100 most recent executions.
    async fn list_executions(&self) -> Result<Vec<PipelineResult>, PipelineError> {
        let dal = DAL::new(self.database.pool());

        let executions = dal.pipeline_execution().list_recent(100).map_err(|e| {
            PipelineError::ExecutionFailed {
                message: format!("Failed to list executions: {}", e),
            }
        })?;

        let mut results = Vec::new();
        for execution in executions {
            if let Ok(result) = self.build_pipeline_result(execution.id.into()).await {
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Shuts down the executor
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error status
    async fn shutdown(&self) -> Result<(), PipelineError> {
        UnifiedExecutor::shutdown(self).await
    }
}

// Implementation to make UnifiedExecutor cloneable (for async execution handles)
impl Clone for UnifiedExecutor {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            config: self.config.clone(),
            scheduler: self.scheduler.clone(),
            executor: self.executor.clone(),
            runtime_handles: self.runtime_handles.clone(),
        }
    }
}

// Implement Drop for graceful shutdown
impl Drop for UnifiedExecutor {
    fn drop(&mut self) {
        // Note: Can't use async in Drop, but we can attempt shutdown
        // Users should call shutdown() explicitly for graceful shutdown
        tracing::info!("UnifiedExecutor dropping - consider calling shutdown() explicitly");
    }
}
