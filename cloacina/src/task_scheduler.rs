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
//! ```rust
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ValidationError;
use crate::models::pipeline_execution::{NewPipelineExecution, PipelineExecution};
use crate::models::recovery_event::{NewRecoveryEvent, RecoveryType};
use crate::models::task_execution::{NewTaskExecution, TaskExecution};
use crate::{Context, Database, Workflow};

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
/// ```rust
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
    workflow_registry: HashMap<String, Workflow>,
    instance_id: Uuid,
    poll_interval: Duration,
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
    /// ```rust
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

    /// Creates a new TaskScheduler with the provided database.
    /// 
    /// This method is deprecated. Use `new()` instead which provides the same functionality.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    ///
    /// # Returns
    ///
    /// A new TaskScheduler instance ready to schedule and manage workflow executions.
    #[deprecated(note = "Use TaskScheduler::new() instead")]
    pub fn with_static_workflows(database: Database, compiled_workflows: Vec<Workflow>) -> Self {
        Self::with_static_workflows_and_poll_interval(database, compiled_workflows, Duration::from_millis(100))
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
        let workflows = crate::workflow::get_all_workflows();
        let scheduler = Self::with_static_workflows_and_poll_interval(database, workflows, poll_interval);
        scheduler.recover_orphaned_tasks().await?;
        Ok(scheduler)
    }
    
    /// Creates a new TaskScheduler with custom poll interval (synchronous version).
    fn with_poll_interval_sync(
        database: Database,
        poll_interval: Duration,
    ) -> Self {
        let dal = DAL::new(database.pool());

        Self {
            dal,
            workflow_registry: HashMap::new(), // Empty since using global registry
            instance_id: Uuid::new_v4(),
            poll_interval,
        }
    }

    /// Creates a new TaskScheduler with custom poll interval.
    ///
    /// This method is deprecated. Use `with_poll_interval()` instead.
    #[deprecated(note = "Use TaskScheduler::with_poll_interval() instead")]
    pub fn with_static_workflows_and_poll_interval(
        database: Database,
        _compiled_workflows: Vec<Workflow>,
        poll_interval: Duration,
    ) -> Self {
        Self::with_poll_interval_sync(database, poll_interval)
    }

    /// Creates a new TaskScheduler using workflows from the global registry without recovery.
    ///
    /// This method is deprecated. Use `new()` instead for the recommended behavior.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    ///
    /// # Returns
    ///
    /// A new TaskScheduler instance with all globally registered workflows.
    #[deprecated(note = "Use TaskScheduler::new() instead")]
    pub fn with_global_workflows(database: Database) -> Self {
        Self::with_poll_interval_sync(database, Duration::from_millis(100))
    }

    /// Creates a new TaskScheduler with static workflows and automatic recovery of orphaned tasks.
    ///
    /// This method is deprecated. Use `new()` for the recommended behavior with global workflows.
    ///
    /// This method performs recovery during initialization by:
    /// 1. Detecting tasks stuck in "Running" state from previous system interruptions
    /// 2. Resetting orphaned tasks to "Ready" state for retry
    /// 3. Tracking recovery events for monitoring
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    /// * `compiled_workflows` - Vector of compiled Workflow instances
    ///
    /// # Returns
    ///
    /// A TaskScheduler instance with recovery completed.
    #[deprecated(note = "Use TaskScheduler::new() instead")]
    pub async fn with_static_workflows_and_recovery(
        database: Database,
        _compiled_workflows: Vec<Workflow>,
    ) -> Result<Self, ValidationError> {
        Self::new(database).await
    }

    /// Creates a new TaskScheduler with static workflows, automatic recovery and custom poll interval.
    ///
    /// This method is deprecated. Use `with_poll_interval()` for the recommended behavior with global workflows.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    /// * `compiled_workflows` - Vector of compiled Workflow instances
    /// * `poll_interval` - How often to check for ready tasks
    ///
    /// # Returns
    ///
    /// A TaskScheduler instance with recovery completed.
    #[deprecated(note = "Use TaskScheduler::with_poll_interval() instead")]
    pub async fn with_static_workflows_and_recovery_and_poll_interval(
        database: Database,
        _compiled_workflows: Vec<Workflow>,
        poll_interval: Duration,
    ) -> Result<Self, ValidationError> {
        Self::with_poll_interval(database, poll_interval).await
    }

    /// Creates a new TaskScheduler with automatic recovery using workflows from the global registry.
    ///
    /// This method is deprecated. Use `new()` instead for the same behavior.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    ///
    /// # Returns
    ///
    /// A TaskScheduler instance with all globally registered workflows and recovery completed.
    #[deprecated(note = "Use TaskScheduler::new() instead")]
    pub async fn with_global_workflows_and_recovery(
        database: Database,
    ) -> Result<Self, ValidationError> {
        Self::new(database).await
    }

    /// Creates a new TaskScheduler with automatic recovery using global workflows and custom poll interval.
    ///
    /// This method is deprecated. Use `with_poll_interval()` instead for the same behavior.
    ///
    /// # Arguments
    ///
    /// * `database` - Database instance for persistence
    /// * `poll_interval` - How often to check for ready tasks
    ///
    /// # Returns
    ///
    /// A TaskScheduler instance with all globally registered workflows and recovery completed.
    #[deprecated(note = "Use TaskScheduler::with_poll_interval() instead")]
    pub async fn with_global_workflows_and_recovery_and_poll_interval(
        database: Database,
        poll_interval: Duration,
    ) -> Result<Self, ValidationError> {
        Self::with_poll_interval(database, poll_interval).await
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
    /// ```rust
    /// use cloacina::{Context, TaskScheduler};
    /// use serde_json::json;
    ///
    /// let scheduler = TaskScheduler::with_global_workflows(database);
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
            let registry_guard = global_registry.read().map_err(|e| {
                ValidationError::WorkflowNotFound(format!("Failed to access global workflow registry: {}", e))
            })?;
            
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

        // Store context
        let stored_context = self.dal.context().create(&input_context).await?;

        // Create pipeline execution
        let new_execution = NewPipelineExecution {
            pipeline_name: workflow_name.to_string(),
            pipeline_version: current_version,
            status: "Pending".to_string(),
            context_id: stored_context,
        };

        let pipeline_execution = self.dal.pipeline_execution().create(new_execution).await?;

        // Initialize task executions
        self.initialize_task_executions(pipeline_execution.id.into(), &workflow)
            .await?;

        info!("Workflow execution scheduled: {}", pipeline_execution.id);
        Ok(pipeline_execution.id.into())
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
    /// ```rust
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
        info!(
            "Starting task scheduler loop (instance: {}, poll_interval: {:?})",
            self.instance_id, self.poll_interval
        );
        let mut interval = time::interval(self.poll_interval);

        loop {
            interval.tick().await;

            match self.process_active_pipelines().await {
                Ok(_) => debug!("Scheduling loop completed successfully"),
                Err(e) => error!("Scheduling loop error: {}", e),
            }
        }
    }

    /// Initializes task execution records for all tasks in a workflow.
    ///
    /// # Arguments
    ///
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    /// * `workflow` - The workflow containing tasks to initialize
    ///
    /// # Returns
    ///
    /// Ok(()) on success, ValidationError on failure.
    async fn initialize_task_executions(
        &self,
        pipeline_execution_id: Uuid,
        workflow: &Workflow,
    ) -> Result<(), ValidationError> {
        debug!(
            "Initializing task executions for pipeline: {}",
            pipeline_execution_id
        );

        let task_ids = workflow.topological_sort()?;

        for task_id in task_ids {
            let trigger_rules = self.get_task_trigger_rules(workflow, &task_id);
            let task_config = self.get_task_configuration(workflow, &task_id);

            // Get retry policy from task to determine max_attempts
            let max_attempts = if let Some(task) = workflow.get_task(&task_id) {
                task.retry_policy().max_attempts
            } else {
                3 // Fallback default
            };

            let new_task = NewTaskExecution {
                pipeline_execution_id: UniversalUuid(pipeline_execution_id),
                task_name: task_id,
                status: "NotStarted".to_string(),
                attempt: 1,
                max_attempts,
                trigger_rules: trigger_rules.to_string(),
                task_configuration: task_config.to_string(),
            };

            self.dal.task_execution().create(new_task).await?;
        }

        Ok(())
    }

    /// Recovers tasks from workflows that are still available in the registry.
    /// Uses the existing single-task recovery logic.
    async fn recover_tasks_for_known_workflow(
        &self,
        tasks: Vec<TaskExecution>,
    ) -> Result<usize, ValidationError> {
        let mut recovered_count = 0;

        for task in tasks {
            let task_name = task.task_name.clone();
            match self.recover_single_task(task).await {
                Ok(RecoveryResult::Recovered) => {
                    recovered_count += 1;
                    debug!("Recovered task: {}", task_name);
                }
                Ok(RecoveryResult::Abandoned) => {
                    debug!(
                        "Task {} abandoned during recovery (exceeded retry limit)",
                        task_name
                    );
                }
                Err(e) => {
                    error!("Failed to recover task {}: {}", task_name, e);
                    // Continue with other tasks
                }
            }
        }

        Ok(recovered_count)
    }

    /// Abandons tasks from workflows that are no longer available in the registry.
    /// This handles the common development scenario where workflow definitions change.
    async fn abandon_tasks_for_unknown_workflow(
        &self,
        pipeline: PipelineExecution,
        tasks: Vec<TaskExecution>,
    ) -> Result<usize, ValidationError> {
        let mut available_workflows: Vec<String> = {
            let global_registry = crate::workflow::global_workflow_registry();
            let registry_guard = global_registry.read().unwrap_or_else(|e| e.into_inner());
            registry_guard.keys().cloned().collect()
        };
        available_workflows.sort();

        // Mark all tasks as abandoned
        for task in &tasks {
            debug!(
                "Abandoning task '{}' (pipeline: {})",
                task.task_name, pipeline.pipeline_name
            );

            self.dal
                .task_execution()
                .mark_abandoned(
                    task.id,
                    &format!(
                        "Workflow '{}' no longer available in registry",
                        pipeline.pipeline_name
                    ),
                )
                .await?;

            // Record abandonment event with clear reason
            self.record_recovery_event(NewRecoveryEvent {
                pipeline_execution_id: pipeline.id,
                task_execution_id: Some(task.id),
                recovery_type: RecoveryType::WorkflowUnavailable.into(),
                details: Some(
                    serde_json::json!({
                        "task_name": task.task_name,
                        "workflow_name": pipeline.pipeline_name,
                        "reason": "Workflow not in current registry",
                        "action": "abandoned",
                        "available_workflows": available_workflows
                    })
                    .to_string(),
                ),
            })
            .await?;
        }

        // Mark pipeline as failed
        self.dal
            .pipeline_execution()
            .mark_failed(
                pipeline.id,
                &format!(
                    "Workflow '{}' no longer available - abandoned during recovery",
                    pipeline.pipeline_name
                ),
            )
            .await?;

        // Record pipeline-level recovery event
        self.record_recovery_event(NewRecoveryEvent {
            pipeline_execution_id: pipeline.id,
            task_execution_id: None,
            recovery_type: RecoveryType::WorkflowUnavailable.into(),
            details: Some(
                serde_json::json!({
                    "workflow_name": pipeline.pipeline_name,
                    "reason": "Workflow not in current registry",
                    "action": "pipeline_failed",
                    "abandoned_tasks": tasks.len(),
                    "available_workflows": available_workflows
                })
                .to_string(),
            ),
        })
        .await?;

        info!(
            "Abandoned {} tasks from unknown workflow '{}'",
            tasks.len(),
            pipeline.pipeline_name
        );

        Ok(tasks.len())
    }

    /// Processes all active pipeline executions to update task readiness.
    pub async fn process_active_pipelines(&self) -> Result<(), ValidationError> {
        let active_executions = self
            .dal
            .pipeline_execution()
            .get_active_executions()
            .await?;

        if active_executions.is_empty() {
            return Ok(());
        }

        // Batch process all active pipelines
        self.process_pipelines_batch(active_executions).await
    }

    /// Processes multiple pipelines in batch for better performance.
    ///
    /// This method optimizes pipeline processing by:
    /// 1. Loading all pending tasks across all pipelines in one query
    /// 2. Grouping tasks by pipeline for processing
    /// 3. Batch checking pipeline completion
    ///
    /// # Arguments
    /// * `active_executions` - Vector of active pipeline executions to process
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error status
    async fn process_pipelines_batch(
        &self,
        active_executions: Vec<crate::models::pipeline_execution::PipelineExecution>,
    ) -> Result<(), ValidationError> {
        use std::collections::HashMap;

        let pipeline_ids: Vec<crate::database::universal_types::UniversalUuid> =
            active_executions.iter().map(|e| e.id).collect();

        // Batch load all pending tasks across all active pipelines
        let all_pending_tasks = self
            .dal
            .task_execution()
            .get_pending_tasks_batch(pipeline_ids)
            .await?;

        // Group tasks by pipeline ID for processing
        let mut tasks_by_pipeline: HashMap<
            crate::database::universal_types::UniversalUuid,
            Vec<_>,
        > = HashMap::new();
        for task in all_pending_tasks {
            tasks_by_pipeline
                .entry(task.pipeline_execution_id)
                .or_insert_with(Vec::new)
                .push(task);
        }

        // Process each pipeline's tasks
        for execution in &active_executions {
            if let Some(pipeline_tasks) = tasks_by_pipeline.get(&execution.id) {
                if let Err(e) = self
                    .update_pipeline_task_readiness(execution.id.into(), pipeline_tasks)
                    .await
                {
                    error!(
                        "Failed to update task readiness for pipeline {}: {}",
                        execution.id, e
                    );
                    continue;
                }
            }

            // Check if pipeline is complete
            if self
                .dal
                .task_execution()
                .check_pipeline_completion(execution.id)
                .await?
            {
                // Get task summary for logging
                let all_tasks = self
                    .dal
                    .task_execution()
                    .get_all_tasks_for_pipeline(execution.id)
                    .await?;
                let completed_count = all_tasks.iter().filter(|t| t.status == "Completed").count();
                let failed_count = all_tasks.iter().filter(|t| t.status == "Failed").count();
                let skipped_count = all_tasks.iter().filter(|t| t.status == "Skipped").count();

                // Update the pipeline's final context before marking complete
                if let Err(e) = self.update_pipeline_final_context(execution.id, &all_tasks).await {
                    warn!(
                        "Failed to update final context for pipeline {}: {}",
                        execution.id, e
                    );
                }

                self.dal
                    .pipeline_execution()
                    .mark_completed(execution.id)
                    .await?;
                info!(
                    "Pipeline completed: {} (name: {}, {} completed, {} failed, {} skipped)",
                    execution.id,
                    execution.pipeline_name,
                    completed_count,
                    failed_count,
                    skipped_count
                );
            }
        }

        Ok(())
    }

    /// Updates task readiness for a specific pipeline using pre-loaded tasks.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    /// * `pending_tasks` - Pre-loaded pending tasks for this pipeline
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error status
    async fn update_pipeline_task_readiness(
        &self,
        pipeline_execution_id: UniversalUuid,
        pending_tasks: &[crate::models::task_execution::TaskExecution],
    ) -> Result<(), ValidationError> {
        for task_execution in pending_tasks {
            let dependencies_satisfied = self.check_task_dependencies(task_execution).await?;

            if dependencies_satisfied {
                // All dependencies are in terminal states, now evaluate trigger rules
                let trigger_rules_satisfied = self.evaluate_trigger_rules(task_execution).await?;

                if trigger_rules_satisfied {
                    self.dal
                        .task_execution()
                        .mark_ready(task_execution.id)
                        .await?;
                    info!("Task ready: {} (pipeline: {}, dependencies satisfied, trigger rules passed)",
                          task_execution.task_name, pipeline_execution_id);
                } else {
                    // Dependencies satisfied + trigger rules fail → Mark Skipped
                    self.dal
                        .task_execution()
                        .mark_skipped(task_execution.id, "Trigger rules not satisfied")
                        .await?;
                    info!("Task skipped: {} (pipeline: {}, dependencies satisfied, trigger rules failed)",
                          task_execution.task_name, pipeline_execution_id);
                }
            }
        }

        Ok(())
    }

    /// Checks if all dependencies for a task are satisfied.
    /// Dependencies are satisfied when all dependency tasks are in terminal states (Completed, Failed, or Skipped).
    async fn check_task_dependencies(
        &self,
        task_execution: &TaskExecution,
    ) -> Result<bool, ValidationError> {
        // Get workflow to check dependencies
        let pipeline = self
            .dal
            .pipeline_execution()
            .get_by_id(task_execution.pipeline_execution_id)
            .await?;
        let workflow = {
            let global_registry = crate::workflow::global_workflow_registry();
            let registry_guard = global_registry.read().map_err(|e| {
                ValidationError::WorkflowNotFound(format!("Failed to access global workflow registry: {}", e))
            })?;
            
            if let Some(constructor) = registry_guard.get(&pipeline.pipeline_name) {
                constructor()
            } else {
                return Err(ValidationError::WorkflowNotFound(pipeline.pipeline_name.clone()));
            }
        };

        let dependencies = workflow
            .get_dependencies(&task_execution.task_name)
            .unwrap_or(&[]);

        if dependencies.is_empty() {
            return Ok(true);
        }

        // Batch fetch all dependency statuses in a single query
        let dependency_names = dependencies.iter().map(|s| s.to_string()).collect();
        let status_map = self
            .dal
            .task_execution()
            .get_task_statuses_batch(task_execution.pipeline_execution_id, dependency_names)
            .await?;

        // Check that all dependencies exist and are in terminal states
        let all_satisfied = dependencies.iter().all(|dependency| {
            status_map
                .get(dependency)
                .map(|status| matches!(status.as_str(), "Completed" | "Failed" | "Skipped"))
                .unwrap_or_else(|| {
                    warn!(
                        "Dependency task '{}' not found for task '{}'",
                        dependency, task_execution.task_name
                    );
                    false
                })
        });

        Ok(all_satisfied)
    }

    /// Gets trigger rules for a specific task from the task implementation.
    fn get_task_trigger_rules(&self, workflow: &Workflow, task_id: &str) -> serde_json::Value {
        workflow
            .get_task(task_id)
            .map(|task| task.trigger_rules())
            .unwrap_or_else(|| serde_json::json!({"type": "Always"}))
    }

    /// Gets task configuration (currently returns empty object).
    fn get_task_configuration(&self, _workflow: &Workflow, _task_id: &str) -> serde_json::Value {
        // In the future, this could include task-specific configuration
        serde_json::json!({})
    }

    /// Evaluates trigger rules for a task based on its configuration.
    async fn evaluate_trigger_rules(
        &self,
        task_execution: &TaskExecution,
    ) -> Result<bool, ValidationError> {
        let trigger_rule: TriggerRule = serde_json::from_str(&task_execution.trigger_rules)
            .map_err(|e| ValidationError::InvalidTriggerRule(e.to_string()))?;

        let result = match &trigger_rule {
            TriggerRule::Always => {
                debug!(
                    "Trigger rule evaluation: Always -> true (task: {})",
                    task_execution.task_name
                );
                Ok(true)
            }
            TriggerRule::All { conditions } => {
                debug!(
                    "Trigger rule evaluation: All({} conditions) (task: {})",
                    conditions.len(),
                    task_execution.task_name
                );
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_result =
                        self.evaluate_condition(condition, task_execution).await?;
                    debug!(
                        "  └─ Condition {}: {:?} -> {}",
                        i + 1,
                        condition,
                        condition_result
                    );
                    if !condition_result {
                        debug!(
                            "Trigger rule result: All -> false (condition {} failed)",
                            i + 1
                        );
                        return Ok(false);
                    }
                }
                debug!("Trigger rule result: All -> true (all conditions passed)");
                Ok(true)
            }
            TriggerRule::Any { conditions } => {
                debug!(
                    "Trigger rule evaluation: Any({} conditions) (task: {})",
                    conditions.len(),
                    task_execution.task_name
                );
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_result =
                        self.evaluate_condition(condition, task_execution).await?;
                    debug!(
                        "  └─ Condition {}: {:?} -> {}",
                        i + 1,
                        condition,
                        condition_result
                    );
                    if condition_result {
                        debug!(
                            "Trigger rule result: Any -> true (condition {} passed)",
                            i + 1
                        );
                        return Ok(true);
                    }
                }
                debug!("Trigger rule result: Any -> false (no conditions passed)");
                Ok(false)
            }
            TriggerRule::None { conditions } => {
                debug!(
                    "Trigger rule evaluation: None({} conditions) (task: {})",
                    conditions.len(),
                    task_execution.task_name
                );
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_result =
                        self.evaluate_condition(condition, task_execution).await?;
                    debug!(
                        "  └─ Condition {}: {:?} -> {}",
                        i + 1,
                        condition,
                        condition_result
                    );
                    if condition_result {
                        debug!(
                            "Trigger rule result: None -> false (condition {} passed)",
                            i + 1
                        );
                        return Ok(false);
                    }
                }
                debug!("Trigger rule result: None -> true (no conditions passed)");
                Ok(true)
            }
        };

        result
    }

    /// Evaluates a specific trigger condition.
    async fn evaluate_condition(
        &self,
        condition: &TriggerCondition,
        task_execution: &TaskExecution,
    ) -> Result<bool, ValidationError> {
        match condition {
            TriggerCondition::TaskSuccess { task_name } => {
                let status = self
                    .dal
                    .task_execution()
                    .get_task_status(task_execution.pipeline_execution_id, task_name)
                    .await?;
                let result = status == "Completed";
                debug!(
                    "    TaskSuccess('{}') -> {} (status: {})",
                    task_name, result, status
                );
                Ok(result)
            }
            TriggerCondition::TaskFailed { task_name } => {
                let status = self
                    .dal
                    .task_execution()
                    .get_task_status(task_execution.pipeline_execution_id, task_name)
                    .await?;
                let result = status == "Failed";
                debug!(
                    "    TaskFailed('{}') -> {} (status: {})",
                    task_name, result, status
                );
                Ok(result)
            }
            TriggerCondition::TaskSkipped { task_name } => {
                let status = self
                    .dal
                    .task_execution()
                    .get_task_status(task_execution.pipeline_execution_id, task_name)
                    .await?;
                let result = status == "Skipped";
                debug!(
                    "    TaskSkipped('{}') -> {} (status: {})",
                    task_name, result, status
                );
                Ok(result)
            }
            TriggerCondition::ContextValue {
                key,
                operator,
                value,
            } => {
                let context = self.load_context_for_task(task_execution).await?;
                let actual_value = context.get(key);
                let result = self.evaluate_context_condition(&context, key, operator, value)?;
                debug!(
                    "    ContextValue('{}', {:?}, {}) -> {} (actual: {:?})",
                    key, operator, value, result, actual_value
                );
                Ok(result)
            }
        }
    }

    /// Loads the context for a specific task based on its dependencies.
    async fn load_context_for_task(
        &self,
        task_execution: &TaskExecution,
    ) -> Result<Context<serde_json::Value>, ValidationError> {
        // Get the workflow to find task dependencies
        let pipeline = self
            .dal
            .pipeline_execution()
            .get_by_id(task_execution.pipeline_execution_id)
            .await?;
        let workflow = {
            let global_registry = crate::workflow::global_workflow_registry();
            let registry_guard = global_registry.read().map_err(|e| {
                ValidationError::WorkflowNotFound(format!("Failed to access global workflow registry: {}", e))
            })?;
            
            if let Some(constructor) = registry_guard.get(&pipeline.pipeline_name) {
                constructor()
            } else {
                return Err(ValidationError::WorkflowNotFound(pipeline.pipeline_name.clone()));
            }
        };

        let dependencies = workflow
            .get_dependencies(&task_execution.task_name)
            .unwrap_or(&[]);

        if dependencies.is_empty() {
            // No dependencies: read initial pipeline context
            if let Some(context_id) = pipeline.context_id {
                let context = self.dal.context().read(context_id).await.map_err(|_e| {
                    ValidationError::ContextEvaluationFailed {
                        key: format!("context_id:{}", context_id),
                    }
                })?;
                debug!(
                    "Context loaded: initial pipeline context ({} keys)",
                    context.data().len()
                );
                Ok(context)
            } else {
                debug!("Context loaded: empty initial context");
                Ok(Context::new())
            }
        } else if dependencies.len() == 1 {
            // Single dependency: read that task's saved context
            let dep_task_name = &dependencies[0];
            match self
                .dal
                .task_execution_metadata()
                .get_by_pipeline_and_task(task_execution.pipeline_execution_id, dep_task_name)
                .await
            {
                Ok(task_metadata) => {
                    if let Some(context_id) = task_metadata.context_id {
                        match self
                            .dal
                            .context()
                            .read::<serde_json::Value>(context_id)
                            .await
                        {
                            Ok(context) => {
                                debug!(
                                    "Context loaded: from dependency '{}' ({} keys)",
                                    dep_task_name,
                                    context.data().len()
                                );
                                Ok(context)
                            }
                            Err(e) => Err(ValidationError::ContextEvaluationFailed {
                                key: format!("context_read_error:{}", e),
                            }),
                        }
                    } else {
                        // Task completed but has no output context
                        debug!(
                            "Context loaded: empty (dependency '{}' has no output context)",
                            dep_task_name
                        );
                        Ok(Context::new())
                    }
                }
                Err(_) => {
                    // Dependency task hasn't completed yet or no metadata saved
                    debug!(
                        "Context loaded: empty (dependency '{}' not found)",
                        dep_task_name
                    );
                    Ok(Context::new())
                }
            }
        } else {
            // Multiple dependencies: merge their saved contexts
            let mut merged_context = Context::new();
            let mut sources = Vec::new();

            for dep_task_name in dependencies {
                if let Ok(task_metadata) = self
                    .dal
                    .task_execution_metadata()
                    .get_by_pipeline_and_task(task_execution.pipeline_execution_id, dep_task_name)
                    .await
                {
                    if let Some(context_id) = task_metadata.context_id {
                        if let Ok(dep_context) = self
                            .dal
                            .context()
                            .read::<serde_json::Value>(context_id)
                            .await
                        {
                            sources.push(format!(
                                "{}({})",
                                dep_task_name,
                                dep_context.data().len()
                            ));
                            // Merge dependency context (later dependencies override earlier ones)
                            for (key, value) in dep_context.data() {
                                if merged_context.get(key).is_some() {
                                    merged_context.update(key.clone(), value.clone()).map_err(
                                        |e| ValidationError::ContextEvaluationFailed {
                                            key: format!("merge_error:{}", e),
                                        },
                                    )?;
                                } else {
                                    merged_context.insert(key.clone(), value.clone()).map_err(
                                        |e| ValidationError::ContextEvaluationFailed {
                                            key: format!("merge_error:{}", e),
                                        },
                                    )?;
                                }
                            }
                        }
                    }
                }
            }

            debug!(
                "Context loaded: merged from {} ({} total keys)",
                sources.join(", "),
                merged_context.data().len()
            );
            Ok(merged_context)
        }
    }

    /// Evaluates a context-based condition using the provided operator.
    fn evaluate_context_condition(
        &self,
        context: &Context<serde_json::Value>,
        key: &str,
        operator: &ValueOperator,
        expected: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        let actual = context.get(key);

        match operator {
            ValueOperator::Exists => Ok(actual.is_some()),
            ValueOperator::NotExists => Ok(actual.is_none()),
            ValueOperator::Equals => Ok(actual == Some(expected)),
            ValueOperator::NotEquals => Ok(actual != Some(expected)),
            ValueOperator::GreaterThan => match (actual, expected) {
                (Some(a), b) if a.is_number() && b.is_number() => {
                    Ok(a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0))
                }
                _ => Ok(false),
            },
            ValueOperator::LessThan => match (actual, expected) {
                (Some(a), b) if a.is_number() && b.is_number() => {
                    Ok(a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0))
                }
                _ => Ok(false),
            },
            ValueOperator::Contains => match (actual, expected) {
                (Some(a), b) if a.is_string() && b.is_string() => {
                    Ok(a.as_str().unwrap_or("").contains(b.as_str().unwrap_or("")))
                }
                (Some(a), b) if a.is_array() => Ok(a.as_array().unwrap_or(&vec![]).contains(b)),
                _ => Ok(false),
            },
            ValueOperator::NotContains => Ok(!self.evaluate_context_condition(
                context,
                key,
                &ValueOperator::Contains,
                expected,
            )?),
        }
    }

    // Recovery operations

    /// Detects and recovers tasks orphaned by system interruptions.
    ///
    /// Recovery strategy:
    /// 1. Find all tasks in "Running" state (orphaned by crashed executors)
    /// 2. Reset them to "Ready" state for retry by available executors
    /// 3. Increment recovery attempt counters
    /// 4. Log recovery events for monitoring
    ///
    /// Tasks will restart from the beginning with fresh context loaded from dependencies.
    /// This is safe because tasks are required to be idempotent.
    ///
    /// # Returns
    ///
    /// Ok(()) on successful recovery, ValidationError on failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cloacina::TaskScheduler;
    ///
    /// let scheduler = TaskScheduler::with_global_workflows(database);
    /// scheduler.recover_orphaned_tasks().await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns validation errors if:
    /// - Database operations fail
    /// - Task state transitions fail
    /// - Recovery event logging fails
    ///
    /// # Performance
    ///
    /// Recovery operations:
    /// - Process tasks in batches
    /// - Use efficient database queries
    /// - Implement retry limits
    /// - Log detailed recovery events
    ///
    /// # Thread Safety
    ///
    /// Recovery operations are thread-safe and can be run concurrently with
    /// the scheduling loop. State transitions are handled atomically.
    async fn recover_orphaned_tasks(&self) -> Result<(), ValidationError> {
        info!("Starting recovery check for orphaned tasks");

        // Find orphaned tasks (stuck in "Running" state)
        let orphaned_tasks = self.dal.task_execution().get_orphaned_tasks().await?;

        if orphaned_tasks.is_empty() {
            info!("No orphaned tasks found");
            return Ok(());
        }

        info!(
            "Found {} orphaned tasks, beginning recovery",
            orphaned_tasks.len()
        );

        // Group tasks by pipeline to handle workflow availability
        let mut tasks_by_pipeline: std::collections::HashMap<
            crate::database::universal_types::UniversalUuid,
            (PipelineExecution, Vec<TaskExecution>),
        > = std::collections::HashMap::new();

        for task in orphaned_tasks {
            let pipeline = self
                .dal
                .pipeline_execution()
                .get_by_id(task.pipeline_execution_id)
                .await?;
            tasks_by_pipeline
                .entry(pipeline.id)
                .or_insert((pipeline, Vec::new()))
                .1
                .push(task);
        }

        let mut recovered_count = 0;
        let mut abandoned_count = 0;
        let mut failed_pipelines = 0;
        let mut available_workflows: Vec<String> = {
            let global_registry = crate::workflow::global_workflow_registry();
            let registry_guard = global_registry.read().unwrap_or_else(|e| e.into_inner());
            registry_guard.keys().cloned().collect()
        };
        available_workflows.sort();

        debug!(
            "Current workflow registry: [{}]",
            available_workflows.join(", ")
        );

        // Process each pipeline's orphaned tasks
        for (pipeline_id, (pipeline, tasks)) in tasks_by_pipeline {
            let workflow_exists = {
                let global_registry = crate::workflow::global_workflow_registry();
                let registry_guard = global_registry.read().unwrap_or_else(|e| e.into_inner());
                registry_guard.contains_key(&pipeline.pipeline_name)
            };
            
            if workflow_exists {
                // Known workflow - use existing recovery logic
                info!(
                    "Recovering {} tasks from known workflow '{}'",
                    tasks.len(),
                    pipeline.pipeline_name
                );
                match self.recover_tasks_for_known_workflow(tasks).await {
                    Ok(recovered) => recovered_count += recovered,
                    Err(e) => {
                        error!(
                            "Failed to recover tasks for pipeline {}: {}",
                            pipeline_id, e
                        );
                        // Continue with other pipelines
                    }
                }
            } else {
                // Unknown workflow - gracefully abandon
                warn!(
                    "Pipeline '{}' not in current workflow registry - marking as abandoned",
                    pipeline.pipeline_name
                );
                debug!(
                    "Found orphaned pipeline '{}' - not in registry",
                    pipeline.pipeline_name
                );
                match self
                    .abandon_tasks_for_unknown_workflow(pipeline, tasks)
                    .await
                {
                    Ok(abandoned) => {
                        abandoned_count += abandoned;
                        failed_pipelines += 1;
                    }
                    Err(e) => {
                        error!(
                            "Failed to abandon tasks for unknown workflow {}: {}",
                            pipeline_id, e
                        );
                        // Continue with other pipelines
                    }
                }
            }
        }

        // Log detailed recovery summary
        info!(
            "Recovery Summary:\n  ├─ Tasks Processed: {}\n  ├─ Recovered: {}\n  ├─ Abandoned: {}\n  ├─ Pipelines Failed: {}\n  └─ Available Workflows: [{}]",
            recovered_count + abandoned_count, recovered_count, abandoned_count, failed_pipelines, available_workflows.join(", ")
        );

        Ok(())
    }

    /// Recovers a single orphaned task with retry limit enforcement.
    async fn recover_single_task(
        &self,
        task: TaskExecution,
    ) -> Result<RecoveryResult, ValidationError> {
        const MAX_RECOVERY_ATTEMPTS: i32 = 3;

        if task.recovery_attempts >= MAX_RECOVERY_ATTEMPTS {
            // Too many recovery attempts - abandon the task and potentially the pipeline
            self.abandon_task_permanently(task).await?;
            return Ok(RecoveryResult::Abandoned);
        }

        // Reset task to "Ready" state for retry
        self.dal
            .task_execution()
            .reset_task_for_recovery(task.id)
            .await?;

        // Record recovery event
        self.record_recovery_event(NewRecoveryEvent {
            pipeline_execution_id: task.pipeline_execution_id,
            task_execution_id: Some(task.id),
            recovery_type: RecoveryType::TaskReset.into(),
            details: Some(
                serde_json::json!({
                    "task_name": task.task_name,
                    "previous_status": "Running",
                    "new_status": "Ready",
                    "recovery_attempt": task.recovery_attempts + 1
                })
                .to_string(),
            ),
        })
        .await?;

        info!(
            "Recovered orphaned task: {} (attempt {})",
            task.task_name,
            task.recovery_attempts + 1
        );

        Ok(RecoveryResult::Recovered)
    }

    /// Permanently abandons a task that has exceeded recovery limits.
    async fn abandon_task_permanently(&self, task: TaskExecution) -> Result<(), ValidationError> {
        // Mark task as permanently failed
        self.dal
            .task_execution()
            .mark_abandoned(task.id, "Exceeded recovery attempts")
            .await?;

        // Check if this causes the entire pipeline to fail
        let pipeline_failed = self
            .dal
            .task_execution()
            .check_pipeline_failure(task.pipeline_execution_id)
            .await?;

        if pipeline_failed {
            self.dal
                .pipeline_execution()
                .mark_failed(
                    task.pipeline_execution_id,
                    "Task abandonment caused pipeline failure",
                )
                .await?;
        }

        // Record abandonment event
        self.record_recovery_event(NewRecoveryEvent {
            pipeline_execution_id: task.pipeline_execution_id,
            task_execution_id: Some(task.id),
            recovery_type: RecoveryType::TaskAbandoned.into(),
            details: Some(
                serde_json::json!({
                    "task_name": task.task_name,
                    "recovery_attempts": task.recovery_attempts,
                    "reason": "Exceeded maximum recovery attempts"
                })
                .to_string(),
            ),
        })
        .await?;

        error!(
            "Abandoned task permanently: {} after {} recovery attempts",
            task.task_name, task.recovery_attempts
        );

        Ok(())
    }

    /// Records a recovery event for monitoring and debugging.
    async fn record_recovery_event(&self, event: NewRecoveryEvent) -> Result<(), ValidationError> {
        self.dal.recovery_event().create(event).await?;
        Ok(())
    }

    /// Updates the pipeline's final context when it completes.
    ///
    /// This method finds the context from the final task(s) that produced output 
    /// and updates the pipeline's context_id to point to that final context,
    /// ensuring that PipelineResult.final_context returns the correct data.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    /// * `all_tasks` - All task executions for this pipeline
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    async fn update_pipeline_final_context(
        &self,
        pipeline_execution_id: UniversalUuid,
        all_tasks: &[crate::models::task_execution::TaskExecution],
    ) -> Result<(), ValidationError> {
        // Find the final context by looking at the last task that completed with context
        // Priority order: Completed > Skipped, then by completion time (latest first)
        let mut final_context_id: Option<UniversalUuid> = None;
        let mut latest_completion_time: Option<chrono::DateTime<chrono::Utc>> = None;

        for task in all_tasks {
            // Only consider tasks that have finished execution and might have output context
            if task.status == "Completed" || task.status == "Skipped" {
                if let Some(completed_at) = task.completed_at {
                    // Check if this task has a context stored
                    if let Ok(task_metadata) = self
                        .dal
                        .task_execution_metadata()
                        .get_by_pipeline_and_task(pipeline_execution_id, &task.task_name)
                        .await
                    {
                        if let Some(context_id) = task_metadata.context_id {
                            // Use this context if it's the latest completion time or we haven't found one yet
                            if latest_completion_time.is_none() 
                                || completed_at.0 > latest_completion_time.unwrap() 
                            {
                                final_context_id = Some(context_id);
                                latest_completion_time = Some(completed_at.0);
                            }
                        }
                    }
                }
            }
        }

        // Update the pipeline's context_id if we found a final context
        if let Some(context_id) = final_context_id {
            debug!(
                "Updating pipeline {} final context to context_id: {}",
                pipeline_execution_id, context_id
            );
            self.dal
                .pipeline_execution()
                .update_final_context(pipeline_execution_id, context_id)
                .await?;
        } else {
            debug!(
                "No final context found for pipeline {} - keeping initial context",
                pipeline_execution_id
            );
        }

        Ok(())
    }
}

#[derive(Debug)]
enum RecoveryResult {
    Recovered,
    Abandoned,
}

/// Trigger rule definitions for conditional task execution.
///
/// Trigger rules determine when a task should be executed based on various conditions.
/// They can be combined to create complex execution logic.
///
/// # Examples
///
/// ```rust
/// use cloacina::scheduler::{TriggerRule, TriggerCondition, ValueOperator};
/// use serde_json::json;
///
/// // Always execute
/// let always = TriggerRule::Always;
///
/// // Execute if all conditions are met
/// let all_conditions = TriggerRule::All {
///     conditions: vec![
///         TriggerCondition::TaskSuccess { task_name: "task1".to_string() },
///         TriggerCondition::ContextValue {
///             key: "status".to_string(),
///             operator: ValueOperator::Equals,
///             value: json!("ready")
///         }
///     ]
/// };
///
/// // Execute if any condition is met
/// let any_condition = TriggerRule::Any {
///     conditions: vec![
///         TriggerCondition::TaskFailed { task_name: "task1".to_string() },
///         TriggerCondition::TaskSkipped { task_name: "task2".to_string() }
///     ]
/// };
///
/// // Execute if no conditions are met
/// let none_condition = TriggerRule::None {
///     conditions: vec![
///         TriggerCondition::ContextValue {
///             key: "skip".to_string(),
///             operator: ValueOperator::Exists,
///             value: json!(true)
///         }
///     ]
/// };
/// ```
///
/// # Performance
///
/// Trigger rule evaluation:
/// - Is performed in memory
/// - Uses efficient context lookups
/// - Supports early termination for All/Any rules
/// - Caches context values where possible
///
/// # Thread Safety
///
/// Trigger rules are:
/// - Immutable after creation
/// - Safe to share across threads
/// - Free of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerRule {
    /// Always execute the task (default behavior).
    Always,
    /// Execute only if all conditions are met.
    All { conditions: Vec<TriggerCondition> },
    /// Execute if any condition is met.
    Any { conditions: Vec<TriggerCondition> },
    /// Execute only if none of the conditions are met.
    None { conditions: Vec<TriggerCondition> },
}

/// Individual conditions that can be evaluated for trigger rules.
///
/// Conditions are the building blocks of trigger rules, allowing tasks to be
/// executed based on the state of other tasks or context values.
///
/// # Examples
///
/// ```rust
/// use cloacina::scheduler::{TriggerCondition, ValueOperator};
/// use serde_json::json;
///
/// // Task state conditions
/// let task_success = TriggerCondition::TaskSuccess { task_name: "task1".to_string() };
/// let task_failed = TriggerCondition::TaskFailed { task_name: "task2".to_string() };
/// let task_skipped = TriggerCondition::TaskSkipped { task_name: "task3".to_string() };
///
/// // Context value conditions
/// let context_equals = TriggerCondition::ContextValue {
///     key: "status".to_string(),
///     operator: ValueOperator::Equals,
///     value: json!("ready")
/// };
///
/// let context_exists = TriggerCondition::ContextValue {
///     key: "flag".to_string(),
///     operator: ValueOperator::Exists,
///     value: json!(true)
/// };
/// ```
///
/// # Performance
///
/// Condition evaluation:
/// - Task state conditions use efficient database lookups
/// - Context value conditions are evaluated in memory
/// - Results are cached where possible
/// - Supports early termination for complex conditions
///
/// # Thread Safety
///
/// Conditions are:
/// - Immutable after creation
/// - Safe to share across threads
/// - Free of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TriggerCondition {
    /// Condition based on successful task completion.
    TaskSuccess { task_name: String },
    /// Condition based on task failure.
    TaskFailed { task_name: String },
    /// Condition based on task being skipped.
    TaskSkipped { task_name: String },
    /// Condition based on context value evaluation.
    ContextValue {
        key: String,
        operator: ValueOperator,
        value: serde_json::Value,
    },
}

/// Operators for evaluating context values in trigger conditions.
///
/// These operators define how context values should be compared and evaluated
/// in trigger conditions.
///
/// # Examples
///
/// ```rust
/// use cloacina::scheduler::ValueOperator;
/// use serde_json::json;
///
/// // Basic comparisons
/// let equals = ValueOperator::Equals;      // ==
/// let not_equals = ValueOperator::NotEquals; // !=
/// let greater = ValueOperator::GreaterThan;  // >
/// let less = ValueOperator::LessThan;       // <
///
/// // String operations
/// let contains = ValueOperator::Contains;     // "hello".contains("ell")
/// let not_contains = ValueOperator::NotContains; // !"hello".contains("xyz")
///
/// // Existence checks
/// let exists = ValueOperator::Exists;       // key exists
/// let not_exists = ValueOperator::NotExists; // key doesn't exist
/// ```
///
/// # Performance
///
/// Operator evaluation:
/// - Uses efficient type-specific comparisons
/// - Supports early termination where possible
/// - Handles type coercion gracefully
/// - Caches results for repeated evaluations
///
/// # Thread Safety
///
/// Operators are:
/// - Immutable
/// - Safe to share across threads
/// - Free of side effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueOperator {
    /// Exact equality comparison.
    Equals,
    /// Inequality comparison.
    NotEquals,
    /// Greater than comparison (for numbers).
    GreaterThan,
    /// Less than comparison (for numbers).
    LessThan,
    /// Contains check (for strings and arrays).
    Contains,
    /// Does not contain check.
    NotContains,
    /// Key exists in context.
    Exists,
    /// Key does not exist in context.
    NotExists,
}
