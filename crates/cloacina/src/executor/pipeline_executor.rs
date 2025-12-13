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

//! Pipeline execution engine for workflow orchestration.
//!
//! This module provides the core functionality for executing workflows as pipelines,
//! managing their lifecycle, and handling execution results. It includes support for
//! both synchronous and asynchronous execution, status monitoring, and error handling.
//!
//! # Key Components
//!
//! - `PipelineExecutor`: Core trait defining the execution engine interface
//! - `PipelineExecution`: Handle for managing asynchronous pipeline executions
//! - `PipelineStatus`: Represents the current state of a pipeline
//! - `PipelineResult`: Contains the final outcome of a pipeline execution
//! - `TaskResult`: Represents the outcome of individual task execution
//!
//! # Example
//!
//! ```rust,ignore
//! use cloacina::executor::PipelineExecutor;
//! use cloacina::Context;
//!
//! async fn run_pipeline(executor: &impl PipelineExecutor) {
//!     let context = Context::new(serde_json::json!({}));
//!     match executor.execute("my_workflow", context).await {
//!         Ok(result) => println!("Pipeline completed: {:?}", result),
//!         Err(e) => println!("Pipeline failed: {}", e),
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

/// Callback trait for receiving real-time status updates during pipeline execution.
///
/// Implement this trait to receive notifications about status changes in a running pipeline.
/// This is useful for monitoring progress, updating UI, or triggering dependent actions.
pub trait StatusCallback: Send + Sync {
    /// Called whenever the pipeline status changes.
    ///
    /// # Arguments
    ///
    /// * `status` - The new status of the pipeline
    fn on_status_change(&self, status: PipelineStatus);
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

/// Unified error type for pipeline execution operations.
///
/// This enum represents all possible error conditions that can occur during
/// pipeline execution, including database errors, workflow not found errors,
/// execution failures, timeouts, and various other error types.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
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

/// Represents the current state of a pipeline execution.
///
/// The status transitions through these states during the lifecycle of a pipeline:
/// Pending -> Running -> (Completed | Failed | Cancelled)
///                    <-> Paused (can resume back to Running)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStatus {
    /// Pipeline is queued but not yet started
    Pending,
    /// Pipeline is currently executing
    Running,
    /// Pipeline completed successfully
    Completed,
    /// Pipeline failed during execution
    Failed,
    /// Pipeline was cancelled before completion
    Cancelled,
    /// Pipeline is paused and can be resumed
    Paused,
}

impl PipelineStatus {
    /// Determines if this status represents a terminal state.
    ///
    /// Terminal states are those from which the pipeline cannot transition to another state.
    ///
    /// # Returns
    ///
    /// `true` if the status is Completed, Failed, or Cancelled
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineStatus::Completed | PipelineStatus::Failed | PipelineStatus::Cancelled
        )
    }
}

/// Contains the complete result of a pipeline execution.
///
/// This struct provides comprehensive information about a completed pipeline execution,
/// including timing information, final context state, and results of all tasks.
#[derive(Debug)]
pub struct PipelineResult {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// Name of the workflow that was executed
    pub workflow_name: String,
    /// Final status of the pipeline
    pub status: PipelineStatus,
    /// When the pipeline started execution
    pub start_time: DateTime<Utc>,
    /// When the pipeline completed execution
    pub end_time: Option<DateTime<Utc>>,
    /// Total duration of the pipeline execution
    pub duration: Option<Duration>,
    /// Final state of the execution context
    pub final_context: Context<serde_json::Value>,
    /// Results of all tasks in the pipeline
    pub task_results: Vec<TaskResult>,
    /// Error message if the pipeline failed
    pub error_message: Option<String>,
}

/// Handle for managing an asynchronous pipeline execution.
///
/// This struct provides methods to monitor and control a running pipeline execution.
/// It can be used to check status, wait for completion, or cancel the execution.
pub struct PipelineExecution {
    /// Unique identifier for this execution
    pub execution_id: Uuid,
    /// Name of the workflow being executed
    pub workflow_name: String,
    executor: crate::runner::DefaultRunner,
}

impl PipelineExecution {
    /// Creates a new pipeline execution handle.
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

    /// Waits for the pipeline to complete execution.
    ///
    /// This method blocks until the pipeline reaches a terminal state.
    ///
    /// # Returns
    ///
    /// * `Ok(PipelineResult)` - The final result of the pipeline execution
    /// * `Err(PipelineError)` - If the execution fails or encounters an error
    pub async fn wait_for_completion(self) -> Result<PipelineResult, PipelineError> {
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
    /// * `Ok(PipelineResult)` - The final result of the pipeline execution
    /// * `Err(PipelineError)` - If the execution fails, times out, or encounters an error
    pub async fn wait_for_completion_with_timeout(
        self,
        timeout: Option<Duration>,
    ) -> Result<PipelineResult, PipelineError> {
        let start_time = std::time::Instant::now();

        loop {
            // Check timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() > timeout_duration {
                    return Err(PipelineError::Timeout {
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

    /// Gets the current status of the pipeline execution.
    ///
    /// # Returns
    ///
    /// * `Ok(PipelineStatus)` - The current status of the execution
    /// * `Err(PipelineError)` - If the status cannot be retrieved
    pub async fn get_status(&self) -> Result<PipelineStatus, PipelineError> {
        self.executor.get_execution_status(self.execution_id).await
    }

    /// Cancels the pipeline execution.
    ///
    /// This method attempts to gracefully stop the pipeline execution.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the cancellation was successful
    /// * `Err(PipelineError)` - If the cancellation failed
    pub async fn cancel(&self) -> Result<(), PipelineError> {
        self.executor.cancel_execution(self.execution_id).await
    }

    /// Pauses the pipeline execution.
    ///
    /// When paused, no new tasks will be scheduled, but in-flight tasks will
    /// complete normally. The pipeline can be resumed later.
    ///
    /// # Arguments
    ///
    /// * `reason` - Optional reason for pausing the execution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the pause was successful
    /// * `Err(PipelineError)` - If the pause failed
    pub async fn pause(&self, reason: Option<&str>) -> Result<(), PipelineError> {
        self.executor
            .pause_execution(self.execution_id, reason)
            .await
    }

    /// Resumes a paused pipeline execution.
    ///
    /// The scheduler will resume scheduling tasks for this pipeline on the next
    /// poll cycle.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the resume was successful
    /// * `Err(PipelineError)` - If the resume failed
    pub async fn resume(&self) -> Result<(), PipelineError> {
        self.executor.resume_execution(self.execution_id).await
    }
}

/// Core trait defining the interface for pipeline execution engines.
///
/// This trait provides the fundamental operations for executing and managing
/// workflow pipelines. Implementations should handle the actual execution
/// logic, state management, and error handling.
#[async_trait]
pub trait PipelineExecutor: Send + Sync {
    /// Executes a workflow and waits for completion.
    ///
    /// # Arguments
    ///
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow execution
    ///
    /// # Returns
    ///
    /// * `Ok(PipelineResult)` - The final result of the pipeline execution
    /// * `Err(PipelineError)` - If the execution fails or encounters an error
    async fn execute(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<PipelineResult, PipelineError>;

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
    /// * `Ok(PipelineExecution)` - Handle to the running execution
    /// * `Err(PipelineError)` - If the execution cannot be started
    async fn execute_async(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<PipelineExecution, PipelineError>;

    /// Gets the current status of a running execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to check
    ///
    /// # Returns
    ///
    /// * `Ok(PipelineStatus)` - Current status of the execution
    /// * `Err(PipelineError)` - If the status cannot be retrieved
    async fn get_execution_status(
        &self,
        execution_id: Uuid,
    ) -> Result<PipelineStatus, PipelineError>;

    /// Gets the final result of a completed execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to get results for
    ///
    /// # Returns
    ///
    /// * `Ok(PipelineResult)` - The final result of the execution
    /// * `Err(PipelineError)` - If the result cannot be retrieved
    async fn get_execution_result(
        &self,
        execution_id: Uuid,
    ) -> Result<PipelineResult, PipelineError>;

    /// Cancels a running execution.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to cancel
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the cancellation was successful
    /// * `Err(PipelineError)` - If the cancellation failed
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<(), PipelineError>;

    /// Pauses a running pipeline execution.
    ///
    /// When paused, no new tasks will be scheduled, but in-flight tasks will
    /// complete normally. The pipeline can be resumed later.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to pause
    /// * `reason` - Optional reason for pausing the execution
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the pause was successful
    /// * `Err(PipelineError)` - If the pause failed (e.g., pipeline not running)
    async fn pause_execution(
        &self,
        execution_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), PipelineError>;

    /// Resumes a paused pipeline execution.
    ///
    /// The scheduler will resume scheduling tasks for this pipeline on the next
    /// poll cycle.
    ///
    /// # Arguments
    ///
    /// * `execution_id` - ID of the execution to resume
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the resume was successful
    /// * `Err(PipelineError)` - If the resume failed (e.g., pipeline not paused)
    async fn resume_execution(&self, execution_id: Uuid) -> Result<(), PipelineError>;

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
    /// * `Ok(PipelineResult)` - The final result of the pipeline execution
    /// * `Err(PipelineError)` - If the execution fails or encounters an error
    async fn execute_with_callback(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
        callback: Box<dyn StatusCallback>,
    ) -> Result<PipelineResult, PipelineError>;

    /// Lists recent pipeline executions.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PipelineResult>)` - List of recent execution results
    /// * `Err(PipelineError)` - If the list cannot be retrieved
    async fn list_executions(&self) -> Result<Vec<PipelineResult>, PipelineError>;

    /// Shuts down the executor and its background services.
    ///
    /// This method should be called before the application exits to ensure
    /// proper cleanup of resources.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If shutdown was successful
    /// * `Err(PipelineError)` - If shutdown failed
    async fn shutdown(&self) -> Result<(), PipelineError>;
}

impl PipelineStatus {
    /// Creates a PipelineStatus from a string representation.
    ///
    /// This method is used internally for deserializing pipeline statuses from
    /// various sources (database, API responses, etc.). It provides a consistent
    /// way to convert string representations of pipeline statuses into the
    /// corresponding enum variants.
    ///
    /// # Arguments
    ///
    /// * `s` - String representation of the status
    ///
    /// # Returns
    ///
    /// The corresponding PipelineStatus variant, or Failed if the string is invalid
    ///
    /// # Usage
    /// - Database deserialization
    /// - API response parsing
    /// - Status conversion from external systems
    /// - Testing and validation
    #[allow(dead_code)]
    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "Pending" => PipelineStatus::Pending,
            "Running" => PipelineStatus::Running,
            "Completed" => PipelineStatus::Completed,
            "Failed" => PipelineStatus::Failed,
            "Cancelled" => PipelineStatus::Cancelled,
            "Paused" => PipelineStatus::Paused,
            _ => PipelineStatus::Failed,
        }
    }
}
