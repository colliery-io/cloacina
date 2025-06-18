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
//! The TaskExecutor is responsible for:
//! - Polling for and claiming ready tasks
//! - Executing tasks with proper timeout handling
//! - Managing task retries and error handling
//! - Maintaining task execution state
//! - Handling task dependencies and context management
//!
//! The executor uses a semaphore to limit concurrent task execution and implements
//! a robust retry mechanism with configurable policies.

use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time;
use tracing::{debug, error, info, warn};

use super::traits::TaskExecutorTrait;
use super::types::{ClaimedTask, DependencyLoader, ExecutionScope, ExecutorConfig};
use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;
use crate::error::ExecutorError;
use crate::retry::{RetryCondition, RetryPolicy};
use crate::task::get_task;
use crate::{parse_namespace, Context, Database, Task, TaskRegistry};
use async_trait::async_trait;

/// ThreadTaskExecutor is a thread-based implementation of task execution.
///
/// This executor runs tasks in the current thread/process and manages:
/// - Task claiming and execution
/// - Context management and dependency resolution
/// - Error handling and retry logic
/// - State persistence
///
/// The executor maintains its own instance ID for tracking and logging purposes
/// and uses a task registry to resolve task implementations.
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
        let global_tasks = match global_registry.read() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Task registry RwLock was poisoned, recovering data");
                poisoned.into_inner()
            }
        };

        for (namespace, constructor) in global_tasks.iter() {
            let task = constructor();
            registry.register_arc(namespace.clone(), task)?;
        }

        Ok(Self::new(database, Arc::new(registry), config))
    }

    /// Starts the task executor's main execution loop.
    ///
    /// This method begins polling for and executing tasks according to the configured
    /// parameters. The executor will continue running until explicitly stopped.
    ///
    /// # Returns
    /// Result indicating success or failure of the execution loop
    pub async fn run(&self) -> Result<(), ExecutorError> {
        info!("Starting task executor (instance: {})", self.instance_id);
        self.run_execution_loop().await
    }

    /// Main execution loop that polls for and executes tasks.
    ///
    /// This method implements the core execution logic:
    /// 1. Polls for ready tasks at configured intervals
    /// 2. Claims available tasks using a semaphore for concurrency control
    /// 3. Executes tasks in background tasks
    /// 4. Handles task results and retries
    ///
    /// # Returns
    /// Result indicating success or failure of the execution loop
    async fn run_execution_loop(&self) -> Result<(), ExecutorError> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_tasks));
        let mut interval = time::interval(self.config.poll_interval);

        loop {
            interval.tick().await;

            // Only poll if we have available concurrency slots
            if semaphore.available_permits() == 0 {
                debug!("All execution slots busy, skipping poll");
                continue;
            }

            // Try to claim a ready task with pre-loaded context
            match self.claim_task_with_context().await {
                Ok(Some((claimed_task, preloaded_context))) => {
                    let permit = semaphore.clone().acquire_owned().await?;
                    let executor = self.clone();

                    // Execute task in background with pre-loaded context
                    tokio::spawn(async move {
                        let _permit = permit; // Hold permit until task completes

                        info!(
                            "Executing task with pre-loaded context: {} (attempt {})",
                            claimed_task.task_name, claimed_task.attempt
                        );

                        if let Err(e) = executor
                            .execute_claimed_task_with_context(claimed_task, preloaded_context)
                            .await
                        {
                            error!("Task execution failed: {}", e);
                        }
                    });
                }
                Ok(None) => {
                    // No ready tasks available
                    debug!("No ready tasks found");
                }
                Err(e) => {
                    error!("Failed to claim task: {}", e);
                }
            }
        }
    }

    /// Claims a ready task and pre-loads its execution context in a single transaction.
    ///
    /// This method optimizes task claiming by combining the claim operation with
    /// context loading, reducing database roundtrips and latency between claim and execution.
    ///
    /// # Returns
    /// Result containing either a (ClaimedTask, Context) tuple or None if no tasks are ready
    async fn claim_task_with_context(
        &self,
    ) -> Result<Option<(ClaimedTask, Context<serde_json::Value>)>, ExecutorError> {
        // Use DAL's atomic claim method
        if let Some(claim_result) = self.dal.task_execution().claim_ready_task().await? {
            let claimed_task = ClaimedTask {
                task_execution_id: claim_result.id,
                pipeline_execution_id: claim_result.pipeline_execution_id,
                task_name: claim_result.task_name.clone(),
                attempt: claim_result.attempt,
            };

            // Get task from global registry to determine dependencies
            let namespace = parse_namespace(&claimed_task.task_name)
                .map_err(|e| ExecutorError::TaskNotFound(format!("Invalid namespace: {}", e)))?;
            let task = get_task(&namespace)
                .ok_or_else(|| ExecutorError::TaskNotFound(claimed_task.task_name.clone()))?;
            let dependencies = task.dependencies();

            // Build context using DAL methods
            let context = self.build_task_context(&claimed_task, dependencies).await?;

            info!(
                "Task state change: Ready -> Running (task: {}, pipeline: {}, attempt: {})",
                claimed_task.task_name, claimed_task.pipeline_execution_id, claimed_task.attempt
            );

            Ok(Some((claimed_task, context)))
        } else {
            Ok(None)
        }
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

        // Create dependency loader for automatic context merging
        let dependency_loader = DependencyLoader::new(
            self.database.clone(),
            claimed_task.pipeline_execution_id,
            dependencies.to_vec(),
        );

        // Create context with execution scope and dependency loader
        let mut context = Context::new();
        context.set_execution_scope(execution_scope);
        context.set_dependency_loader(dependency_loader);

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
                                        self.merge_context_values(existing_value, value);
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
        &self,
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
                            self.merge_context_values(existing_value, value),
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

    /// Executes a claimed task with pre-loaded context.
    ///
    /// # Arguments
    /// * `claimed_task` - The task to execute
    /// * `context` - Pre-loaded execution context
    ///
    /// # Returns
    /// Result indicating success or failure of task execution
    async fn execute_claimed_task_with_context(
        &self,
        claimed_task: ClaimedTask,
        context: Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        // 1. Resolve task from global registry
        let namespace = parse_namespace(&claimed_task.task_name)
            .map_err(|e| ExecutorError::TaskNotFound(format!("Invalid namespace: {}", e)))?;
        let task = get_task(&namespace)
            .ok_or_else(|| ExecutorError::TaskNotFound(claimed_task.task_name.clone()))?;

        // 2. Execute task with pre-loaded context (skip context building)
        let execution_result = self.execute_with_timeout(task.as_ref(), context).await;

        // 3. Handle result and update database
        self.handle_task_result(claimed_task, execution_result)
            .await?;

        Ok(())
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
        }
    }
}

#[async_trait]
impl TaskExecutorTrait for ThreadTaskExecutor {
    async fn run(&self) -> Result<(), ExecutorError> {
        info!(
            "Starting thread task executor (instance: {})",
            self.instance_id
        );
        self.run_execution_loop().await
    }
}
