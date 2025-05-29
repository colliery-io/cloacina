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

//! Task Execution Data Access Layer
//!
//! This module provides the data access layer for managing task executions in the pipeline system.
//! It handles all database operations related to task execution states, retries, and recovery.
//!
//! Key features:
//! - Task state management (Ready, Running, Completed, Failed, Skipped)
//! - Retry mechanism with configurable backoff
//! - Recovery system for handling orphaned tasks
//! - Atomic task claiming for distributed execution
//! - Pipeline completion and failure detection
//!
//! The module uses Diesel ORM for database operations and implements optimistic locking
//! patterns for concurrent task execution.

use super::DAL;
use crate::database::schema::task_executions;
use crate::database::universal_types::{current_timestamp, UniversalTimestamp, UniversalUuid};
use crate::error::ValidationError;
use crate::models::task_execution::{NewTaskExecution, TaskExecution};
use diesel::prelude::*;

/// Statistics about retry behavior for a pipeline execution.
///
/// This structure tracks various metrics related to task retries within a pipeline,
/// helping to monitor and analyze the reliability of task execution.
#[derive(Debug, Default)]
pub struct RetryStats {
    /// Number of tasks that required at least one retry.
    pub tasks_with_retries: i32,
    /// Total number of retry attempts across all tasks.
    pub total_retries: i32,
    /// Maximum number of attempts used by any single task.
    pub max_attempts_used: i32,
    /// Number of tasks that exhausted all retry attempts and failed.
    pub tasks_exhausted_retries: i32,
}

/// Data Access Layer for task execution operations.
///
/// This struct provides methods for managing task execution states, handling retries,
/// and implementing recovery mechanisms. It maintains a reference to the main DAL
/// for database connection management.
pub struct TaskExecutionDAL<'a> {
    pub dal: &'a DAL,
}

impl<'a> TaskExecutionDAL<'a> {
    /// Creates a new task execution record in the database.
    ///
    /// # Arguments
    /// * `new_task` - A `NewTaskExecution` struct containing the task details
    ///
    /// # Returns
    /// * `Result<TaskExecution, ValidationError>` - The created task execution record
    pub fn create(&self, new_task: NewTaskExecution) -> Result<TaskExecution, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        // For SQLite, we need to manually generate the UUID and timestamps
        let id = UniversalUuid::new_v4();
        let now = current_timestamp();

        // Insert with explicit values for SQLite
        diesel::insert_into(task_executions::table)
            .values((
                task_executions::id.eq(&id),
                task_executions::pipeline_execution_id.eq(&new_task.pipeline_execution_id),
                task_executions::task_name.eq(&new_task.task_name),
                task_executions::status.eq(&new_task.status),
                task_executions::attempt.eq(&new_task.attempt),
                task_executions::max_attempts.eq(&new_task.max_attempts),
                task_executions::trigger_rules.eq(&new_task.trigger_rules),
                task_executions::task_configuration.eq(&new_task.task_configuration),
                task_executions::recovery_attempts.eq(0),
                task_executions::created_at.eq(&now),
                task_executions::updated_at.eq(&now),
            ))
            .execute(&mut conn)?;

        // Retrieve the inserted record
        let task = task_executions::table
            .find(id)
            .first(&mut conn)?;

        Ok(task)
    }

    /// Retrieves all pending (NotStarted) tasks for a specific pipeline execution.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecution>, ValidationError>` - List of pending tasks
    pub fn get_pending_tasks(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let tasks = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .filter(task_executions::status.eq("NotStarted"))
            .load(&mut conn)?;

        Ok(tasks)
    }

    /// Marks a task as ready for execution.
    ///
    /// This method updates the task status to "Ready" and logs the state change.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to mark as ready
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn mark_ready(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        // Get task info for logging before updating
        let task = task_executions::table
            .find(task_id)
            .first::<TaskExecution>(&mut conn)?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Ready"),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        tracing::debug!(
            "Task state change: {} -> Ready (task: {}, pipeline: {})",
            task.status,
            task.task_name,
            task.pipeline_execution_id
        );
        Ok(())
    }

    /// Marks a task as skipped with a provided reason.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to skip
    /// * `reason` - String explaining why the task was skipped
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn mark_skipped(&self, task_id: UniversalUuid, reason: &str) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        // Get task info for logging before updating
        let task = task_executions::table
            .find(task_id)
            .first::<TaskExecution>(&mut conn)?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Skipped"),
                task_executions::error_details.eq(reason),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        tracing::info!(
            "Task state change: {} -> Skipped (task: {}, pipeline: {}, reason: {})",
            task.status,
            task.task_name,
            task.pipeline_execution_id,
            reason
        );
        Ok(())
    }

    /// Retrieves all tasks associated with a pipeline execution.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecution>, ValidationError>` - List of all tasks
    pub fn get_all_tasks_for_pipeline(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let tasks = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .load(&mut conn)?;

        Ok(tasks)
    }

    /// Gets the current status of a specific task in a pipeline.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    /// * `task_name` - Name of the task to check
    ///
    /// # Returns
    /// * `Result<String, ValidationError>` - Current status of the task
    pub fn get_task_status(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_name: &str,
    ) -> Result<String, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let status: String = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .filter(task_executions::task_name.eq(task_name))
            .select(task_executions::status)
            .first(&mut conn)?;

        Ok(status)
    }

    /// Gets the status of multiple tasks in a single database query.
    ///
    /// This method provides efficient batch lookup of task statuses, reducing
    /// the number of database roundtrips when checking multiple task dependencies.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    /// * `task_names` - Vector of task names to check
    ///
    /// # Returns
    /// * `Result<std::collections::HashMap<String, String>, ValidationError>` - Map of task names to their statuses
    ///
    /// # Example
    /// ```rust
    /// let task_names = vec!["task1".to_string(), "task2".to_string()];
    /// let statuses = dal.task_execution().get_task_statuses_batch(pipeline_id, task_names)?;
    /// let task1_status = statuses.get("task1");
    /// ```
    pub fn get_task_statuses_batch(
        &self,
        pipeline_execution_id: UniversalUuid,
        task_names: Vec<String>,
    ) -> Result<std::collections::HashMap<String, String>, ValidationError> {
        use std::collections::HashMap;

        if task_names.is_empty() {
            return Ok(HashMap::new());
        }

        let mut conn = self.dal.pool.get()?;

        let results: Vec<(String, String)> = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .filter(task_executions::task_name.eq_any(&task_names))
            .select((task_executions::task_name, task_executions::status))
            .load(&mut conn)?;

        let status_map: HashMap<String, String> = results.into_iter().collect();
        Ok(status_map)
    }

    /// Gets all pending tasks for multiple pipelines in a single query.
    ///
    /// This method provides efficient batch lookup of pending tasks across multiple
    /// pipeline executions, reducing database roundtrips when processing multiple pipelines.
    ///
    /// # Arguments
    /// * `pipeline_execution_ids` - Vector of pipeline execution IDs to check
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecution>, ValidationError>` - All pending tasks across all pipelines
    ///
    /// # Performance
    /// This replaces N individual queries (one per pipeline) with a single batch query.
    pub fn get_pending_tasks_batch(
        &self,
        pipeline_execution_ids: Vec<UniversalUuid>,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        if pipeline_execution_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.dal.pool.get()?;

        let tasks: Vec<TaskExecution> = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq_any(&pipeline_execution_ids))
            .filter(task_executions::status.eq_any(vec!["NotStarted", "Pending"]))
            .load(&mut conn)?;

        Ok(tasks)
    }

    /// Checks if all tasks in a pipeline have reached a terminal state.
    ///
    /// A pipeline is considered complete when all tasks are in one of:
    /// - Completed
    /// - Failed
    /// - Skipped
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<bool, ValidationError>` - True if pipeline is complete
    pub fn check_pipeline_completion(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let incomplete_count: i64 = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .filter(task_executions::status.ne_all(vec!["Completed", "Failed", "Skipped"]))
            .count()
            .get_result(&mut conn)?;

        Ok(incomplete_count == 0)
    }

    /// Retrieves tasks that are stuck in "Running" state (orphaned tasks).
    ///
    /// These are tasks that were executing when the system crashed or was interrupted.
    /// They need to be recovered for proper pipeline execution.
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecution>, ValidationError>` - List of orphaned tasks
    pub fn get_orphaned_tasks(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let orphaned_tasks = task_executions::table
            .filter(task_executions::status.eq("Running"))
            .load(&mut conn)?;

        Ok(orphaned_tasks)
    }

    /// Resets a task from "Running" to "Ready" state for recovery.
    ///
    /// This method is used in recovery scenarios to allow orphaned tasks to be
    /// picked up again by an available executor.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to reset
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn reset_task_for_recovery(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Ready"),
                task_executions::started_at.eq(None::<UniversalTimestamp>),
                task_executions::recovery_attempts.eq(task_executions::recovery_attempts + 1),
                task_executions::last_recovery_at.eq(Some(current_timestamp())),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Marks a task as permanently abandoned after too many recovery attempts.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to mark as abandoned
    /// * `reason` - String explaining why the task was abandoned
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn mark_abandoned(&self, task_id: UniversalUuid, reason: &str) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Failed"),
                task_executions::completed_at.eq(Some(current_timestamp())),
                task_executions::error_details.eq(format!("ABANDONED: {}", reason)),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Checks if a pipeline should be marked as failed due to abandoned tasks.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<bool, ValidationError>` - True if pipeline should be marked as failed
    pub fn check_pipeline_failure(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<bool, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        // Check if any tasks are permanently failed (abandoned)
        let failed_count: i64 = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .filter(task_executions::status.eq("Failed"))
            .filter(task_executions::error_details.like("ABANDONED:%"))
            .count()
            .get_result(&mut conn)?;

        Ok(failed_count > 0)
    }

    /// Retrieves a specific task execution by its ID.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to retrieve
    ///
    /// # Returns
    /// * `Result<TaskExecution, ValidationError>` - The task execution record
    pub fn get_by_id(&self, task_id: UniversalUuid) -> Result<TaskExecution, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let task = task_executions::table.find(task_id).first(&mut conn)?;

        Ok(task)
    }

    /// Retrieves tasks that are ready for retry (retry_at time has passed).
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecution>, ValidationError>` - List of tasks ready for retry
    pub fn get_ready_for_retry(&self) -> Result<Vec<TaskExecution>, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let ready_tasks = task_executions::table
            .filter(task_executions::status.eq("Ready"))
            .filter(
                task_executions::retry_at
                    .is_null()
                    .or(task_executions::retry_at.le(current_timestamp())),
            )
            .load(&mut conn)?;

        Ok(ready_tasks)
    }

    /// Updates a task's retry schedule with a new attempt count and retry time.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to schedule for retry
    /// * `retry_at` - DateTime when the task should be retried
    /// * `new_attempt` - The new attempt number
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn schedule_retry(
        &self,
        task_id: UniversalUuid,
        retry_at: UniversalTimestamp,
        new_attempt: i32,
    ) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Ready"),
                task_executions::attempt.eq(new_attempt),
                task_executions::retry_at.eq(Some(retry_at)),
                task_executions::started_at.eq(None::<UniversalTimestamp>),
                task_executions::completed_at.eq(None::<UniversalTimestamp>),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Calculates retry statistics for a specific pipeline execution.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<RetryStats, ValidationError>` - Statistics about retry behavior
    pub fn get_retry_stats(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<RetryStats, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        let tasks = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .load::<TaskExecution>(&mut conn)?;

        let mut stats = RetryStats::default();

        for task in tasks {
            if task.attempt > 1 {
                stats.tasks_with_retries += 1;
                stats.total_retries += task.attempt - 1;
            }

            if task.attempt > stats.max_attempts_used {
                stats.max_attempts_used = task.attempt;
            }

            if task.status == "Failed" && task.attempt >= task.max_attempts {
                stats.tasks_exhausted_retries += 1;
            }
        }

        Ok(stats)
    }

    /// Retrieves tasks that have exceeded their retry limit.
    ///
    /// # Arguments
    /// * `pipeline_execution_id` - UUID of the pipeline execution
    ///
    /// # Returns
    /// * `Result<Vec<TaskExecution>, ValidationError>` - List of exhausted tasks
    pub fn get_exhausted_retry_tasks(
        &self,
        pipeline_execution_id: UniversalUuid,
    ) -> Result<Vec<TaskExecution>, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        // Use a more explicit query to avoid type inference issues
        let exhausted_tasks = task_executions::table
            .filter(task_executions::pipeline_execution_id.eq(pipeline_execution_id))
            .filter(task_executions::status.eq("Failed"))
            .load::<TaskExecution>(&mut conn)?
            .into_iter()
            .filter(|task| task.attempt >= task.max_attempts)
            .collect();

        Ok(exhausted_tasks)
    }

    /// Resets the retry state for a task to its initial state.
    ///
    /// This is typically used in recovery scenarios to give a task a fresh start.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to reset
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn reset_retry_state(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::attempt.eq(1),
                task_executions::retry_at.eq(None::<UniversalTimestamp>),
                task_executions::started_at.eq(None::<UniversalTimestamp>),
                task_executions::completed_at.eq(None::<UniversalTimestamp>),
                task_executions::last_error.eq(None::<String>),
                task_executions::status.eq("Ready"),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Marks a task execution as completed.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to mark as completed
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn mark_completed(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Completed"),
                task_executions::completed_at.eq(Some(current_timestamp())),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Marks a task execution as failed with an error message.
    ///
    /// # Arguments
    /// * `task_id` - UUID of the task to mark as failed
    /// * `error_message` - String describing the failure reason
    ///
    /// # Returns
    /// * `Result<(), ValidationError>` - Success or error
    pub fn mark_failed(&self, task_id: UniversalUuid, error_message: &str) -> Result<(), ValidationError> {
        let mut conn = self.dal.pool.get()?;

        diesel::update(task_executions::table.find(task_id))
            .set((
                task_executions::status.eq("Failed"),
                task_executions::completed_at.eq(Some(current_timestamp())),
                task_executions::last_error.eq(error_message),
                task_executions::updated_at.eq(current_timestamp()),
            ))
            .execute(&mut conn)?;

        Ok(())
    }

    /// Atomically claims a ready task for execution.
    ///
    /// For SQLite, this uses a transaction-based approach to safely claim tasks.
    ///
    /// # Returns
    /// * `Result<Option<ClaimResult>, ValidationError>` - The claimed task or None if no tasks available
    pub fn claim_ready_task(&self) -> Result<Option<ClaimResult>, ValidationError> {
        let mut conn = self.dal.pool.get()?;

        // SQLite doesn't support FOR UPDATE SKIP LOCKED, so we use a transaction
        conn.transaction(|conn| {
            // Find a ready task
            let task: Option<TaskExecution> = task_executions::table
                .filter(task_executions::status.eq("Ready"))
                .filter(
                    task_executions::retry_at
                        .is_null()
                        .or(task_executions::retry_at.le(current_timestamp())),
                )
                .order(task_executions::id.asc())
                .first(conn)
                .optional()?;

            if let Some(task) = task {
                // Update the task to Running
                diesel::update(task_executions::table.find(&task.id))
                    .set((
                        task_executions::status.eq("Running"),
                        task_executions::started_at.eq(Some(current_timestamp())),
                    ))
                    .execute(conn)?;

                Ok(Some(ClaimResult {
                    id: task.id,
                    pipeline_execution_id: task.pipeline_execution_id,
                    task_name: task.task_name,
                    attempt: task.attempt,
                }))
            } else {
                Ok(None)
            }
        })
    }
}

/// Result structure for atomic task claiming operations.
///
/// This structure contains the essential information about a task that has been
/// atomically claimed for execution.
#[derive(Debug)]
pub struct ClaimResult {
    /// Unique identifier of the claimed task
    pub id: UniversalUuid,
    /// ID of the pipeline execution this task belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// Name of the task that was claimed
    pub task_name: String,
    /// Current attempt number for this task
    pub attempt: i32,
}
