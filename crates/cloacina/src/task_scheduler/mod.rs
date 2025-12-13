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

//! # Task Scheduler
//!
//! The Task Scheduler converts Workflow definitions into persistent database execution plans
//! and manages task readiness based on dependencies and trigger rules.
//!
//! ## Overview
//!
//! The scheduler builds on existing Cloacina components:
//! - **Workflow**: Task definitions and dependency graphs
//! - **Context**: Type-safe serializable execution context
//! - **Database**: Persistent execution state tracking
//! - **DAL**: Data access layer for database operations
//!
//! ## Key Features
//!
//! - Convert Workflow instances into database execution plans
//! - Manage task state transitions based on dependencies
//! - Support advanced trigger rules for conditional execution
//! - Coordinate with executor through database state
//! - Automatic recovery of orphaned tasks
//! - Context management and merging for task dependencies
//!
//! ## Task State Management
//!
//! Tasks transition through the following states:
//! - **NotStarted**: Initial state when task is created
//! - **Pending**: Waiting for dependencies to complete
//! - **Ready**: Dependencies satisfied, ready for execution
//! - **Running**: Currently being executed
//! - **Completed**: Successfully finished
//! - **Failed**: Execution failed
//! - **Skipped**: Skipped due to trigger rules
//! - **Abandoned**: Permanently failed after recovery attempts
//!
//! ## Error Handling & Recovery
//!
//! The scheduler implements robust error handling and recovery:
//! - Automatic detection of orphaned tasks (stuck in Running state)
//! - Configurable retry policies with maximum attempts
//! - Graceful handling of missing workflows
//! - Detailed recovery event logging
//! - Pipeline-level failure propagation
//!
//! ## Context Management
//!
//! Context handling follows these rules:
//! - Initial context provided at workflow execution
//! - Single dependency: inherits context directly
//! - Multiple dependencies: merges contexts with later overrides
//! - Type-safe serialization/deserialization
//! - Validation of context values in trigger rules
//!
//! ## Performance Considerations
//!
//! - Scheduling loop runs every second by default
//! - Efficient database queries for task state updates
//! - Batch processing of task readiness checks
//! - Optimized context merging for multiple dependencies
//! - Minimal database locking for concurrent operations
//!
//! ## Thread Safety
//!
//! The scheduler is designed for concurrent operation:
//! - Thread-safe database operations
//! - Atomic task state transitions
//! - Safe context merging for parallel tasks
//! - Lock-free trigger rule evaluation
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cloacina::{workflow, task, Context, Database, TaskError};
//! use cloacina::scheduler::TaskScheduler;
//!
//! // Define tasks
//! #[task(id = "fetch-data", dependencies = [])]
//! async fn fetch_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
//!     context.insert("data", serde_json::json!({"status": "fetched"}))?;
//!     Ok(())
//! }
//!
//! // Create workflow
//! let workflow = workflow! {
//!     name: "data-pipeline",
//!     description: "Simple data processing pipeline",
//!     tasks: [fetch_data]
//! };
//!
//! // Schedule execution
//! let database = Database::new("postgresql://localhost/cloacina")?;
//! let scheduler = TaskScheduler::new(database, vec![workflow]);
//! let input_context = Context::new();
//! let execution_id = scheduler.schedule_workflow_execution("data-pipeline", input_context).await?;
//!
//! // Run scheduling loop
//! scheduler.run_scheduling_loop().await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod context_manager;
mod recovery;
mod scheduler_loop;
mod state_manager;
mod trigger_rules;

// Re-export public types
pub use trigger_rules::{TriggerCondition, TriggerRule, ValueOperator};

use std::sync::Arc;
use std::time::Duration;

use diesel::prelude::*;
use diesel::Connection;
use tracing::info;
use uuid::Uuid;

use crate::dal::unified::models::{NewUnifiedPipelineExecution, NewUnifiedTaskExecution};
use crate::dal::DAL;
use crate::database::schema::unified::{pipeline_executions, task_executions};
use crate::database::universal_types::{UniversalTimestamp, UniversalUuid};
use crate::dispatcher::Dispatcher;
use crate::error::ValidationError;
use crate::task::TaskNamespace;
use crate::{Context, Database, Workflow};

use recovery::RecoveryManager;
use scheduler_loop::SchedulerLoop;

/// The main Task Scheduler that manages workflow execution and task readiness.
///
/// The TaskScheduler converts Workflow definitions into persistent database execution plans,
/// tracks task state transitions, and manages dependencies through trigger rules.
///
/// # Thread Safety
///
/// The TaskScheduler is designed to be thread-safe and can be shared across multiple threads.
/// All database operations are performed through a connection pool, and state transitions
/// are handled atomically.
///
/// # Error Handling
///
/// The scheduler implements comprehensive error handling:
/// - Database errors are wrapped in ValidationError
/// - Workflow validation errors are caught early
/// - Recovery errors are logged and tracked
/// - Context evaluation errors are handled gracefully
///
/// # Performance
///
/// The scheduler is optimized for:
/// - Efficient database operations
/// - Minimal locking
/// - Batch processing where possible
/// - Memory-efficient context management
///
/// # Examples
///
/// ```rust,ignore
/// use cloacina::{Database, TaskScheduler};
/// use cloacina::workflow::Workflow;
///
/// // Create a new scheduler with recovery
/// let database = Database::new("postgresql://localhost/cloacina")?;
/// let scheduler = TaskScheduler::with_global_workflows_and_recovery(database).await?;
///
/// // Run the scheduling loop
/// scheduler.run_scheduling_loop().await?;
/// ```
pub struct TaskScheduler {
    dal: DAL,
    instance_id: Uuid,
    poll_interval: Duration,
    /// Optional dispatcher for push-based task execution
    dispatcher: Option<Arc<dyn Dispatcher>>,
}

impl TaskScheduler {
    /// Creates a new TaskScheduler instance with default configuration using global workflow registry.
    ///
    /// This is the recommended constructor for most use cases. The TaskScheduler will:
    /// - Use all workflows registered in the global registry
    /// - Enable automatic recovery of orphaned tasks
    /// - Use default poll interval (100ms)
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    ///
    /// # Returns
    ///
    /// A new TaskScheduler instance ready to schedule and manage workflow executions.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::{Database, TaskScheduler};
    ///
    /// let database = Database::new("postgresql://localhost/cloacina")?;
    /// let scheduler = TaskScheduler::new(database).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// May return ValidationError if recovery operations fail.
    pub async fn new(database: Database) -> Result<Self, ValidationError> {
        let scheduler = Self::with_poll_interval(database, Duration::from_millis(100)).await?;
        Ok(scheduler)
    }

    /// Creates a new TaskScheduler with custom poll interval using global workflow registry.
    ///
    /// Uses all workflows registered in the global registry and enables automatic recovery.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    /// * `poll_interval` - How often to check for ready tasks
    ///
    /// # Returns
    ///
    /// A new TaskScheduler instance ready to schedule and manage workflow executions.
    ///
    /// # Errors
    ///
    /// May return ValidationError if recovery operations fail.
    pub async fn with_poll_interval(
        database: Database,
        poll_interval: Duration,
    ) -> Result<Self, ValidationError> {
        let scheduler = Self::with_poll_interval_sync(database, poll_interval);
        let recovery_manager = RecoveryManager::new(&scheduler.dal);
        recovery_manager.recover_orphaned_tasks().await?;
        Ok(scheduler)
    }

    /// Creates a new TaskScheduler with custom poll interval (synchronous version).
    pub(crate) fn with_poll_interval_sync(database: Database, poll_interval: Duration) -> Self {
        let dal = DAL::new(database.clone());

        Self {
            dal,
            instance_id: Uuid::new_v4(),
            poll_interval,
            dispatcher: None,
        }
    }

    /// Sets the dispatcher for push-based task execution.
    ///
    /// When a dispatcher is configured, the scheduler will dispatch task events
    /// when tasks become ready, in addition to marking them Ready in the database.
    ///
    /// # Arguments
    ///
    /// * `dispatcher` - The dispatcher to use for task events
    ///
    /// # Returns
    ///
    /// Self for method chaining
    pub fn with_dispatcher(mut self, dispatcher: Arc<dyn Dispatcher>) -> Self {
        self.dispatcher = Some(dispatcher);
        self
    }

    /// Returns a reference to the dispatcher if configured.
    pub fn dispatcher(&self) -> Option<&Arc<dyn Dispatcher>> {
        self.dispatcher.as_ref()
    }

    /// Schedules a new workflow execution with the provided input context.
    ///
    /// This method:
    /// 1. Validates the workflow exists in the registry
    /// 2. Stores the input context in the database
    /// 3. Creates a new pipeline execution record
    /// 4. Initializes task execution records for all workflow tasks
    ///
    /// # Arguments
    ///
    /// * `workflow_name` - Name of the workflow to execute
    /// * `input_context` - Context containing input data for the workflow
    ///
    /// # Returns
    ///
    /// The UUID of the created pipeline execution on success.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::{Context, TaskScheduler};
    /// use serde_json::json;
    ///
    /// let scheduler = TaskScheduler::new(database).await?;
    /// let mut context = Context::new();
    /// context.insert("input", json!({"key": "value"}))?;
    ///
    /// let execution_id = scheduler.schedule_workflow_execution("my-workflow", context).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::WorkflowNotFound` if the workflow doesn't exist in the registry,
    /// or other validation errors if database operations fail.
    ///
    /// # Performance
    ///
    /// This operation performs multiple database transactions:
    /// - Context storage
    /// - Pipeline execution creation
    /// - Task execution initialization
    ///
    /// All operations are performed in a single transaction for consistency.
    pub async fn schedule_workflow_execution(
        &self,
        workflow_name: &str,
        input_context: Context<serde_json::Value>,
    ) -> Result<Uuid, ValidationError> {
        info!("Scheduling workflow execution: {}", workflow_name);

        // Look up workflow in global registry
        let workflow = {
            let global_registry = crate::workflow::global_workflow_registry();
            let registry_guard = global_registry.read();

            if let Some(constructor) = registry_guard.get(workflow_name) {
                constructor()
            } else {
                return Err(ValidationError::WorkflowNotFound(workflow_name.to_string()));
            }
        };

        let current_version = workflow.metadata().version.clone();
        let last_version = self
            .dal
            .pipeline_execution()
            .get_last_version(workflow_name)
            .await?;

        if last_version.as_ref() != Some(&current_version) {
            info!(
                "Workflow '{}' version changed: {} -> {}",
                workflow_name,
                last_version.unwrap_or_else(|| "none".to_string()),
                current_version
            );
        }

        // Store context first (separate operation - needed before main transaction)
        let stored_context = self.dal.context().create(&input_context).await?;

        // Build all task data BEFORE the transaction
        let task_ids = workflow.topological_sort()?;
        let mut task_data: Vec<(String, String, String, i32)> = Vec::with_capacity(task_ids.len());

        for task_id in &task_ids {
            let trigger_rules = self.get_task_trigger_rules(&workflow, task_id);
            let task_config = self.get_task_configuration(&workflow, task_id);
            let max_attempts = workflow
                .get_task(task_id)
                .map(|t| t.retry_policy().max_attempts)
                .unwrap_or(3);

            task_data.push((
                task_id.to_string(),
                trigger_rules.to_string(),
                task_config.to_string(),
                max_attempts,
            ));
        }

        // Prepare pipeline data
        let pipeline_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let pipeline_name = workflow_name.to_string();
        let pipeline_version = current_version.clone();

        // Create pipeline AND tasks in a single atomic transaction
        // This prevents the race condition where the scheduler sees a pipeline before tasks exist
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_pipeline_postgres(
                pipeline_id,
                now,
                pipeline_name,
                pipeline_version,
                stored_context,
                task_data,
            )
            .await?,
            self.create_pipeline_sqlite(
                pipeline_id,
                now,
                pipeline_name,
                pipeline_version,
                stored_context,
                task_data,
            )
            .await?
        );

        info!("Workflow execution scheduled: {}", pipeline_id);
        Ok(pipeline_id.into())
    }

    /// Creates pipeline and tasks in PostgreSQL.
    #[cfg(feature = "postgres")]
    async fn create_pipeline_postgres(
        &self,
        pipeline_id: UniversalUuid,
        now: UniversalTimestamp,
        pipeline_name: String,
        pipeline_version: String,
        stored_context: Option<UniversalUuid>,
        task_data: Vec<(String, String, String, i32)>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database()
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction(|conn| {
                // Insert pipeline
                diesel::insert_into(pipeline_executions::table)
                    .values(&NewUnifiedPipelineExecution {
                        id: pipeline_id,
                        pipeline_name,
                        pipeline_version,
                        status: "Pending".to_string(),
                        context_id: stored_context,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    })
                    .execute(conn)?;

                // Insert all tasks
                for (task_name, trigger_rules, task_config, max_attempts) in task_data {
                    diesel::insert_into(task_executions::table)
                        .values(&NewUnifiedTaskExecution {
                            id: UniversalUuid::new_v4(),
                            pipeline_execution_id: pipeline_id,
                            task_name,
                            status: "NotStarted".to_string(),
                            attempt: 1,
                            max_attempts,
                            trigger_rules,
                            task_configuration: task_config,
                            created_at: now,
                            updated_at: now,
                        })
                        .execute(conn)?;
                }

                Ok::<_, diesel::result::Error>(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Creates pipeline and tasks in SQLite.
    #[cfg(feature = "sqlite")]
    async fn create_pipeline_sqlite(
        &self,
        pipeline_id: UniversalUuid,
        now: UniversalTimestamp,
        pipeline_name: String,
        pipeline_version: String,
        stored_context: Option<UniversalUuid>,
        task_data: Vec<(String, String, String, i32)>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database()
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction(|conn| {
                // Insert pipeline
                diesel::insert_into(pipeline_executions::table)
                    .values(&NewUnifiedPipelineExecution {
                        id: pipeline_id,
                        pipeline_name,
                        pipeline_version,
                        status: "Pending".to_string(),
                        context_id: stored_context,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    })
                    .execute(conn)?;

                // Insert all tasks
                for (task_name, trigger_rules, task_config, max_attempts) in task_data {
                    diesel::insert_into(task_executions::table)
                        .values(&NewUnifiedTaskExecution {
                            id: UniversalUuid::new_v4(),
                            pipeline_execution_id: pipeline_id,
                            task_name,
                            status: "NotStarted".to_string(),
                            attempt: 1,
                            max_attempts,
                            trigger_rules,
                            task_configuration: task_config,
                            created_at: now,
                            updated_at: now,
                        })
                        .execute(conn)?;
                }

                Ok::<_, diesel::result::Error>(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }

    /// Runs the main scheduling loop that continuously processes active pipeline executions.
    ///
    /// This loop:
    /// 1. Checks for active pipeline executions
    /// 2. Updates task readiness based on dependencies and trigger rules
    /// 3. Marks completed pipelines
    /// 4. Repeats every second
    ///
    /// # Returns
    ///
    /// This method runs indefinitely until an error occurs.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use cloacina::TaskScheduler;
    ///
    /// let scheduler = TaskScheduler::with_global_workflows(database);
    /// scheduler.run_scheduling_loop().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns validation errors if database operations fail during the scheduling loop.
    /// The loop will continue running on non-fatal errors, with errors logged.
    ///
    /// # Performance
    ///
    /// The scheduling loop:
    /// - Runs every second by default
    /// - Processes all active pipelines in each iteration
    /// - Uses efficient batch queries where possible
    /// - Implements backoff for database errors
    ///
    /// # Thread Safety
    ///
    /// The scheduling loop is designed to be run in a separate thread or task.
    /// Multiple instances should not be run simultaneously.
    pub async fn run_scheduling_loop(&self) -> Result<(), ValidationError> {
        let scheduler_loop = SchedulerLoop::with_dispatcher(
            &self.dal,
            self.instance_id,
            self.poll_interval,
            self.dispatcher.clone(),
        );
        scheduler_loop.run().await
    }

    /// Processes all active pipeline executions to update task readiness.
    pub async fn process_active_pipelines(&self) -> Result<(), ValidationError> {
        let scheduler_loop = SchedulerLoop::with_dispatcher(
            &self.dal,
            self.instance_id,
            self.poll_interval,
            self.dispatcher.clone(),
        );
        scheduler_loop.process_active_pipelines().await
    }

    /// Gets trigger rules for a specific task from the task implementation.
    fn get_task_trigger_rules(
        &self,
        workflow: &Workflow,
        task_namespace: &TaskNamespace,
    ) -> serde_json::Value {
        workflow
            .get_task(task_namespace)
            .map(|task| task.trigger_rules())
            .unwrap_or_else(|_| serde_json::json!({"type": "Always"}))
    }

    /// Gets task configuration (currently returns empty object).
    fn get_task_configuration(
        &self,
        _workflow: &Workflow,
        _task_namespace: &TaskNamespace,
    ) -> serde_json::Value {
        // In the future, this could include task-specific configuration
        serde_json::json!({})
    }
}
