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

//! Workflow execution result building for the DefaultRunner.
//!
//! This module provides methods for building workflow execution results
//! from database records.

use std::time::Duration;
use uuid::Uuid;

use crate::dal::DAL;
use crate::executor::workflow_executor::{
    TaskResult, WorkflowExecutionError, WorkflowExecutionResult, WorkflowStatus,
};
use crate::task::TaskState;
use crate::Context;
use crate::UniversalUuid;

use super::DefaultRunner;

impl DefaultRunner {
    /// Builds a workflow execution result from an execution ID
    ///
    /// # Arguments
    /// * `execution_id` - UUID of the workflow execution
    ///
    /// # Returns
    /// * `Result<WorkflowExecutionResult, WorkflowExecutionError>` - The complete workflow result or an error
    ///
    /// This method:
    /// 1. Retrieves workflow execution details
    /// 2. Gets all task executions
    /// 3. Retrieves the final context
    /// 4. Builds task results
    /// 5. Constructs the complete workflow execution result
    pub(super) async fn build_workflow_result(
        &self,
        execution_id: Uuid,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        let dal = DAL::new(self.database.clone());

        let workflow_execution = dal
            .workflow_execution()
            .get_by_id(UniversalUuid(execution_id))
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get workflow execution: {}", e),
            })?;

        let task_executions = dal
            .task_execution()
            .get_all_tasks_for_pipeline(UniversalUuid(execution_id))
            .await
            .map_err(|e| WorkflowExecutionError::ExecutionFailed {
                message: format!("Failed to get task executions: {}", e),
            })?;

        // Get final context using DAL
        let final_context = if let Some(context_id) = workflow_execution.context_id {
            dal.context().read(context_id).await.map_err(|e| {
                WorkflowExecutionError::ExecutionFailed {
                    message: format!("Failed to get context: {}", e),
                }
            })?
        } else {
            Context::new()
        };

        // Build task results
        let task_results: Vec<TaskResult> = task_executions
            .into_iter()
            .map(|task_exec| {
                let status = match task_exec.status.as_str() {
                    "Pending" => TaskState::Pending,
                    "Running" => TaskState::Running {
                        start_time: task_exec
                            .started_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    "Completed" => TaskState::Completed {
                        completion_time: task_exec
                            .completed_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    "Failed" => TaskState::Failed {
                        error: task_exec
                            .error_details
                            .clone()
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        failure_time: task_exec
                            .completed_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    "Skipped" => TaskState::Skipped {
                        reason: task_exec
                            .error_details
                            .clone()
                            .unwrap_or_else(|| "Trigger rules not satisfied".to_string()),
                        skip_time: task_exec
                            .completed_at
                            .map(|ts| ts.0)
                            .unwrap_or_else(chrono::Utc::now),
                    },
                    _ => TaskState::Failed {
                        error: format!("Unknown status: {}", task_exec.status),
                        failure_time: chrono::Utc::now(),
                    },
                };

                let duration =
                    task_exec
                        .completed_at
                        .zip(task_exec.started_at)
                        .map(|(end, start)| {
                            let end_utc = end.0;
                            let start_utc = start.0;
                            (end_utc - start_utc).to_std().unwrap_or(Duration::ZERO)
                        });

                TaskResult {
                    task_name: task_exec.task_name,
                    status,
                    start_time: task_exec.started_at.map(|ts| ts.0),
                    end_time: task_exec.completed_at.map(|ts| ts.0),
                    duration,
                    attempt_count: task_exec.attempt,
                    error_message: task_exec.error_details,
                }
            })
            .collect();

        // Convert status
        let status = match workflow_execution.status.as_str() {
            "Pending" => WorkflowStatus::Pending,
            "Running" => WorkflowStatus::Running,
            "Completed" => WorkflowStatus::Completed,
            "Failed" => WorkflowStatus::Failed,
            _ => WorkflowStatus::Failed,
        };

        let duration = workflow_execution.completed_at.map(|end| {
            let end_utc = end.0;
            let start_utc = workflow_execution.started_at.0;
            (end_utc - start_utc).to_std().unwrap_or(Duration::ZERO)
        });

        Ok(WorkflowExecutionResult {
            execution_id,
            workflow_name: workflow_execution.workflow_name,
            status,
            start_time: workflow_execution.started_at.0,
            end_time: workflow_execution.completed_at.map(|ts| ts.0),
            duration,
            final_context,
            task_results,
            error_message: workflow_execution.error_details,
        })
    }
}
