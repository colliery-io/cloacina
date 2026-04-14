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

//! Task recovery and orphan detection.
//!
//! This module handles detection and recovery of tasks that were orphaned
//! due to system interruptions or crashes.

use tracing::{debug, error, info, warn};

use std::sync::Arc;

use crate::dal::DAL;
use crate::error::ValidationError;
use crate::models::recovery_event::{NewRecoveryEvent, RecoveryType};
use crate::models::task_execution::TaskExecution;
use crate::models::workflow_execution::WorkflowExecutionRecord;
use crate::Runtime;

/// Result of attempting to recover a task.
#[derive(Debug)]
pub enum RecoveryResult {
    /// Task was successfully recovered and reset for retry.
    Recovered,
    /// Task was abandoned due to exceeding recovery limits.
    Abandoned,
}

/// Maximum number of recovery attempts before abandoning a task.
const MAX_RECOVERY_ATTEMPTS: i32 = 3;

/// Recovery operations for the scheduler.
pub struct RecoveryManager<'a> {
    dal: &'a DAL,
    runtime: Arc<Runtime>,
}

impl<'a> RecoveryManager<'a> {
    /// Creates a new RecoveryManager.
    pub fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self {
        Self { dal, runtime }
    }

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
    pub async fn recover_orphaned_tasks(&self) -> Result<(), ValidationError> {
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

        // Group tasks by workflow execution to handle workflow availability
        let mut tasks_by_execution: std::collections::HashMap<
            crate::database::universal_types::UniversalUuid,
            (WorkflowExecutionRecord, Vec<TaskExecution>),
        > = std::collections::HashMap::new();

        for task in orphaned_tasks {
            let workflow_exec = self
                .dal
                .workflow_execution()
                .get_by_id(task.workflow_execution_id)
                .await?;
            tasks_by_execution
                .entry(workflow_exec.id)
                .or_insert((workflow_exec, Vec::new()))
                .1
                .push(task);
        }

        let mut recovered_count = 0;
        let mut abandoned_count = 0;
        let mut failed_executions = 0;
        let mut available_workflows = self.runtime.workflow_names();
        available_workflows.sort();

        debug!(
            "Current workflow registry: [{}]",
            available_workflows.join(", ")
        );

        // Process each workflow execution's orphaned tasks
        for (execution_id, (workflow_exec, tasks)) in tasks_by_execution {
            let workflow_exists = self
                .runtime
                .get_workflow(&workflow_exec.workflow_name)
                .is_some();

            if workflow_exists {
                // Known workflow - use existing recovery logic
                info!(
                    "Recovering {} tasks from known workflow '{}'",
                    tasks.len(),
                    workflow_exec.workflow_name
                );
                match self.recover_tasks_for_known_workflow(tasks).await {
                    Ok(recovered) => recovered_count += recovered,
                    Err(e) => {
                        error!(
                            "Failed to recover tasks for workflow execution {}: {}",
                            execution_id, e
                        );
                        // Continue with other workflow executions
                    }
                }
            } else {
                // Unknown workflow - gracefully abandon
                warn!(
                    "Workflow '{}' not in current workflow registry - marking as abandoned",
                    workflow_exec.workflow_name
                );
                debug!(
                    "Found orphaned workflow execution '{}' - not in registry",
                    workflow_exec.workflow_name
                );
                match self
                    .abandon_tasks_for_unknown_workflow(workflow_exec, tasks, &available_workflows)
                    .await
                {
                    Ok(abandoned) => {
                        abandoned_count += abandoned;
                        failed_executions += 1;
                    }
                    Err(e) => {
                        error!(
                            "Failed to abandon tasks for unknown workflow {}: {}",
                            execution_id, e
                        );
                        // Continue with other workflow executions
                    }
                }
            }
        }

        // Log detailed recovery summary
        info!(
            "Recovery Summary:\n  ├─ Tasks Processed: {}\n  ├─ Recovered: {}\n  ├─ Abandoned: {}\n  ├─ Workflow Executions Failed: {}\n  └─ Available Workflows: [{}]",
            recovered_count + abandoned_count, recovered_count, abandoned_count, failed_executions, available_workflows.join(", ")
        );

        Ok(())
    }

    /// Recovers tasks from workflows that are still available in the registry.
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
    async fn abandon_tasks_for_unknown_workflow(
        &self,
        workflow_exec: WorkflowExecutionRecord,
        tasks: Vec<TaskExecution>,
        available_workflows: &[String],
    ) -> Result<usize, ValidationError> {
        // Mark all tasks as abandoned
        for task in &tasks {
            debug!(
                "Abandoning task '{}' (workflow: {})",
                task.task_name, workflow_exec.workflow_name
            );

            self.dal
                .task_execution()
                .mark_abandoned(
                    task.id,
                    &format!(
                        "Workflow '{}' no longer available in registry",
                        workflow_exec.workflow_name
                    ),
                )
                .await?;

            // Record abandonment event with clear reason
            self.record_recovery_event(NewRecoveryEvent {
                workflow_execution_id: workflow_exec.id,
                task_execution_id: Some(task.id),
                recovery_type: RecoveryType::WorkflowUnavailable.into(),
                details: Some(
                    serde_json::json!({
                        "task_name": task.task_name,
                        "workflow_name": workflow_exec.workflow_name,
                        "reason": "Workflow not in current registry",
                        "action": "abandoned",
                        "available_workflows": available_workflows
                    })
                    .to_string(),
                ),
            })
            .await?;
        }

        // Mark workflow execution as failed
        self.dal
            .workflow_execution()
            .mark_failed(
                workflow_exec.id,
                &format!(
                    "Workflow '{}' no longer available - abandoned during recovery",
                    workflow_exec.workflow_name
                ),
            )
            .await?;

        // Record workflow execution-level recovery event
        self.record_recovery_event(NewRecoveryEvent {
            workflow_execution_id: workflow_exec.id,
            task_execution_id: None,
            recovery_type: RecoveryType::WorkflowUnavailable.into(),
            details: Some(
                serde_json::json!({
                    "workflow_name": workflow_exec.workflow_name,
                    "reason": "Workflow not in current registry",
                    "action": "workflow_execution_failed",
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
            workflow_exec.workflow_name
        );

        Ok(tasks.len())
    }

    /// Recovers a single orphaned task with retry limit enforcement.
    async fn recover_single_task(
        &self,
        task: TaskExecution,
    ) -> Result<RecoveryResult, ValidationError> {
        if task.recovery_attempts >= MAX_RECOVERY_ATTEMPTS {
            // Too many recovery attempts - abandon the task and potentially the workflow execution
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
            workflow_execution_id: task.workflow_execution_id,
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

        // Check if this causes the entire workflow execution to fail
        let workflow_failed = self
            .dal
            .task_execution()
            .check_pipeline_failure(task.workflow_execution_id)
            .await?;

        if workflow_failed {
            self.dal
                .workflow_execution()
                .mark_failed(
                    task.workflow_execution_id,
                    "Task abandonment caused workflow execution failure",
                )
                .await?;
        }

        // Record abandonment event
        self.record_recovery_event(NewRecoveryEvent {
            workflow_execution_id: task.workflow_execution_id,
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
}
