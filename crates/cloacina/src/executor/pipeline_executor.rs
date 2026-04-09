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

//! Workflow execution engine for workflow orchestration.
//!
//! This module provides the core functionality for executing workflows,
//! managing their lifecycle, and handling execution results. It includes support for
//! both synchronous and asynchronous execution, status monitoring, and error handling.
//!
//! # Key Components
//!
//! - `WorkflowExecutor`: Core trait defining the execution engine interface
//! - `WorkflowExecution`: Handle for managing asynchronous workflow executions
//! - `WorkflowStatus`: Represents the current state of a workflow
//! - `WorkflowExecutionResult`: Contains the final outcome of a workflow execution
//! - `TaskResult`: Represents the outcome of individual task execution
//!
//! # Example
//!
//! ```rust,ignore
//! use cloacina::executor::WorkflowExecutor;
//! use cloacina::Context;
//!
//! async fn run_workflow(executor: &impl WorkflowExecutor) {
//!     let context = Context::new(serde_json::json!({}));
//!     match executor.execute("my_workflow", context).await {
//!         Ok(result) => println!("Workflow completed: {:?}", result),
//!         Err(e) => println!("Workflow failed: {}", e),
//!     }
//! }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::time::Duration;
use uuid::Uuid;

use crate::error::{ExecutorError, TaskError, ValidationError};
use crate::task::TaskState;
use crate::Context;

/// Callback trait for receiving real-time status updates during workflow execution.
///
/// Implement this trait to receive notifications about status changes in a running workflow.
/// This is useful for monitoring progress, updating UI, or triggering dependent actions.
pub trait StatusCallback: Send + Sync {
    /// Called whenever the workflow status changes.
    ///
    /// # Arguments
    ///
    /// * `status` - The new status of the workflow
    fn on_status_change(&self, status: WorkflowStatus);
}

/// Represents the outcome of a single task execution within a pipeline.
///
/// This struct contains detailed information about a task's execution, including
/// timing information, status, and any error messages.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Name of the task that was executed
    pub task_name: String,
    /// Final status of the task execution
    pub status: TaskState,
    /// When the task started execution
    pub start_time: Option<DateTime<Utc>>,
    /// When the task completed execution
    pub end_time: Option<DateTime<Utc>>,
    /// Total duration of the task execution
    pub duration: Option<Duration>,
    /// Number of attempts made to execute the task
    pub attempt_count: i32,
    /// Error message if the task failed
    pub error_message: Option<String>,
}

/// Unified error type for workflow execution operations.
///
/// This enum represents all possible error conditions that can occur during
/// workflow execution, including database errors, workflow not found errors,
/// execution failures, timeouts, and various other error types.
#[derive(Debug, thiserror::Error)]
pub enum WorkflowExecutionError {
    #[error("Database connection failed: {message}")]
    DatabaseConnection { message: String },

    #[error("Workflow not found: {workflow_name}")]
    WorkflowNotFound { workflow_name: String },

    #[error("Pipeline execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("Pipeline timeout after {timeout_seconds}s")]
    Timeout { timeout_seconds: u64 },

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Task execution error: {0}")]
    TaskExecution(#[from] TaskError),

    #[error("Executor error: {0}")]
    Executor(#[from] ExecutorError),

    #[error("Configuration error: {message}")]
    Configuration { message: String },
}

/// Represents the current state of a workflow execution.
///
/// The status transitions through these states during the lifecycle of a workflow:
/// Pending -> Running -> (Completed | Failed | Cancelled)
///                    <-> Paused (can resume back to Running)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStatus {
    /// Workflow is queued but not yet started
    Pending,
    /// Workflow is currently executing
    Running,
    /// Workflow completed successfully
    Completed,
    /// Workflow failed during execution
    Failed,
    /// Workflow was cancelled before completion
    Cancelled,
    /// Workflow is paused and can be resumed
    Paused,
}

impl WorkflowStatus {
    /// Determines if this status represents a terminal state.
    ///
    /// Terminal states are those from which the workflow cannot transition to another state.
    ///
    /// # Returns
    ///
    /// `true` if the status is Completed, Failed, or Cancelled
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }
}

/// Contains the complete result of a workflow execution.
///
/// This struct provides comprehensive information about a completed workflow execution,
/// including timing information, final context state, and results of all tasks.
#[derive(Debug)]
pub struct WorkflowExecutionResult {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// Name of the workflow that was executed
    pub workflow_name: String,
    /// Final status of the workflow
    pub status: WorkflowStatus,
    /// When the workflow started execution
    pub start_time: DateTime<Utc>,
    /// When the workflow completed execution
    pub end_time: Option<DateTime<Utc>>,
    /// Total duration of the workflow execution
    pub duration: Option<Duration>,
    /// Final state of the execution context
    pub final_context: Context<serde_json::Value>,
    /// Results of all tasks in the workflow
    pub task_results: Vec<TaskResult>,
    /// Error message if the workflow failed
    pub error_message: Option<String>,
}

/// Handle for managing an asynchronous workflow execution.
///
/// This struct provides methods to monitor and control a running workflow execution.
/// It can be used to check status, wait for completion, or cancel the execution.
pub struct WorkflowExecution {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// Name of the workflow being executed
    pub workflow_name: String,
    executor: crate::runner::DefaultRunner,
}

impl WorkflowExecution {
    /// Creates a new workflow execution handle.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - Unique identifier for the execution
    /// * `workflow_name` - Name of the workflow being executed
    /// * `executor` - The executor instance managing this execution
    pub fn new(
        execution_id: Uuid,
        workflow_name: String,
        executor: crate::runner::DefaultRunner,
    ) -> Self {
        Self {
            execution_id,
            workflow_name,
            executor,
        }
    }

    /// Waits for the workflow to complete execution.
    ///
    /// This method blocks until the workflow reaches a terminal state.
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
    /// * `Err(WorkflowExecutionError)` - If the execution fails or encounters an error
    pub async fn wait_for_completion(
        self,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        self.wait_for_completion_with_timeout(None).await
    }

    /// Waits for completion with a specified timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional duration after which to timeout the wait
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
    /// * `Err(WorkflowExecutionError)` - If the execution fails, times out, or encounters an error
    pub async fn wait_for_completion_with_timeout(
        self,
        timeout: Option<Duration>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        let start_time = std::time::Instant::now();

        loop {
            // Check timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() > timeout_duration {
                    return Err(WorkflowExecutionError::Timeout {
                        timeout_seconds: timeout_duration.as_secs(),
                    });
                }
            }

            // Check status
            match self
                .executor
                .get_execution_status(self.execution_id)
                .await?
            {
                status if status.is_terminal() => {
                    return self.executor.get_execution_result(self.execution_id).await;
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Gets the current status of the workflow execution.
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowStatus)` - The current status of the execution
    /// * `Err(WorkflowExecutionError)` - If the status cannot be retrieved
    pub async fn get_status(&self) -> Result<WorkflowStatus, WorkflowExecutionError> {
        self.executor.get_execution_status(self.execution_id).await
    }

    /// Cancels the workflow execution.
    ///
    /// This method attempts to gracefully stop the workflow execution.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the cancellation was successful
    /// * `Err(WorkflowExecutionError)` - If the cancellation failed
    pub async fn cancel(&self) -> Result<(), WorkflowExecutionError> {
        self.executor.cancel_execution(self.execution_id).await
    }

    /// Pauses the workflow execution.
    ///
    /// When paused, no new tasks will be scheduled, but in-flight tasks will
    /// complete normally. The workflow can be resumed later.
    ///
    /// # Arguments
    ///
    /// * `reason` - Optional reason for pausing the execution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the pause was successful
    /// * `Err(WorkflowExecutionError)` - If the pause failed
    pub async fn pause(&self, reason: Option<&str>) -> Result<(), WorkflowExecutionError> {
        self.executor
            .pause_execution(self.execution_id, reason)
            .await
    }

    /// Resumes a paused workflow execution.
    ///
    /// The scheduler will resume scheduling tasks for this workflow on the next
    /// poll cycle.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the resume was successful
    /// * `Err(WorkflowExecutionError)` - If the resume failed
    pub async fn resume(&self) -> Result<(), WorkflowExecutionError> {
        self.executor.resume_execution(self.execution_id).await
    }
}

/// Core trait defining the interface for workflow execution engines.
///
/// This trait provides the fundamental operations for executing and managing
/// workflows. Implementations should handle the actual execution
/// logic, state management, and error handling.
#[async_trait]
pub trait WorkflowExecutor: Send + Sync {
    /// Executes a workflow and waits for completion.
    ///
    /// # Arguments
    ///
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow execution
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
    /// * `Err(WorkflowExecutionError)` - If the execution fails or encounters an error
    async fn execute(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError>;

    /// Executes a workflow asynchronously.
    ///
    /// This method returns immediately with a handle to the execution,
    /// allowing the caller to monitor progress.
    ///
    /// # Arguments
    ///
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow execution
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowExecution)` - Handle to the running execution
    /// * `Err(WorkflowExecutionError)` - If the execution cannot be started
    async fn execute_async(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecution, WorkflowExecutionError>;

    /// Gets the current status of a running execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to check
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowStatus)` - Current status of the execution
    /// * `Err(WorkflowExecutionError)` - If the status cannot be retrieved
    async fn get_execution_status(
        &self,
        execution_id: Uuid,
    ) -> Result<WorkflowStatus, WorkflowExecutionError>;

    /// Gets the final result of a completed execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to get results for
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowExecutionResult)` - The final result of the execution
    /// * `Err(WorkflowExecutionError)` - If the result cannot be retrieved
    async fn get_execution_result(
        &self,
        execution_id: Uuid,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError>;

    /// Cancels a running execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to cancel
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the cancellation was successful
    /// * `Err(WorkflowExecutionError)` - If the cancellation failed
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError>;

    /// Pauses a running workflow execution.
    ///
    /// When paused, no new tasks will be scheduled, but in-flight tasks will
    /// complete normally. The workflow can be resumed later.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to pause
    /// * `reason` - Optional reason for pausing the execution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the pause was successful
    /// * `Err(WorkflowExecutionError)` - If the pause failed (e.g., workflow not running)
    async fn pause_execution(
        &self,
        execution_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), WorkflowExecutionError>;

    /// Resumes a paused workflow execution.
    ///
    /// The scheduler will resume scheduling tasks for this workflow on the next
    /// poll cycle.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to resume
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the resume was successful
    /// * `Err(WorkflowExecutionError)` - If the resume failed (e.g., workflow not paused)
    async fn resume_execution(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError>;

    /// Executes a workflow with status updates via callback.
    ///
    /// # Arguments
    ///
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow execution
    /// * `callback` - Callback to receive status updates
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
    /// * `Err(WorkflowExecutionError)` - If the execution fails or encounters an error
    async fn execute_with_callback(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
        callback: Box<dyn StatusCallback>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError>;

    /// Lists recent workflow executions.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<WorkflowExecutionResult>)` - List of recent execution results
    /// * `Err(WorkflowExecutionError)` - If the list cannot be retrieved
    async fn list_executions(&self)
        -> Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError>;

    /// Shuts down the executor and its background services.
    ///
    /// This method should be called before the application exits to ensure
    /// proper cleanup of resources.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If shutdown was successful
    /// * `Err(WorkflowExecutionError)` - If shutdown failed
    async fn shutdown(&self) -> Result<(), WorkflowExecutionError>;
}

impl WorkflowStatus {
    /// Creates a WorkflowStatus from a string representation.
    ///
    /// This method is used internally for deserializing workflow statuses from
    /// various sources (database, API responses, etc.). It provides a consistent
    /// way to convert string representations of workflow statuses into the
    /// corresponding enum variants.
    ///
    /// # Arguments
    ///
    /// * `s` - String representation of the status
    ///
    /// # Returns
    ///
    /// The corresponding WorkflowStatus variant, or Failed if the string is invalid
    ///
    /// # Usage
    /// - Database deserialization
    /// - API response parsing
    /// - Status conversion from external systems
    /// - Testing and validation
    #[allow(dead_code)]
    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "Pending" => WorkflowStatus::Pending,
            "Running" => WorkflowStatus::Running,
            "Completed" => WorkflowStatus::Completed,
            "Failed" => WorkflowStatus::Failed,
            "Cancelled" => WorkflowStatus::Cancelled,
            "Paused" => WorkflowStatus::Paused,
            _ => WorkflowStatus::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // -----------------------------------------------------------------------
    // WorkflowStatus tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_pipeline_status_is_terminal() {
        assert!(WorkflowStatus::Completed.is_terminal());
        assert!(WorkflowStatus::Failed.is_terminal());
        assert!(WorkflowStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_pipeline_status_is_not_terminal() {
        assert!(!WorkflowStatus::Pending.is_terminal());
        assert!(!WorkflowStatus::Running.is_terminal());
        assert!(!WorkflowStatus::Paused.is_terminal());
    }

    #[test]
    fn test_pipeline_status_from_str_valid() {
        assert_eq!(WorkflowStatus::from_str("Pending"), WorkflowStatus::Pending);
        assert_eq!(WorkflowStatus::from_str("Running"), WorkflowStatus::Running);
        assert_eq!(
            WorkflowStatus::from_str("Completed"),
            WorkflowStatus::Completed
        );
        assert_eq!(WorkflowStatus::from_str("Failed"), WorkflowStatus::Failed);
        assert_eq!(
            WorkflowStatus::from_str("Cancelled"),
            WorkflowStatus::Cancelled
        );
        assert_eq!(WorkflowStatus::from_str("Paused"), WorkflowStatus::Paused);
    }

    #[test]
    fn test_pipeline_status_from_str_invalid_defaults_to_failed() {
        assert_eq!(WorkflowStatus::from_str("garbage"), WorkflowStatus::Failed);
        assert_eq!(WorkflowStatus::from_str(""), WorkflowStatus::Failed);
        assert_eq!(WorkflowStatus::from_str("running"), WorkflowStatus::Failed);
        // case-sensitive
    }

    #[test]
    fn test_pipeline_status_eq() {
        assert_eq!(WorkflowStatus::Running, WorkflowStatus::Running);
        assert_ne!(WorkflowStatus::Running, WorkflowStatus::Pending);
    }

    #[test]
    fn test_pipeline_status_clone() {
        let status = WorkflowStatus::Paused;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_pipeline_status_debug() {
        let debug_str = format!("{:?}", WorkflowStatus::Running);
        assert_eq!(debug_str, "Running");
    }

    // -----------------------------------------------------------------------
    // WorkflowExecutionError tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_pipeline_error_display_database_connection() {
        let err = WorkflowExecutionError::DatabaseConnection {
            message: "connection refused".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Database connection failed: connection refused"
        );
    }

    #[test]
    fn test_pipeline_error_display_workflow_not_found() {
        let err = WorkflowExecutionError::WorkflowNotFound {
            workflow_name: "my_workflow".to_string(),
        };
        assert_eq!(err.to_string(), "Workflow not found: my_workflow");
    }

    #[test]
    fn test_pipeline_error_display_execution_failed() {
        let err = WorkflowExecutionError::ExecutionFailed {
            message: "something broke".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Pipeline execution failed: something broke"
        );
    }

    #[test]
    fn test_pipeline_error_display_timeout() {
        let err = WorkflowExecutionError::Timeout {
            timeout_seconds: 300,
        };
        assert_eq!(err.to_string(), "Pipeline timeout after 300s");
    }

    #[test]
    fn test_pipeline_error_display_configuration() {
        let err = WorkflowExecutionError::Configuration {
            message: "bad config".to_string(),
        };
        assert_eq!(err.to_string(), "Configuration error: bad config");
    }

    // -----------------------------------------------------------------------
    // TaskResult tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_task_result_construction() {
        let now = Utc::now();
        let result = TaskResult {
            task_name: "extract".to_string(),
            status: TaskState::Completed {
                completion_time: now,
            },
            start_time: Some(now),
            end_time: Some(now),
            duration: Some(Duration::from_secs(5)),
            attempt_count: 1,
            error_message: None,
        };
        assert_eq!(result.task_name, "extract");
        assert_eq!(result.attempt_count, 1);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_task_result_with_error() {
        let result = TaskResult {
            task_name: "transform".to_string(),
            status: TaskState::Failed {
                error: "division by zero".to_string(),
                failure_time: Utc::now(),
            },
            start_time: None,
            end_time: None,
            duration: None,
            attempt_count: 3,
            error_message: Some("division by zero".to_string()),
        };
        assert_eq!(result.error_message.as_deref(), Some("division by zero"));
        assert_eq!(result.attempt_count, 3);
    }

    #[test]
    fn test_task_result_clone() {
        let result = TaskResult {
            task_name: "load".to_string(),
            status: TaskState::Pending,
            start_time: None,
            end_time: None,
            duration: None,
            attempt_count: 0,
            error_message: None,
        };
        let cloned = result.clone();
        assert_eq!(cloned.task_name, result.task_name);
    }

    // -----------------------------------------------------------------------
    // WorkflowExecutionResult tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_pipeline_result_construction() {
        let result = WorkflowExecutionResult {
            execution_id: Uuid::new_v4(),
            workflow_name: "etl_pipeline".to_string(),
            status: WorkflowStatus::Completed,
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration: Some(Duration::from_secs(10)),
            final_context: Context::new(),
            task_results: vec![],
            error_message: None,
        };
        assert_eq!(result.workflow_name, "etl_pipeline");
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert!(result.error_message.is_none());
        assert!(result.task_results.is_empty());
    }

    #[test]
    fn test_pipeline_result_with_tasks() {
        let task1 = TaskResult {
            task_name: "step_1".to_string(),
            status: TaskState::Completed {
                completion_time: Utc::now(),
            },
            start_time: None,
            end_time: None,
            duration: Some(Duration::from_secs(2)),
            attempt_count: 1,
            error_message: None,
        };
        let task2 = TaskResult {
            task_name: "step_2".to_string(),
            status: TaskState::Failed {
                error: "oops".to_string(),
                failure_time: Utc::now(),
            },
            start_time: None,
            end_time: None,
            duration: Some(Duration::from_secs(1)),
            attempt_count: 2,
            error_message: Some("oops".to_string()),
        };
        let result = WorkflowExecutionResult {
            execution_id: Uuid::new_v4(),
            workflow_name: "two_step".to_string(),
            status: WorkflowStatus::Failed,
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration: Some(Duration::from_secs(3)),
            final_context: Context::new(),
            task_results: vec![task1, task2],
            error_message: Some("step_2 failed".to_string()),
        };
        assert_eq!(result.task_results.len(), 2);
        assert_eq!(result.task_results[0].task_name, "step_1");
        assert_eq!(result.task_results[1].task_name, "step_2");
    }

    #[test]
    fn test_pipeline_result_debug() {
        let result = WorkflowExecutionResult {
            execution_id: Uuid::new_v4(),
            workflow_name: "debug_wf".to_string(),
            status: WorkflowStatus::Running,
            start_time: Utc::now(),
            end_time: None,
            duration: None,
            final_context: Context::new(),
            task_results: vec![],
            error_message: None,
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("WorkflowExecutionResult"));
        assert!(debug_str.contains("debug_wf"));
    }
}
