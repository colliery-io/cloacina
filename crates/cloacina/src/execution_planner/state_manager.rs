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

//! Task state management and dependency checking.
//!
//! This module handles checking task dependencies and updating task readiness
//! based on dependency states and trigger rules. Dispatch of Ready tasks is
//! handled separately by the scheduler loop.

use tracing::{debug, info, warn};

use std::collections::HashMap;
use std::sync::Arc;

use crate::dal::DAL;
use crate::error::ValidationError;
use crate::models::task_execution::TaskExecution;
use crate::models::workflow_execution::WorkflowExecutionRecord;
use crate::Runtime;

use super::context_manager::ContextManager;
use super::trigger_rules::{TriggerCondition, TriggerRule};

/// State management operations for the scheduler.
pub struct StateManager<'a> {
    dal: &'a DAL,
    runtime: Arc<Runtime>,
}

impl<'a> StateManager<'a> {
    /// Creates a new StateManager.
    pub fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self {
        Self { dal, runtime }
    }

    /// Updates task readiness for a specific workflow execution using pre-loaded tasks.
    ///
    /// When a task becomes ready, marks it as Ready in the database.
    /// Dispatch to executors is handled separately by the scheduler loop's
    /// dispatch_ready_tasks() method.
    pub async fn update_workflow_task_readiness(
        &self,
        workflow_execution: &WorkflowExecutionRecord,
        pending_tasks: &[TaskExecution],
        statuses: &HashMap<String, String>,
    ) -> Result<(), ValidationError> {
        let workflow_execution_id = workflow_execution.id;
        for task_execution in pending_tasks {
            // CLOACI-T-0745: dependency gating resolves from the pre-loaded
            // per-execution status map — no per-task DB round-trips.
            let dependencies_satisfied =
                self.check_task_dependencies(task_execution, workflow_execution, statuses)?;

            if dependencies_satisfied {
                // All dependencies are in terminal states, now evaluate trigger rules
                let trigger_rules_satisfied = self
                    .evaluate_trigger_rules(task_execution, statuses)
                    .await?;

                if trigger_rules_satisfied {
                    // Mark ready in database - dispatch is handled separately by scheduler_loop
                    self.dal
                        .task_execution()
                        .mark_ready(task_execution.id)
                        .await?;
                    info!("Task ready: {} (workflow execution: {}, dependencies satisfied, trigger rules passed)",
                          task_execution.task_name, workflow_execution_id);
                } else {
                    // Dependencies satisfied + trigger rules fail -> Mark Skipped
                    self.dal
                        .task_execution()
                        .mark_skipped(task_execution.id, "Trigger rules not satisfied")
                        .await?;
                    info!("Task skipped: {} (workflow execution: {}, dependencies satisfied, trigger rules failed)",
                          task_execution.task_name, workflow_execution_id);
                }
            }
        }

        Ok(())
    }

    /// Checks if all dependencies for a task are satisfied.
    /// Dependencies are satisfied when all dependency tasks are in terminal states
    /// (Completed, Failed, or Skipped).
    ///
    /// CLOACI-T-0745: synchronous + map-driven. The caller supplies the
    /// already-loaded `WorkflowExecutionRecord` (no per-task `get_by_id`) and a
    /// `task_name -> status` map for the whole execution (no per-task status
    /// query), so this does zero DB round-trips.
    pub fn check_task_dependencies(
        &self,
        task_execution: &TaskExecution,
        workflow_execution: &WorkflowExecutionRecord,
        statuses: &HashMap<String, String>,
    ) -> Result<bool, ValidationError> {
        let workflow = match self.runtime.get_workflow(&workflow_execution.workflow_name) {
            Some(wf) => wf,
            None => {
                return Err(ValidationError::WorkflowNotFound(
                    workflow_execution.workflow_name.clone(),
                ));
            }
        };

        // Parse the task name string to TaskNamespace
        let task_namespace = crate::task::TaskNamespace::from_string(&task_execution.task_name)
            .map_err(ValidationError::InvalidTaskName)?;

        let dependencies = workflow
            .get_dependencies(&task_namespace)
            .map_err(|e| ValidationError::InvalidTaskName(e.to_string()))?;

        if dependencies.is_empty() {
            return Ok(true);
        }

        // Resolve dependency statuses from the pre-loaded per-execution map.
        let all_satisfied = dependencies.iter().all(|dependency| {
            statuses
                .get(&dependency.to_string())
                .map(|status| matches!(status.as_str(), "Completed" | "Failed" | "Skipped"))
                .unwrap_or_else(|| {
                    warn!(
                        "Dependency task '{}' not found for task '{}'",
                        dependency, task_execution.task_name
                    );
                    false
                })
        });

        Ok(all_satisfied)
    }

    /// Evaluates trigger rules for a task based on its configuration.
    pub async fn evaluate_trigger_rules(
        &self,
        task_execution: &TaskExecution,
        statuses: &HashMap<String, String>,
    ) -> Result<bool, ValidationError> {
        let trigger_rule: TriggerRule = serde_json::from_str(&task_execution.trigger_rules)
            .map_err(|e| ValidationError::InvalidTriggerRule(e.to_string()))?;

        match &trigger_rule {
            TriggerRule::Always => {
                debug!(
                    "Trigger rule evaluation: Always -> true (task: {})",
                    task_execution.task_name
                );
                Ok(true)
            }
            TriggerRule::All { conditions } => {
                debug!(
                    "Trigger rule evaluation: All({} conditions) (task: {})",
                    conditions.len(),
                    task_execution.task_name
                );
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_result = self
                        .evaluate_condition(condition, task_execution, statuses)
                        .await?;
                    debug!(
                        "  └─ Condition {}: {:?} -> {}",
                        i + 1,
                        condition,
                        condition_result
                    );
                    if !condition_result {
                        debug!(
                            "Trigger rule result: All -> false (condition {} failed)",
                            i + 1
                        );
                        return Ok(false);
                    }
                }
                debug!("Trigger rule result: All -> true (all conditions passed)");
                Ok(true)
            }
            TriggerRule::Any { conditions } => {
                debug!(
                    "Trigger rule evaluation: Any({} conditions) (task: {})",
                    conditions.len(),
                    task_execution.task_name
                );
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_result = self
                        .evaluate_condition(condition, task_execution, statuses)
                        .await?;
                    debug!(
                        "  └─ Condition {}: {:?} -> {}",
                        i + 1,
                        condition,
                        condition_result
                    );
                    if condition_result {
                        debug!(
                            "Trigger rule result: Any -> true (condition {} passed)",
                            i + 1
                        );
                        return Ok(true);
                    }
                }
                debug!("Trigger rule result: Any -> false (no conditions passed)");
                Ok(false)
            }
            TriggerRule::None { conditions } => {
                debug!(
                    "Trigger rule evaluation: None({} conditions) (task: {})",
                    conditions.len(),
                    task_execution.task_name
                );
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_result = self
                        .evaluate_condition(condition, task_execution, statuses)
                        .await?;
                    debug!(
                        "  └─ Condition {}: {:?} -> {}",
                        i + 1,
                        condition,
                        condition_result
                    );
                    if condition_result {
                        debug!(
                            "Trigger rule result: None -> false (condition {} passed)",
                            i + 1
                        );
                        return Ok(false);
                    }
                }
                debug!("Trigger rule result: None -> true (no conditions passed)");
                Ok(true)
            }
        }
    }

    /// Evaluates a specific trigger condition.
    async fn evaluate_condition(
        &self,
        condition: &TriggerCondition,
        task_execution: &TaskExecution,
        statuses: &HashMap<String, String>,
    ) -> Result<bool, ValidationError> {
        match condition {
            TriggerCondition::TaskSuccess { task_name } => {
                tracing::debug!(
                    "[DEBUG] Scheduler evaluating TaskSuccess trigger rule: looking up task_name '{}' in workflow execution {}",
                    task_name, task_execution.workflow_execution_id
                );
                // CLOACI-T-0745: read from the pre-loaded per-execution status
                // map; fall back to a query only if a referenced task is absent.
                let status = match statuses.get(task_name) {
                    Some(s) => s.clone(),
                    None => {
                        self.dal
                            .task_execution()
                            .get_task_status(task_execution.workflow_execution_id, task_name)
                            .await?
                    }
                };
                let result = status == "Completed";
                debug!(
                    "    TaskSuccess('{}') -> {} (status: {})",
                    task_name, result, status
                );
                Ok(result)
            }
            TriggerCondition::TaskFailed { task_name } => {
                tracing::debug!(
                    "[DEBUG] Scheduler evaluating TaskFailed trigger rule: looking up task_name '{}' in workflow execution {}",
                    task_name, task_execution.workflow_execution_id
                );
                // CLOACI-T-0745: read from the pre-loaded per-execution status
                // map; fall back to a query only if a referenced task is absent.
                let status = match statuses.get(task_name) {
                    Some(s) => s.clone(),
                    None => {
                        self.dal
                            .task_execution()
                            .get_task_status(task_execution.workflow_execution_id, task_name)
                            .await?
                    }
                };
                let result = status == "Failed";
                debug!(
                    "    TaskFailed('{}') -> {} (status: {})",
                    task_name, result, status
                );
                Ok(result)
            }
            TriggerCondition::TaskSkipped { task_name } => {
                tracing::debug!(
                    "[DEBUG] Scheduler evaluating TaskSkipped trigger rule: looking up task_name '{}' in workflow execution {}",
                    task_name, task_execution.workflow_execution_id
                );
                // CLOACI-T-0745: read from the pre-loaded per-execution status
                // map; fall back to a query only if a referenced task is absent.
                let status = match statuses.get(task_name) {
                    Some(s) => s.clone(),
                    None => {
                        self.dal
                            .task_execution()
                            .get_task_status(task_execution.workflow_execution_id, task_name)
                            .await?
                    }
                };
                let result = status == "Skipped";
                debug!(
                    "    TaskSkipped('{}') -> {} (status: {})",
                    task_name, result, status
                );
                Ok(result)
            }
            TriggerCondition::ContextValue {
                key,
                operator,
                value,
            } => {
                let context_manager = ContextManager::new(self.dal, self.runtime.clone());
                let context = context_manager
                    .load_context_for_task(task_execution)
                    .await?;
                let actual_value = context.get(key);
                let result =
                    ContextManager::evaluate_context_condition(&context, key, operator, value)?;
                debug!(
                    "    ContextValue('{}', {:?}, {}) -> {} (actual: {:?})",
                    key, operator, value, result, actual_value
                );
                Ok(result)
            }
        }
    }
}
