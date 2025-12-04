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

//! # Task Trait and State
//!
//! This module provides the core `Task` trait and `TaskState` enum for defining
//! executable tasks in Cloacina workflows.

use crate::context::Context;
use crate::error::{CheckpointError, TaskError};
use crate::namespace::TaskNamespace;
use crate::retry::RetryPolicy;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents the execution state of a task throughout its lifecycle.
///
/// Tasks progress through these states during execution, providing visibility
/// into the current status and enabling proper error handling and recovery.
///
/// # State Transitions
///
/// - `Pending` -> `Running`: When task execution begins
/// - `Running` -> `Completed`: When task completes successfully
/// - `Running` -> `Failed`: When task encounters an error
/// - `Failed` -> `Running`: When task is retried
///
/// Terminal states (`Completed` and `Failed`) do not transition to other states
/// unless a retry is attempted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskState {
    /// Task is registered but not yet started
    Pending,
    /// Task is currently executing
    Running { start_time: DateTime<Utc> },
    /// Task finished successfully
    Completed { completion_time: DateTime<Utc> },
    /// Task encountered an error
    Failed {
        error: String,
        failure_time: DateTime<Utc>,
    },
    /// Task was skipped (e.g., trigger rule not satisfied)
    Skipped {
        reason: String,
        skip_time: DateTime<Utc>,
    },
}

impl TaskState {
    /// Returns true if the task is in the completed state
    pub fn is_completed(&self) -> bool {
        matches!(self, TaskState::Completed { .. })
    }

    /// Returns true if the task is in the failed state
    pub fn is_failed(&self) -> bool {
        matches!(self, TaskState::Failed { .. })
    }

    /// Returns true if the task is currently running
    pub fn is_running(&self) -> bool {
        matches!(self, TaskState::Running { .. })
    }

    /// Returns true if the task is pending execution
    pub fn is_pending(&self) -> bool {
        matches!(self, TaskState::Pending)
    }

    /// Returns true if the task was skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, TaskState::Skipped { .. })
    }
}

/// Core trait that defines an executable task in a pipeline.
///
/// Tasks are the fundamental units of work in Cloacina. Most users should use the
/// `#[task]` macro instead of implementing this trait directly, as the macro provides
/// automatic registration, code fingerprinting, and convenient syntax.
///
/// # Task Execution Model
///
/// Tasks follow a simple but powerful execution model:
///
/// 1. **Input**: Receive a context containing data from previous tasks
/// 2. **Processing**: Execute the task's business logic
/// 3. **Output**: Update the context with results
/// 4. **Completion**: Return success or failure
///
/// # Using the Macro (Recommended)
///
/// ```rust,ignore
/// use cloacina_workflow::*;
///
/// #[task(id = "my_task", dependencies = [])]
/// async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
///     // Your task logic here
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait Task: Send + Sync {
    /// Executes the task with the provided context.
    ///
    /// This is the main entry point for task execution. The method receives
    /// a context containing data from previous tasks and should return an
    /// updated context with any new or modified data.
    ///
    /// # Arguments
    ///
    /// * `context` - The execution context containing task data
    ///
    /// # Returns
    ///
    /// * `Ok(Context)` - Updated context with task results
    /// * `Err(TaskError)` - If the task execution fails
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError>;

    /// Returns the unique identifier for this task.
    ///
    /// The task ID must be unique within a Workflow or TaskRegistry.
    /// It's used for dependency resolution and task lookup.
    fn id(&self) -> &str;

    /// Returns the list of task namespaces that this task depends on.
    ///
    /// Dependencies define the execution order - this task will only
    /// execute after all its dependencies have completed successfully.
    fn dependencies(&self) -> &[TaskNamespace];

    /// Saves a checkpoint for this task.
    ///
    /// This method is called to save intermediate state during task execution.
    /// The default implementation is a no-op, but tasks can override this
    /// to implement custom checkpointing logic.
    ///
    /// # Arguments
    ///
    /// * `context` - The current execution context
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If checkpointing succeeds
    /// * `Err(CheckpointError)` - If checkpointing fails
    fn checkpoint(&self, _context: &Context<serde_json::Value>) -> Result<(), CheckpointError> {
        // Default implementation - tasks can override for custom checkpointing
        Ok(())
    }

    /// Returns the retry policy for this task.
    ///
    /// This method defines how the task should behave when it fails, including
    /// the number of retry attempts, backoff strategy, and conditions under
    /// which retries should be attempted.
    ///
    /// The default implementation returns a sensible production-ready policy
    /// with exponential backoff and 3 retry attempts.
    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::default()
    }

    /// Returns the trigger rules for this task.
    ///
    /// Trigger rules define the conditions under which this task should execute
    /// beyond simple dependency satisfaction. The default implementation returns
    /// an "Always" trigger rule, meaning the task executes whenever its dependencies
    /// are satisfied.
    ///
    /// # Returns
    ///
    /// A JSON value representing the trigger rules for this task.
    fn trigger_rules(&self) -> serde_json::Value {
        serde_json::json!({"type": "Always"})
    }

    /// Returns a code fingerprint for content-based versioning.
    ///
    /// This method should return a hash of the task's implementation code,
    /// enabling automatic detection of changes for Workflow versioning.
    ///
    /// The default implementation returns None, indicating that the task
    /// doesn't support code fingerprinting. Tasks generated by the `#[task]`
    /// macro automatically provide fingerprints.
    ///
    /// # Returns
    ///
    /// - `Some(String)` - A hex-encoded hash of the task's code content
    /// - `None` - Task doesn't support code fingerprinting
    fn code_fingerprint(&self) -> Option<String> {
        None
    }
}
