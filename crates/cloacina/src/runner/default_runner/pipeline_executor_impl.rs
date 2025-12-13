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

//! PipelineExecutor trait implementation for DefaultRunner.
//!
//! This module provides the core workflow execution functionality including
//! synchronous and asynchronous execution, status monitoring, and result retrieval.

use async_trait::async_trait;
use std::time::Duration;
use uuid::Uuid;

use crate::dal::DAL;
use crate::executor::pipeline_executor::{
    PipelineError, PipelineExecution, PipelineExecutor, PipelineResult, PipelineStatus,
};
use crate::Context;
use crate::UniversalUuid;

use super::DefaultRunner;

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
        let dal = DAL::new(self.database.clone());

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
        let dal = DAL::new(self.database.clone());
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
            "Cancelled" => PipelineStatus::Cancelled,
            "Paused" => PipelineStatus::Paused,
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
        let dal = DAL::new(self.database.clone());

        dal.pipeline_execution()
            .cancel(execution_id.into())
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to cancel execution: {}", e),
            })?;

        Ok(())
    }

    /// Pauses a running pipeline execution
    ///
    /// When paused, no new tasks will be scheduled, but in-flight tasks will
    /// complete normally. The pipeline can be resumed later.
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the pipeline execution to pause
    /// * `reason` - Optional reason for pausing the execution
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error status
    async fn pause_execution(
        &self,
        execution_id: Uuid,
        reason: Option<&str>,
    ) -> Result<(), PipelineError> {
        let dal = DAL::new(self.database.clone());

        // Verify the pipeline is in a pausable state (Pending or Running)
        let pipeline = dal
            .pipeline_execution()
            .get_by_id(UniversalUuid(execution_id))
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get execution: {}", e),
            })?;

        // Allow pausing both Pending and Running pipelines
        // Pending = waiting to start, Running = actively executing
        if pipeline.status != "Running" && pipeline.status != "Pending" {
            return Err(PipelineError::ExecutionFailed {
                message: format!(
                    "Cannot pause pipeline with status '{}'. Only 'Pending' or 'Running' pipelines can be paused.",
                    pipeline.status
                ),
            });
        }

        dal.pipeline_execution()
            .pause(execution_id.into(), reason)
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to pause execution: {}", e),
            })?;

        Ok(())
    }

    /// Resumes a paused pipeline execution
    ///
    /// The scheduler will resume scheduling tasks for this pipeline on the next
    /// poll cycle.
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the pipeline execution to resume
    ///
    /// # Returns
    /// * `Result<(), PipelineError>` - Success or error status
    async fn resume_execution(&self, execution_id: Uuid) -> Result<(), PipelineError> {
        let dal = DAL::new(self.database.clone());

        // Verify the pipeline is in a resumable state (Paused)
        let pipeline = dal
            .pipeline_execution()
            .get_by_id(UniversalUuid(execution_id))
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to get execution: {}", e),
            })?;

        if pipeline.status != "Paused" {
            return Err(PipelineError::ExecutionFailed {
                message: format!(
                    "Cannot resume pipeline with status '{}'. Only 'Paused' pipelines can be resumed.",
                    pipeline.status
                ),
            });
        }

        dal.pipeline_execution()
            .resume(execution_id.into())
            .await
            .map_err(|e| PipelineError::ExecutionFailed {
                message: format!("Failed to resume execution: {}", e),
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
        let dal = DAL::new(self.database.clone());

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
