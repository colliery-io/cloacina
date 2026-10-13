---
title: "cloacina::execution_planner::state_manager"
description: "Task state management and dependency checking."
editUrl: false
sidebar:
  order: 115
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Task state management and dependency checking.

This module handles checking task dependencies and updating task readiness
based on dependency states and trigger rules. Dispatch of Ready tasks is
handled separately by the scheduler loop.

## Structs

### <span id="struct.StateManager"></span><code style="font-size: 0.85em">struct StateManager&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

State management operations for the scheduler.

#### Methods

##### <span id="StateManager.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self
```

Creates a new StateManager.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self {
        Self { dal, runtime }
    }
```

</details>

##### <span id="StateManager.update_workflow_task_readiness"></span><code>update_​workflow_​task_​readiness</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn update_workflow_task_readiness(
    &self,
    workflow_execution: &WorkflowExecutionRecord,
    pending_tasks: &[TaskExecution],
    statuses: &HashMap<String, String>,
) -> Result<(), ValidationError>
```

Updates task readiness for a specific workflow execution using pre-loaded tasks.

When a task becomes ready, marks it as Ready in the database.
Dispatch to executors is handled separately by the scheduler loop's
dispatch_ready_tasks() method.

<details>
<summary>Source</summary>

```rust
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
                    info!(
                        "Task ready: {} (workflow execution: {}, dependencies satisfied, trigger rules passed)",
                        task_execution.task_name, workflow_execution_id
                    );
                } else {
                    // Dependencies satisfied + trigger rules fail -> Mark Skipped
                    self.dal
                        .task_execution()
                        .mark_skipped(task_execution.id, "Trigger rules not satisfied")
                        .await?;
                    info!(
                        "Task skipped: {} (workflow execution: {}, dependencies satisfied, trigger rules failed)",
                        task_execution.task_name, workflow_execution_id
                    );
                }
            }
        }

        Ok(())
    }
```

</details>

##### <span id="StateManager.check_task_dependencies"></span><code>check_​task_​dependencies</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn check_task_dependencies(
    &self,
    task_execution: &TaskExecution,
    workflow_execution: &WorkflowExecutionRecord,
    statuses: &HashMap<String, String>,
) -> Result<bool, ValidationError>
```

Checks if all dependencies for a task are satisfied. Dependencies are satisfied when all dependency tasks are in terminal states (Completed, Failed, or Skipped).

CLOACI-T-0745: synchronous + map-driven. The caller supplies the
already-loaded `WorkflowExecutionRecord` (no per-task `get_by_id`) and a
`task_name -> status` map for the whole execution (no per-task status
query), so this does zero DB round-trips.

<details>
<summary>Source</summary>

```rust
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
```

</details>

##### <span id="StateManager.evaluate_trigger_rules"></span><code>evaluate_​trigger_​rules</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn evaluate_trigger_rules(
    &self,
    task_execution: &TaskExecution,
    statuses: &HashMap<String, String>,
) -> Result<bool, ValidationError>
```

Evaluates trigger rules for a task based on its configuration.

<details>
<summary>Source</summary>

```rust
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
```

</details>
