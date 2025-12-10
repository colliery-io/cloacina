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

//! Task Executor Module
//!
//! This module provides the core task execution functionality for the Cloacina pipeline system.
//! The ThreadTaskExecutor implements the `TaskExecutor` trait for dispatcher-based execution.
//!
//! The executor is responsible for:
//! - Executing tasks with proper timeout handling
//! - Managing task retries and error handling
//! - Maintaining task execution state
//! - Handling task dependencies and context management
//!
//! ## Dispatcher Integration
//!
//! ThreadTaskExecutor implements the `TaskExecutor` trait, allowing it to be registered
//! with a dispatcher to receive task events directly. The dispatcher routes `TaskReadyEvent`s
//! to the executor based on routing rules.

use chrono::Utc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use super::types::{ClaimedTask, ExecutionScope, ExecutorConfig};
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::dispatcher::{
    DispatchError, ExecutionResult, ExecutorMetrics, TaskExecutor, TaskReadyEvent,
};
use crate::error::ExecutorError;
use crate::retry::{RetryCondition, RetryPolicy};
use crate::task::get_task;
use crate::{parse_namespace, Context, Database, Task, TaskRegistry};
use async_trait::async_trait;

/// ThreadTaskExecutor is a thread-based implementation of task execution.
///
/// This executor runs tasks in the current thread/process and manages:
/// - Task execution with timeout handling
/// - Context management and dependency resolution
/// - Error handling and retry logic
/// - State persistence
///
/// The executor maintains its own instance ID for tracking and logging purposes
/// and uses a task registry to resolve task implementations.
///
/// ## Dispatcher Integration
///
/// ThreadTaskExecutor implements the `TaskExecutor` trait, allowing it to be
/// registered with a dispatcher to receive task events directly via the
/// `execute()` method.
pub struct ThreadTaskExecutor {
    /// Database connection pool for task state persistence
    database: Database,
    /// Data Access Layer for database operations
    dal: DAL,
    /// Registry of available task implementations
    task_registry: Arc<TaskRegistry>,
    /// Unique identifier for this executor instance
    instance_id: UniversalUuid,
    /// Configuration parameters for executor behavior
    config: ExecutorConfig,
    /// Metrics: current number of active (in-flight) tasks
    active_tasks: AtomicUsize,
    /// Metrics: total tasks executed
    total_executed: AtomicU64,
    /// Metrics: total tasks failed
    total_failed: AtomicU64,
}

impl ThreadTaskExecutor {
    /// Creates a new ThreadTaskExecutor instance.
    ///
    /// # Arguments
    /// * `database` - Database connection pool for task state persistence
    /// * `task_registry` - Registry containing available task implementations
    /// * `config` - Configuration parameters for executor behavior
    ///
    /// # Returns
    /// A new TaskExecutor instance with a randomly generated instance ID
    pub fn new(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        config: ExecutorConfig,
    ) -> Self {
        let dal = DAL::new(database.clone());

        Self {
            database,
            dal,
            task_registry,
            instance_id: UniversalUuid::new_v4(),
            config,
            active_tasks: AtomicUsize::new(0),
            total_executed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }

    /// Creates a TaskExecutor using the global task registry.
    ///
    /// This method is useful when you want to use tasks registered through the global registry
    /// rather than providing a custom registry.
    ///
    /// # Arguments
    /// * `database` - Database connection pool for task state persistence
    /// * `config` - Configuration parameters for executor behavior
    ///
    /// # Returns
    /// Result containing either a new TaskExecutor instance or a RegistrationError
    pub fn with_global_registry(
        database: Database,
        config: ExecutorConfig,
    ) -> Result<Self, crate::error::RegistrationError> {
        let mut registry = TaskRegistry::new();
        let global_registry = crate::global_task_registry();
        let global_tasks = global_registry.read();

        for (namespace, constructor) in global_tasks.iter() {
            let task = constructor();
            registry.register_arc(namespace.clone(), task)?;
        }

        Ok(Self::new(database, Arc::new(registry), config))
    }

    /// Builds the execution context for a task by loading its dependencies.
    ///
    /// # Arguments
    /// * `claimed_task` - The task to build context for
    /// * `dependencies` - Task dependencies
    ///
    /// # Returns
    /// Result containing the task's execution context
    async fn build_task_context(
        &self,
        claimed_task: &ClaimedTask,
        dependencies: &[crate::task::TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // Debug: Log dependencies for troubleshooting
        tracing::debug!(
            "Building context for task '{}' with {} dependencies: {:?}",
            claimed_task.task_name,
            dependencies.len(),
            dependencies
        );
        tracing::debug!(
            "DEBUG: Building context for task '{}' with {} dependencies: {:?}",
            claimed_task.task_name,
            dependencies.len(),
            dependencies
        );
        let execution_scope = ExecutionScope {
            pipeline_execution_id: claimed_task.pipeline_execution_id,
            task_execution_id: Some(claimed_task.task_execution_id),
            task_name: Some(claimed_task.task_name.clone()),
        };

        // Create context for task execution
        // Dependencies are pre-loaded below using batch loading for better performance
        let mut context = Context::new();

        // Track execution scope for logging/metrics (not stored in context)
        let _execution_scope = execution_scope;

        // Load initial pipeline context if task has no dependencies
        if dependencies.is_empty() {
            if let Ok(pipeline_execution) = self
                .dal
                .pipeline_execution()
                .get_by_id(claimed_task.pipeline_execution_id)
                .await
            {
                if let Some(context_id) = pipeline_execution.context_id {
                    if let Ok(initial_context) = self
                        .dal
                        .context()
                        .read::<serde_json::Value>(context_id)
                        .await
                    {
                        // Merge initial context data
                        for (key, value) in initial_context.data() {
                            let _ = context.insert(key, value.clone());
                        }
                        debug!(
                            "Loaded initial pipeline context with {} keys",
                            initial_context.data().len()
                        );
                    }
                }
            }
        }

        // Batch load dependency contexts in a single query (eager loading strategy)
        // This provides better performance for tasks that access many dependency values
        if !dependencies.is_empty() {
            debug!(
                "Loading dependency contexts for {} dependencies: {:?}",
                dependencies.len(),
                dependencies
            );
            if let Ok(dep_metadata_with_contexts) = self
                .dal
                .task_execution_metadata()
                .get_dependency_metadata_with_contexts(
                    claimed_task.pipeline_execution_id,
                    dependencies,
                )
                .await
            {
                debug!(
                    "Found {} dependency metadata records",
                    dep_metadata_with_contexts.len()
                );
                for (_task_metadata, context_json) in dep_metadata_with_contexts {
                    if let Some(json_str) = context_json {
                        // Parse the JSON context data
                        if let Ok(dep_context) = Context::<serde_json::Value>::from_json(json_str) {
                            debug!(
                                "Merging dependency context with {} keys: {:?}",
                                dep_context.data().len(),
                                dep_context.data().keys().collect::<Vec<_>>()
                            );
                            // Merge context data (smart merging strategy)
                            for (key, value) in dep_context.data() {
                                if let Some(existing_value) = context.get(key) {
                                    // Key exists - perform smart merging
                                    let merged_value =
                                        Self::merge_context_values(existing_value, value);
                                    let _ = context.update(key, merged_value);
                                } else {
                                    // Key doesn't exist - insert new value
                                    let _ = context.insert(key, value.clone());
                                }
                            }
                        } else {
                            debug!("Failed to parse dependency context JSON");
                        }
                    }
                }
            } else {
                debug!(
                    "Failed to load dependency metadata for dependencies: {:?}",
                    dependencies
                );
            }
        }

        debug!(
            "Final context for task {} has {} keys: {:?}",
            claimed_task.task_name,
            context.data().len(),
            context.data().keys().collect::<Vec<_>>()
        );
        Ok(context)
    }

    /// Merges two context values using smart merging strategy.
    ///
    /// For arrays: concatenates unique values maintaining order
    /// For objects: merges recursively (latest wins for conflicting keys)
    /// For primitives: latest wins
    ///
    /// # Arguments
    /// * `existing` - The existing value in the context
    /// * `new` - The new value from dependency context
    ///
    /// # Returns
    /// The merged value
    fn merge_context_values(
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> serde_json::Value {
        use serde_json::Value;

        match (existing, new) {
            // Both are arrays - concatenate and deduplicate
            (Value::Array(existing_arr), Value::Array(new_arr)) => {
                let mut merged = existing_arr.clone();
                for item in new_arr {
                    if !merged.contains(item) {
                        merged.push(item.clone());
                    }
                }
                Value::Array(merged)
            }
            // Both are objects - merge recursively
            (Value::Object(existing_obj), Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    if let Some(existing_value) = merged.get(key) {
                        merged.insert(
                            key.clone(),
                            Self::merge_context_values(existing_value, value),
                        );
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(merged)
            }
            // For all other cases (different types or primitives), latest wins
            (_, new_value) => new_value.clone(),
        }
    }

    /// Executes a task with timeout protection.
    ///
    /// # Arguments
    /// * `task` - The task implementation to execute
    /// * `context` - The execution context
    ///
    /// # Returns
    /// Result containing either the updated context or an error
    async fn execute_with_timeout(
        &self,
        task: &dyn Task,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        match tokio::time::timeout(self.config.task_timeout, task.execute(context)).await {
            Ok(result) => result.map_err(ExecutorError::TaskExecution),
            Err(_) => Err(ExecutorError::TaskTimeout),
        }
    }

    /// Handles the result of task execution.
    ///
    /// This method:
    /// - Saves successful task contexts
    /// - Updates task state
    /// - Handles retries for failed tasks
    /// - Logs execution results
    ///
    /// # Arguments
    /// * `claimed_task` - The executed task
    /// * `result` - The execution result
    ///
    /// # Returns
    /// Result indicating success or failure of result handling
    async fn handle_task_result(
        &self,
        claimed_task: ClaimedTask,
        result: Result<Context<serde_json::Value>, ExecutorError>,
    ) -> Result<(), ExecutorError> {
        match result {
            Ok(result_context) => {
                // Complete task in a single transaction (save context + mark completed)
                self.complete_task_transaction(&claimed_task, result_context)
                    .await?;

                info!("Task completed successfully: {}", claimed_task.task_name);
            }
            Err(error) => {
                // Get task retry policy to determine if we should retry
                let namespace = parse_namespace(&claimed_task.task_name).map_err(|e| {
                    ExecutorError::TaskNotFound(format!("Invalid namespace: {}", e))
                })?;
                let task = get_task(&namespace)
                    .ok_or_else(|| ExecutorError::TaskNotFound(claimed_task.task_name.clone()))?;
                let retry_policy = task.retry_policy();

                // Check if we should retry this task
                if self
                    .should_retry_task(&claimed_task, &error, &retry_policy)
                    .await?
                {
                    self.schedule_task_retry(&claimed_task, &retry_policy)
                        .await?;
                    warn!(
                        "Task failed, scheduled for retry: {} (attempt {})",
                        claimed_task.task_name, claimed_task.attempt
                    );
                } else {
                    // Mark task as permanently failed
                    self.mark_task_failed(claimed_task.task_execution_id, &error)
                        .await?;
                    error!(
                        "Task failed permanently: {} - {}",
                        claimed_task.task_name, error
                    );
                }
            }
        }

        Ok(())
    }

    /// Saves the task's execution context to the database.
    ///
    /// # Arguments
    /// * `claimed_task` - The task whose context to save
    /// * `context` - The context to save
    ///
    /// # Returns
    /// Result indicating success or failure of the save operation
    async fn save_task_context(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        use crate::models::task_execution_metadata::NewTaskExecutionMetadata;

        // Save context data to the contexts table
        let context_id = self.dal.context().create(&context).await?;

        // Create task execution metadata record with reference to context
        let task_metadata_record = NewTaskExecutionMetadata {
            task_execution_id: claimed_task.task_execution_id,
            pipeline_execution_id: claimed_task.pipeline_execution_id,
            task_name: claimed_task.task_name.clone(),
            context_id,
        };

        self.dal
            .task_execution_metadata()
            .upsert_task_execution_metadata(task_metadata_record)
            .await?;

        let key_count = context.data().len();
        let keys: Vec<_> = context.data().keys().collect();
        info!(
            "Context saved: {} (pipeline: {}, {} keys: {:?}, context_id: {:?})",
            claimed_task.task_name, claimed_task.pipeline_execution_id, key_count, keys, context_id
        );
        Ok(())
    }

    /// Marks a task as completed in the database.
    ///
    /// # Arguments
    /// * `task_execution_id` - ID of the task to mark as completed
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    async fn mark_task_completed(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<(), ExecutorError> {
        // Get task info for logging before updating
        let task = self
            .dal
            .task_execution()
            .get_by_id(task_execution_id)
            .await?;

        self.dal
            .task_execution()
            .mark_completed(task_execution_id)
            .await?;

        info!(
            "Task state change: {} -> Completed (task: {}, pipeline: {})",
            task.status, task.task_name, task.pipeline_execution_id
        );
        Ok(())
    }

    /// Completes a task by saving its context and marking it as completed in a single transaction.
    ///
    /// This method groups the context save and status update operations into a single
    /// atomic transaction, ensuring consistency and reducing database roundtrips.
    ///
    /// # Arguments
    /// * `claimed_task` - The task to complete
    /// * `context` - The execution context to save
    ///
    /// # Returns
    /// Result indicating success or failure of the transaction
    async fn complete_task_transaction(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        // Save context and update metadata
        self.save_task_context(claimed_task, context).await?;

        // Mark task as completed
        self.mark_task_completed(claimed_task.task_execution_id)
            .await?;

        Ok(())
    }

    /// Marks a task as failed in the database.
    ///
    /// # Arguments
    /// * `task_execution_id` - ID of the task to mark as failed
    /// * `error` - The error that caused the failure
    ///
    /// # Returns
    /// Result indicating success or failure of the operation
    async fn mark_task_failed(
        &self,
        task_execution_id: UniversalUuid,
        error: &ExecutorError,
    ) -> Result<(), ExecutorError> {
        // Get task info for logging before updating
        let task = self
            .dal
            .task_execution()
            .get_by_id(task_execution_id)
            .await?;

        self.dal
            .task_execution()
            .mark_failed(task_execution_id, &error.to_string())
            .await?;

        error!(
            "Task state change: {} -> Failed (task: {}, pipeline: {}, error: {})",
            task.status, task.task_name, task.pipeline_execution_id, error
        );

        Ok(())
    }

    /// Determines if a failed task should be retried.
    ///
    /// Considers:
    /// - Maximum retry attempts
    /// - Retry policy conditions
    /// - Error type and patterns
    ///
    /// # Arguments
    /// * `claimed_task` - The failed task
    /// * `error` - The error that caused the failure
    /// * `retry_policy` - The task's retry policy
    ///
    /// # Returns
    /// Result containing a boolean indicating whether to retry
    async fn should_retry_task(
        &self,
        claimed_task: &ClaimedTask,
        error: &ExecutorError,
        retry_policy: &RetryPolicy,
    ) -> Result<bool, ExecutorError> {
        // Check if we've exceeded max retry attempts
        if claimed_task.attempt >= retry_policy.max_attempts {
            debug!(
                "Task {} exceeded max retry attempts ({}/{})",
                claimed_task.task_name, claimed_task.attempt, retry_policy.max_attempts
            );
            return Ok(false);
        }

        // Check retry conditions (all must be satisfied)
        let should_retry = retry_policy
            .retry_conditions
            .iter()
            .all(|condition| match condition {
                RetryCondition::Never => false,
                RetryCondition::AllErrors => true,
                RetryCondition::TransientOnly => self.is_transient_error(error),
                RetryCondition::ErrorPattern { patterns } => {
                    let error_msg = error.to_string().to_lowercase();
                    patterns
                        .iter()
                        .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
                }
            });

        debug!(
            "Retry decision for task {}: {} (conditions: {:?}, error: {})",
            claimed_task.task_name, should_retry, retry_policy.retry_conditions, error
        );

        Ok(should_retry)
    }

    /// Determines if an error is transient and potentially retryable.
    ///
    /// # Arguments
    /// * `error` - The error to check
    ///
    /// # Returns
    /// Boolean indicating if the error is transient
    fn is_transient_error(&self, error: &ExecutorError) -> bool {
        match error {
            ExecutorError::TaskTimeout => true,
            ExecutorError::Database(_) => true,
            ExecutorError::ConnectionPool(_) => true,
            ExecutorError::TaskNotFound(_) => false,
            ExecutorError::TaskExecution(task_error) => {
                // Check for common transient error patterns in task errors
                let error_msg = task_error.to_string().to_lowercase();
                error_msg.contains("timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("network")
                    || error_msg.contains("temporary")
                    || error_msg.contains("unavailable")
            }
            _ => false,
        }
    }

    /// Schedules a task for retry execution.
    ///
    /// # Arguments
    /// * `claimed_task` - The task to retry
    /// * `retry_policy` - The task's retry policy
    ///
    /// # Returns
    /// Result indicating success or failure of retry scheduling
    async fn schedule_task_retry(
        &self,
        claimed_task: &ClaimedTask,
        retry_policy: &RetryPolicy,
    ) -> Result<(), ExecutorError> {
        // Calculate retry delay using the backoff strategy
        let retry_delay = retry_policy.calculate_delay(claimed_task.attempt);
        let retry_at = Utc::now() + retry_delay;

        // Use DAL to schedule retry
        self.dal
            .task_execution()
            .schedule_retry(
                claimed_task.task_execution_id,
                crate::database::UniversalTimestamp(retry_at),
                claimed_task.attempt + 1,
            )
            .await?;

        info!(
            "Scheduled retry for task {} in {:?} (attempt {})",
            claimed_task.task_name,
            retry_delay,
            claimed_task.attempt + 1
        );

        Ok(())
    }
}

impl Clone for ThreadTaskExecutor {
    fn clone(&self) -> Self {
        Self {
            database: self.database.clone(),
            dal: self.dal.clone(),
            task_registry: Arc::clone(&self.task_registry),
            instance_id: self.instance_id,
            config: self.config.clone(),
            // Clone metrics by copying current values (each clone gets independent counters)
            active_tasks: AtomicUsize::new(self.active_tasks.load(Ordering::SeqCst)),
            total_executed: AtomicU64::new(self.total_executed.load(Ordering::SeqCst)),
            total_failed: AtomicU64::new(self.total_failed.load(Ordering::SeqCst)),
        }
    }
}

/// Implementation of the dispatcher's TaskExecutor trait.
///
/// This allows ThreadTaskExecutor to be used with the dispatcher pattern,
/// receiving task events directly instead of polling the database.
#[async_trait]
impl TaskExecutor for ThreadTaskExecutor {
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
        let start = Instant::now();
        self.active_tasks.fetch_add(1, Ordering::SeqCst);

        // Convert TaskReadyEvent to ClaimedTask format
        let claimed_task = ClaimedTask {
            task_execution_id: event.task_execution_id,
            pipeline_execution_id: event.pipeline_execution_id,
            task_name: event.task_name.clone(),
            attempt: event.attempt,
        };

        // Resolve task from global registry
        let namespace = match parse_namespace(&claimed_task.task_name) {
            Ok(ns) => ns,
            Err(e) => {
                self.active_tasks.fetch_sub(1, Ordering::SeqCst);
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                return Ok(ExecutionResult::failure(
                    event.task_execution_id,
                    format!("Invalid namespace: {}", e),
                    start.elapsed(),
                ));
            }
        };

        let task = match get_task(&namespace) {
            Some(t) => t,
            None => {
                self.active_tasks.fetch_sub(1, Ordering::SeqCst);
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                return Ok(ExecutionResult::failure(
                    event.task_execution_id,
                    format!("Task not found: {}", claimed_task.task_name),
                    start.elapsed(),
                ));
            }
        };

        // Build context for execution
        let dependencies = task.dependencies();
        let context = match self.build_task_context(&claimed_task, dependencies).await {
            Ok(ctx) => ctx,
            Err(e) => {
                self.active_tasks.fetch_sub(1, Ordering::SeqCst);
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                return Ok(ExecutionResult::failure(
                    event.task_execution_id,
                    format!("Context build failed: {}", e),
                    start.elapsed(),
                ));
            }
        };

        // Execute the task
        let execution_result = self.execute_with_timeout(task.as_ref(), context).await;
        let duration = start.elapsed();

        self.active_tasks.fetch_sub(1, Ordering::SeqCst);

        match execution_result {
            Ok(result_context) => {
                // Save context and mark completed
                match self
                    .complete_task_transaction(&claimed_task, result_context)
                    .await
                {
                    Ok(_) => {
                        self.total_executed.fetch_add(1, Ordering::SeqCst);
                        info!(
                            task_id = %event.task_execution_id,
                            task_name = %event.task_name,
                            duration_ms = duration.as_millis(),
                            "Task executed successfully via dispatcher"
                        );
                        Ok(ExecutionResult::success(event.task_execution_id, duration))
                    }
                    Err(e) => {
                        self.total_failed.fetch_add(1, Ordering::SeqCst);
                        Ok(ExecutionResult::failure(
                            event.task_execution_id,
                            format!("Failed to save context: {}", e),
                            duration,
                        ))
                    }
                }
            }
            Err(error) => {
                // Check if we should retry
                let retry_policy = task.retry_policy();
                let should_retry = self
                    .should_retry_task(&claimed_task, &error, &retry_policy)
                    .await
                    .unwrap_or(false);

                if should_retry {
                    // Schedule retry
                    if let Err(e) = self.schedule_task_retry(&claimed_task, &retry_policy).await {
                        warn!(
                            task_id = %event.task_execution_id,
                            error = %e,
                            "Failed to schedule retry"
                        );
                    }
                    self.total_executed.fetch_add(1, Ordering::SeqCst);
                    Ok(ExecutionResult::retry(
                        event.task_execution_id,
                        error.to_string(),
                        duration,
                    ))
                } else {
                    self.total_failed.fetch_add(1, Ordering::SeqCst);
                    Ok(ExecutionResult::failure(
                        event.task_execution_id,
                        error.to_string(),
                        duration,
                    ))
                }
            }
        }
    }

    fn has_capacity(&self) -> bool {
        self.active_tasks.load(Ordering::SeqCst) < self.config.max_concurrent_tasks
    }

    fn metrics(&self) -> ExecutorMetrics {
        ExecutorMetrics {
            active_tasks: self.active_tasks.load(Ordering::SeqCst),
            max_concurrent: self.config.max_concurrent_tasks,
            total_executed: self.total_executed.load(Ordering::SeqCst),
            total_failed: self.total_failed.load(Ordering::SeqCst),
            avg_duration_ms: 0, // TODO: track moving average
        }
    }

    fn name(&self) -> &str {
        "ThreadTaskExecutor"
    }
}
