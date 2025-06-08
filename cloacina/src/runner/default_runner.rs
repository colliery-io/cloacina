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
use tokio::sync::{broadcast, watch, RwLock};
use uuid::Uuid;

use crate::dal::DAL;
use crate::executor::pipeline_executor::*;
use crate::executor::types::ExecutorConfig;
use crate::task::TaskState;
use crate::UniversalUuid;
use crate::{Context, Database, TaskExecutor, TaskScheduler};
use crate::{CronScheduler, CronSchedulerConfig};

/// Configuration for the default runner
///
/// This struct defines the configuration parameters that control the behavior
/// of the DefaultRunner. It includes settings for concurrency, timeouts,
/// polling intervals, and database connection management.
#[derive(Debug, Clone)]
pub struct DefaultRunnerConfig {
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

    /// Whether to enable cron scheduling functionality
    pub enable_cron_scheduling: bool,

    /// How often to poll for due cron schedules (when cron enabled)
    pub cron_poll_interval: Duration,

    /// Maximum number of missed executions to run in catchup mode (usize::MAX = unlimited)
    pub cron_max_catchup_executions: usize,

    /// Whether to enable automatic recovery of lost cron executions
    pub cron_enable_recovery: bool,

    /// How often to check for lost cron executions
    pub cron_recovery_interval: Duration,

    /// Consider executions lost if claimed more than this many minutes ago
    pub cron_lost_threshold_minutes: i32,

    /// Maximum age of executions to recover (older ones are abandoned)
    pub cron_max_recovery_age: Duration,

    /// Maximum number of recovery attempts per execution
    pub cron_max_recovery_attempts: usize,
}

impl Default for DefaultRunnerConfig {
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
            enable_cron_scheduling: true, // Opt-out
            cron_poll_interval: Duration::from_secs(30),
            cron_max_catchup_executions: usize::MAX, // No practical limit by default
            cron_enable_recovery: true,
            cron_recovery_interval: Duration::from_secs(300), // 5 minutes
            cron_lost_threshold_minutes: 10,
            cron_max_recovery_age: Duration::from_secs(86400), // 24 hours
            cron_max_recovery_attempts: 3,
        }
    }
}

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
    database: Database,
    /// Configuration parameters for the runner
    config: DefaultRunnerConfig,
    /// Task scheduler for managing workflow execution scheduling
    scheduler: Arc<TaskScheduler>,
    /// Task executor for running individual tasks
    executor: Arc<TaskExecutor>,
    /// Runtime handles for managing background services
    runtime_handles: Arc<RwLock<RuntimeHandles>>,
    /// Optional cron scheduler for time-based workflow execution
    cron_scheduler: Arc<RwLock<Option<Arc<CronScheduler>>>>,
    /// Optional cron recovery service for handling lost executions
    cron_recovery: Arc<RwLock<Option<Arc<crate::CronRecoveryService>>>>,
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
    /// Handle to the cron scheduler background task (if enabled)
    cron_scheduler_handle: Option<tokio::task::JoinHandle<()>>,
    /// Handle to the cron recovery service background task (if enabled)
    cron_recovery_handle: Option<tokio::task::JoinHandle<()>>,
    /// Channel sender for broadcasting shutdown signals
    shutdown_sender: Option<broadcast::Sender<()>>,
}

#[cfg(feature = "postgres")]
/// Builder for creating a DefaultRunner with PostgreSQL schema-based multi-tenancy
///
/// This builder supports PostgreSQL schema-based multi-tenancy for complete tenant isolation.
/// Each schema provides complete data isolation with zero collision risk.
///
/// # Example
/// ```rust
/// // Single-tenant PostgreSQL (uses public schema)
/// let runner = DefaultRunnerBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .build()
///     .await?;
///
/// // Multi-tenant PostgreSQL with schema isolation
/// let tenant_a = DefaultRunnerBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .schema("tenant_a")
///     .build()
///     .await?;
///
/// let tenant_b = DefaultRunnerBuilder::new()
///     .database_url("postgresql://user:pass@localhost/cloacina")
///     .schema("tenant_b")
///     .build()
///     .await?;
/// ```
pub struct DefaultRunnerBuilder {
    database_url: Option<String>,
    schema: Option<String>,
    config: DefaultRunnerConfig,
}

#[cfg(feature = "postgres")]
impl Default for DefaultRunnerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "postgres")]
impl DefaultRunnerBuilder {
    /// Creates a new builder with default configuration
    pub fn new() -> Self {
        Self {
            database_url: None,
            schema: None,
            config: DefaultRunnerConfig::default(),
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
    pub fn with_config(mut self, config: DefaultRunnerConfig) -> Self {
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

    /// Builds the DefaultRunner
    pub async fn build(self) -> Result<DefaultRunner, PipelineError> {
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
                .await
                .map_err(|e| PipelineError::Configuration {
                    message: format!("Failed to set up schema '{}': {}", schema, e),
                })?;
        } else {
            // Run migrations in public schema
            let conn =
                database
                    .pool()
                    .get()
                    .await
                    .map_err(|e| PipelineError::DatabaseConnection {
                        message: e.to_string(),
                    })?;
            conn.interact(|conn| crate::database::run_migrations(conn))
                .await
                .map_err(|e| PipelineError::DatabaseConnection {
                    message: e.to_string(),
                })?
                .map_err(|e| PipelineError::DatabaseConnection {
                    message: e.to_string(),
                })?;
        }

        // Create scheduler with global workflow registry (always dynamic)
        let scheduler = TaskScheduler::with_poll_interval(
            database.clone(),
            self.config.scheduler_poll_interval,
        )
        .await
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

        let default_runner = DefaultRunner {
            database,
            config: self.config.clone(),
            scheduler: Arc::new(scheduler),
            executor: Arc::new(executor),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                cron_scheduler_handle: None,
                cron_recovery_handle: None,
                shutdown_sender: None,
            })),
            cron_scheduler: Arc::new(RwLock::new(None)), // Initially empty
            cron_recovery: Arc::new(RwLock::new(None)),  // Initially empty
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
    }
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
    /// ```rust
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
    /// ```rust
    /// let runner = DefaultRunner::builder()
    ///     .database_url("postgres://localhost/db")
    ///     .build()
    ///     .await?;
    /// ```
    #[cfg(feature = "postgres")]
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
    /// ```rust
    /// let runner = DefaultRunner::with_schema(
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
        config: DefaultRunnerConfig,
    ) -> Result<Self, PipelineError> {
        // Initialize database
        let database = Database::new(database_url, "cloacina", config.db_pool_size);

        // Run migrations
        {
            let conn =
                database
                    .pool()
                    .get()
                    .await
                    .map_err(|e| PipelineError::DatabaseConnection {
                        message: e.to_string(),
                    })?;
            conn.interact(|conn| crate::database::run_migrations(conn))
                .await
                .map_err(|e| PipelineError::DatabaseConnection {
                    message: e.to_string(),
                })?
                .map_err(|e| PipelineError::DatabaseConnection {
                    message: e.to_string(),
                })?;
        }

        // Create scheduler with global workflow registry (always dynamic)
        let scheduler =
            TaskScheduler::with_poll_interval(database.clone(), config.scheduler_poll_interval)
                .await
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

        let default_runner = Self {
            database,
            config,
            scheduler: Arc::new(scheduler),
            executor: Arc::new(executor),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                cron_scheduler_handle: None,
                cron_recovery_handle: None,
                shutdown_sender: None,
            })),
            cron_scheduler: Arc::new(RwLock::new(None)), // Initially empty
            cron_recovery: Arc::new(RwLock::new(None)),  // Initially empty
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
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
        handles.shutdown_sender = Some(shutdown_tx.clone());

        // Phase 2: Create and start CronScheduler if enabled
        if self.config.enable_cron_scheduling {
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
            let dal = DAL::new(self.database.pool());
            let cron_scheduler = CronScheduler::new(
                Arc::new(dal),
                Arc::new(self.clone()), // self implements PipelineExecutor!
                cron_config,
                cron_shutdown_rx,
            );

            // Start cron background service
            let mut cron_scheduler_clone = cron_scheduler.clone();
            let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
            let cron_handle = tokio::spawn(async move {
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
            });

            // Store cron scheduler and handle
            *self.cron_scheduler.write().await = Some(Arc::new(cron_scheduler));
            handles.cron_scheduler_handle = Some(cron_handle);

            // Phase 3: Create and start CronRecoveryService if cron is enabled
            if self.config.cron_enable_recovery {
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
                let dal = DAL::new(self.database.pool());
                let recovery_service = crate::CronRecoveryService::new(
                    Arc::new(dal),
                    Arc::new(self.clone()), // self implements PipelineExecutor!
                    recovery_config,
                    recovery_shutdown_rx,
                );

                // Start recovery background service
                let mut recovery_service_clone = recovery_service.clone();
                let mut broadcast_shutdown_rx = shutdown_tx.subscribe();
                let recovery_handle = tokio::spawn(async move {
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
                });

                // Store recovery service and handle
                *self.cron_recovery.write().await = Some(Arc::new(recovery_service));
                handles.cron_recovery_handle = Some(recovery_handle);
            }
        }

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

        // Wait for cron scheduler to finish (if enabled)
        if let Some(handle) = handles.cron_scheduler_handle.take() {
            let _ = handle.await;
        }

        // Wait for cron recovery service to finish (if enabled)
        if let Some(handle) = handles.cron_recovery_handle.take() {
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
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get pipeline execution: {}", e),
            })?;

        let task_executions = dal
            .task_execution()
            .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get task executions: {}", e),
            })?;

        // Get final context using DAL
        let final_context = if let Some(context_id) = pipeline_execution.context_id {
            dal.context()
                .read(context_id)
                .await
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

/// Implementation of PipelineExecutor trait for DefaultRunner
///
/// This implementation provides the core workflow execution functionality:
/// - Synchronous and asynchronous execution
/// - Status monitoring and result retrieval
/// - Execution cancellation
/// - Execution listing and management
#[async_trait]
impl PipelineExecutor for DefaultRunner {
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
                .await
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
        callback: Box<dyn crate::executor::pipeline_executor::StatusCallback>,
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
            .await
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
            .await
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

        let executions = dal
            .pipeline_execution()
            .list_recent(100)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to list executions: {}", e),
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
        DefaultRunner::shutdown(self).await
    }
}

impl DefaultRunner {
    /// Register a workflow to run on a cron schedule
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to schedule
    /// * `cron_expression` - Cron expression (e.g., "0 9 * * *" for daily at 9 AM)
    /// * `timezone` - Timezone for interpreting the cron expression (e.g., "UTC", "America/New_York")
    ///
    /// # Returns
    /// * `Result<UniversalUuid, PipelineError>` - The ID of the created schedule or an error
    ///
    /// # Example
    /// ```rust
    /// let runner = DefaultRunner::new("postgresql://localhost/db").await?;
    ///
    /// // Schedule daily backup at 2 AM UTC
    /// runner.register_cron_workflow("backup_workflow", "0 2 * * *", "UTC").await?;
    ///
    /// // Schedule hourly reports during business hours in Eastern time
    /// runner.register_cron_workflow("hourly_report", "0 9-17 * * 1-5", "America/New_York").await?;
    /// ```
    pub async fn register_cron_workflow(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: &str,
    ) -> Result<UniversalUuid, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled. Use enable_cron_scheduling(true) in config."
                    .to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());

        // Validate cron expression and timezone
        use crate::CronEvaluator;
        CronEvaluator::validate(cron_expression, timezone).map_err(|e| {
            PipelineError::Configuration {
                message: format!("Invalid cron expression or timezone: {}", e),
            }
        })?;

        // Calculate initial next run time
        let evaluator = CronEvaluator::new(cron_expression, timezone).map_err(|e| {
            PipelineError::Configuration {
                message: format!("Failed to create cron evaluator: {}", e),
            }
        })?;

        let now = chrono::Utc::now();
        // Calculate next run time from now, ensuring it's in the future
        let next_run = evaluator
            .next_execution(now)
            .map_err(|e| PipelineError::Configuration {
                message: format!("Failed to calculate next execution: {}", e),
            })?;

        // Create the schedule
        use crate::database::universal_types::{UniversalBool, UniversalTimestamp};
        use crate::models::cron_schedule::NewCronSchedule;

        let new_schedule = NewCronSchedule {
            workflow_name: workflow_name.to_string(),
            cron_expression: cron_expression.to_string(),
            timezone: Some(timezone.to_string()),
            enabled: Some(UniversalBool::new(true)),
            catchup_policy: Some("skip".to_string()),
            start_date: None,
            end_date: None,
            next_run_at: UniversalTimestamp(next_run),
        };

        let schedule = dal
            .cron_schedule()
            .create(new_schedule)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to create cron schedule: {}", e),
            })?;

        Ok(schedule.id)
    }

    /// List all registered cron schedules
    ///
    /// # Arguments
    /// * `enabled_only` - If true, only return enabled schedules
    /// * `limit` - Maximum number of schedules to return
    /// * `offset` - Number of schedules to skip for pagination
    ///
    /// # Returns
    /// * `Result<Vec<CronSchedule>, PipelineError>` - List of cron schedules
    pub async fn list_cron_schedules(
        &self,
        enabled_only: bool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::cron_schedule::CronSchedule>, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());
        dal.cron_schedule()
            .list(enabled_only, limit, offset)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to list cron schedules: {}", e),
            })
    }

    /// Enable or disable a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to modify
    /// * `enabled` - Whether to enable (true) or disable (false) the schedule
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error
    pub async fn set_cron_schedule_enabled(
        &self,
        schedule_id: UniversalUuid,
        enabled: bool,
    ) -> Result<(), PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());

        if enabled {
            dal.cron_schedule().enable(schedule_id).await
        } else {
            dal.cron_schedule().disable(schedule_id).await
        }
        .map_err(|e| PipelineError::ExecutionFailed {
            message: format!("Failed to update cron schedule: {}", e),
        })
    }

    /// Delete a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to delete
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error
    pub async fn delete_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<(), PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());
        dal.cron_schedule()
            .delete(schedule_id)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to delete cron schedule: {}", e),
            })
    }

    /// Get a specific cron schedule by ID
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to retrieve
    ///
    /// # Returns
    /// * `Result<CronSchedule, PipelineError>` - The cron schedule or an error
    pub async fn get_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
    ) -> Result<crate::models::cron_schedule::CronSchedule, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());
        dal.cron_schedule()
            .get_by_id(schedule_id)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron schedule: {}", e),
            })
    }

    /// Update a cron schedule's expression and/or timezone
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule to update
    /// * `cron_expression` - New cron expression (optional)
    /// * `timezone` - New timezone (optional)
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error
    pub async fn update_cron_schedule(
        &self,
        schedule_id: UniversalUuid,
        cron_expression: Option<&str>,
        timezone: Option<&str>,
    ) -> Result<(), PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());

        // Validate inputs if provided
        if let (Some(expr), Some(tz)) = (cron_expression, timezone) {
            use crate::CronEvaluator;
            CronEvaluator::validate(expr, tz).map_err(|e| PipelineError::Configuration {
                message: format!("Invalid cron expression or timezone: {}", e),
            })?;
        }

        // Get current schedule
        let mut schedule = dal
            .cron_schedule()
            .get_by_id(schedule_id)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron schedule: {}", e),
            })?;

        // Update fields if provided
        if let Some(expr) = cron_expression {
            schedule.cron_expression = expr.to_string();
        }
        if let Some(tz) = timezone {
            schedule.timezone = tz.to_string();
        }

        // Calculate new next run time
        use crate::CronEvaluator;
        let evaluator =
            CronEvaluator::new(&schedule.cron_expression, &schedule.timezone).map_err(|e| {
                PipelineError::Configuration {
                    message: format!("Failed to create cron evaluator: {}", e),
                }
            })?;

        let now = chrono::Utc::now();
        let next_run = evaluator
            .next_execution(now)
            .map_err(|e| PipelineError::Configuration {
                message: format!("Failed to calculate next execution: {}", e),
            })?;

        // Update the schedule with new expression, timezone, and next run time
        dal.cron_schedule()
            .update_expression_and_timezone(schedule_id, cron_expression, timezone, next_run)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to update cron schedule: {}", e),
            })?;

        Ok(())
    }

    /// Get execution history for a cron schedule
    ///
    /// # Arguments
    /// * `schedule_id` - UUID of the schedule
    /// * `limit` - Maximum number of executions to return
    /// * `offset` - Number of executions to skip for pagination
    ///
    /// # Returns
    /// * `Result<Vec<CronExecution>, PipelineError>` - List of cron executions
    pub async fn get_cron_execution_history(
        &self,
        schedule_id: UniversalUuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::models::cron_execution::CronExecution>, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());
        dal.cron_execution()
            .get_by_schedule_id(schedule_id, limit, offset)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron execution history: {}", e),
            })
    }

    /// Get cron execution statistics
    ///
    /// # Arguments
    /// * `since` - Only include executions since this timestamp
    ///
    /// # Returns
    /// * `Result<CronExecutionStats, PipelineError>` - Execution statistics
    pub async fn get_cron_execution_stats(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<crate::dal::CronExecutionStats, PipelineError> {
        if !self.config.enable_cron_scheduling {
            return Err(PipelineError::Configuration {
                message: "Cron scheduling not enabled.".to_string(),
            });
        }

        let dal = DAL::new(self.database.pool());
        dal.cron_execution()
            .get_execution_stats(since)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get cron execution stats: {}", e),
            })
    }
}

impl Clone for DefaultRunner {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            config: self.config.clone(),
            scheduler: self.scheduler.clone(),
            executor: self.executor.clone(),
            runtime_handles: self.runtime_handles.clone(),
            cron_scheduler: self.cron_scheduler.clone(),
            cron_recovery: self.cron_recovery.clone(),
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
