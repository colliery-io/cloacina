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

//! WorkflowExecutor trait implementation for DefaultRunner.
//!
//! This module provides the core workflow execution functionality including
//! synchronous and asynchronous execution, status monitoring, and result retrieval.

use async_trait::async_trait;
use std::time::Duration;
use uuid::Uuid;

use crate::dal::DAL;
use crate::executor::pipeline_executor::{
    WorkflowExecution, WorkflowExecutionError, WorkflowExecutionResult, WorkflowExecutor,
    WorkflowStatus,
};
use crate::Context;
use crate::UniversalUuid;

use super::DefaultRunner;

/// Implementation of WorkflowExecutor trait for DefaultRunner
///
/// This implementation provides the core workflow execution functionality:
/// - Synchronous and asynchronous execution
/// - Status monitoring and result retrieval
/// - Execution cancellation
/// - Execution listing and management
#[async_trait]
impl WorkflowExecutor for DefaultRunner {
    /// Executes a workflow synchronously and waits for completion
    ///
    /// # Arguments
    /// * `workflow_name` - Name of the workflow to execute
    /// * `context` - Initial context for the workflow
    ///
    /// # Returns
    /// * `Result<WorkflowExecutionResult, WorkflowExecutionError>` - The execution result or an error
    ///
    /// This method will block until the workflow completes or times out.
    async fn execute(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        // Schedule execution
        let execution_id = self
            .scheduler
            .schedule_workflow_execution(workflow_name, context)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to schedule workflow: {}", e),
            })?;

        // Wait for completion
        let start_time = std::time::Instant::now();
        let dal = DAL::new(self.database.clone());

        loop {
            // Check timeout
            if let Some(timeout) = self.config.pipeline_timeout() {
                if start_time.elapsed() > timeout {
                    return Err(WorkflowExecutionError::Timeout {
                        timeout_seconds: timeout.as_secs(),
                    });
                }
            }

            // Check status
            let pipeline = dal
                .workflow_execution()
                .get_by_id(UniversalUuid(execution_id))
                .await
                .map_err(|e| WorkflowExecutionError::ExecutionFailed {
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
    /// * `Result<WorkflowExecution, WorkflowExecutionError>` - A handle to the execution or an error
    ///
    /// This method returns immediately with an execution handle that can be used
    /// to monitor the workflow's progress.
    async fn execute_async(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
    ) -> Result<WorkflowExecution, WorkflowExecutionError> {
        // Schedule execution
        let execution_id = self
            .scheduler
            .schedule_workflow_execution(workflow_name, context)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to schedule workflow: {}", e),
            })?;

        Ok(WorkflowExecution::new(
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
    /// * `Result<WorkflowExecutionResult, WorkflowExecutionError>` - The execution result or an error
    ///
    /// This method will block until completion but provides status updates
    /// through the callback interface.
    async fn execute_with_callback(
        &self,
        workflow_name: &str,
        context: Context<serde_json::Value>,
        callback: Box<dyn crate::executor::pipeline_executor::StatusCallback>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        // Start async execution
        let execution = self.execute_async(workflow_name, context).await?;
        let execution_id = execution.execution_id;

        // Poll for status changes and call callback
        let mut last_status = WorkflowStatus::Pending;
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

    /// Gets the current status of a workflow execution
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the workflow execution
    ///
    /// # Returns
    /// * `Result<WorkflowStatus, WorkflowExecutionError>` - The current status or an error
    async fn get_execution_status(
        &self,
        execution_id: Uuid,
    ) -> Result<WorkflowStatus, WorkflowExecutionError> {
        let dal = DAL::new(self.database.clone());
        let pipeline = dal
            .workflow_execution()
            .get_by_id(UniversalUuid(execution_id))
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get execution status: {}", e),
            })?;

        let status = match pipeline.status.as_str() {
            "Pending" => WorkflowStatus::Pending,
            "Running" => WorkflowStatus::Running,
            "Completed" => WorkflowStatus::Completed,
            "Failed" => WorkflowStatus::Failed,
            "Cancelled" => WorkflowStatus::Cancelled,
            "Paused" => WorkflowStatus::Paused,
            _ => WorkflowStatus::Failed,
        };

        Ok(status)
    }

    /// Gets the complete result of a workflow execution
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the workflow execution
    ///
    /// # Returns
    /// * `Result<WorkflowExecutionResult, WorkflowExecutionError>` - The complete result or an error
    async fn get_execution_result(
        &self,
        execution_id: Uuid,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        self.build_pipeline_result(execution_id).await
    }

    /// Cancels an in-progress workflow execution
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the workflow execution to cancel
    ///
    /// # Returns
    /// * `Result<(), WorkflowExecutionError>` - Success or error status
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError> {
        // Implementation would mark execution as cancelled in database
        // and notify scheduler/executor to stop processing
        let dal = DAL::new(self.database.clone());

        dal.workflow_execution()
            .cancel(execution_id.into())
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to cancel execution: {}", e),
            })?;

        Ok(())
    }

    /// Pauses a running workflow execution
    ///
    /// When paused, no new tasks will be scheduled, but in-flight tasks will
    /// complete normally. The workflow can be resumed later.
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the workflow execution to pause
    /// * `reason` - Optional reason for pausing the execution
    ///
    /// # Returns
    /// * `Result<(), WorkflowExecutionError>` - Success or error status
    async fn pause_execution(
        &self,
        execution_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), WorkflowExecutionError> {
        let dal = DAL::new(self.database.clone());

        // Verify the workflow is in a pausable state (Pending or Running)
        let pipeline = dal
            .workflow_execution()
            .get_by_id(UniversalUuid(execution_id))
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get execution: {}", e),
            })?;

        // Allow pausing both Pending and Running workflows
        // Pending = waiting to start, Running = actively executing
        if pipeline.status != "Running" && pipeline.status != "Pending" {
            return Err(WorkflowExecutionError::ExecutionFailed {
                message: format!(
                    "Cannot pause workflow with status '{}'. Only 'Pending' or 'Running' workflows can be paused.",
                    pipeline.status
                ),
            });
        }

        dal.workflow_execution()
            .pause(execution_id.into(), reason)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to pause execution: {}", e),
            })?;

        Ok(())
    }

    /// Resumes a paused workflow execution
    ///
    /// The scheduler will resume scheduling tasks for this workflow on the next
    /// poll cycle.
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the workflow execution to resume
    ///
    /// # Returns
    /// * `Result<(), WorkflowExecutionError>` - Success or error status
    async fn resume_execution(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError> {
        let dal = DAL::new(self.database.clone());

        // Verify the workflow is in a resumable state (Paused)
        let pipeline = dal
            .workflow_execution()
            .get_by_id(UniversalUuid(execution_id))
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get execution: {}", e),
            })?;

        if pipeline.status != "Paused" {
            return Err(WorkflowExecutionError::ExecutionFailed {
                message: format!(
                    "Cannot resume workflow with status '{}'. Only 'Paused' workflows can be resumed.",
                    pipeline.status
                ),
            });
        }

        dal.workflow_execution()
            .resume(execution_id.into())
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to resume execution: {}", e),
            })?;

        Ok(())
    }

    /// Lists recent workflow executions
    ///
    /// # Returns
    /// * `Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError>` - List of recent executions or an error
    ///
    /// Currently limited to the 100 most recent executions.
    async fn list_executions(
        &self,
    ) -> Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError> {
        let dal = DAL::new(self.database.clone());

        let executions = dal
            .workflow_execution()
            .list_recent(100)
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
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
    /// * `Result<(), WorkflowExecutionError>` - Success or error status
    async fn shutdown(&self) -> Result<(), WorkflowExecutionError> {
        DefaultRunner::shutdown(self).await
    }
}
