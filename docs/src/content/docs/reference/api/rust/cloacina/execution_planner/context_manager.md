---
title: "cloacina::execution_planner::context_manager"
description: "Context management for task execution."
editUrl: false
sidebar:
  order: 112
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Context management for task execution.

This module handles loading and merging contexts for tasks based on
their dependencies.

## Structs

### <span id="struct.ContextManager"></span><code style="font-size: 0.85em">struct ContextManager&lt;&#39;a&gt;</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Context management operations for the scheduler.

#### Methods

##### <span id="ContextManager.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self
```

Creates a new ContextManager.

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: &'a DAL, runtime: Arc<Runtime>) -> Self {
        Self { dal, runtime }
    }
```

</details>

##### <span id="ContextManager.load_context_for_task"></span><code>load_​context_​for_​task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_context_for_task(
    &self,
    task_execution: &TaskExecution,
) -> Result<Context<serde_json::Value>, ValidationError>
```

Loads the context for a specific task based on its dependencies.

<details>
<summary>Source</summary>

```rust
    pub async fn load_context_for_task(
        &self,
        task_execution: &TaskExecution,
    ) -> Result<Context<serde_json::Value>, ValidationError> {
        // Get the workflow to find task dependencies
        let workflow_execution = self
            .dal
            .workflow_execution()
            .get_by_id(task_execution.workflow_execution_id)
            .await?;
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
            // No dependencies: read initial workflow execution context
            if let Some(context_id) = workflow_execution.context_id {
                let context = self.dal.context().read(context_id).await.map_err(|_e| {
                    ValidationError::ContextEvaluationFailed {
                        key: format!("context_id:{}", context_id),
                    }
                })?;
                debug!(
                    "Context loaded: initial workflow execution context ({} keys)",
                    context.data().len()
                );
                Ok(context)
            } else {
                debug!("Context loaded: empty initial context");
                Ok(Context::new())
            }
        } else if dependencies.len() == 1 {
            // Single dependency: read that task's saved context
            let dep_task_namespace = &dependencies[0];
            let dep_task_name = dep_task_namespace.to_string();
            match self
                .dal
                .task_execution_metadata()
                .get_by_workflow_and_task(task_execution.workflow_execution_id, dep_task_namespace)
                .await
            {
                Ok(task_metadata) => {
                    if let Some(context_id) = task_metadata.context_id {
                        match self
                            .dal
                            .context()
                            .read::<serde_json::Value>(context_id)
                            .await
                        {
                            Ok(context) => {
                                debug!(
                                    "Context loaded: from dependency '{}' ({} keys)",
                                    dep_task_name,
                                    context.data().len()
                                );
                                Ok(context)
                            }
                            Err(e) => Err(ValidationError::ContextEvaluationFailed {
                                key: format!("context_read_error:{}", e),
                            }),
                        }
                    } else {
                        // Task completed but has no output context
                        debug!(
                            "Context loaded: empty (dependency '{}' has no output context)",
                            dep_task_name
                        );
                        Ok(Context::new())
                    }
                }
                Err(_) => {
                    // Dependency task hasn't completed yet or no metadata saved
                    debug!(
                        "Context loaded: empty (dependency '{}' not found)",
                        dep_task_name
                    );
                    Ok(Context::new())
                }
            }
        } else {
            // Multiple dependencies: merge their saved contexts
            self.merge_dependency_contexts(task_execution, dependencies)
                .await
        }
    }
```

</details>

##### <span id="ContextManager.evaluate_context_condition"></span><code>evaluate_​context_​condition</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn evaluate_context_condition(
    context: &Context<serde_json::Value>,
    key: &str,
    operator: &ValueOperator,
    expected: &serde_json::Value,
) -> Result<bool, ValidationError>
```

Evaluates a context-based condition using the provided operator.

<details>
<summary>Source</summary>

```rust
    pub fn evaluate_context_condition(
        context: &Context<serde_json::Value>,
        key: &str,
        operator: &ValueOperator,
        expected: &serde_json::Value,
    ) -> Result<bool, ValidationError> {
        let actual = context.get(key);

        match operator {
            ValueOperator::Exists => Ok(actual.is_some()),
            ValueOperator::NotExists => Ok(actual.is_none()),
            ValueOperator::Equals => Ok(actual == Some(expected)),
            ValueOperator::NotEquals => Ok(actual != Some(expected)),
            ValueOperator::GreaterThan => match (actual, expected) {
                (Some(a), b) if a.is_number() && b.is_number() => {
                    Ok(a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0))
                }
                _ => Ok(false),
            },
            ValueOperator::LessThan => match (actual, expected) {
                (Some(a), b) if a.is_number() && b.is_number() => {
                    Ok(a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0))
                }
                _ => Ok(false),
            },
            ValueOperator::Contains => match (actual, expected) {
                (Some(a), b) if a.is_string() && b.is_string() => {
                    Ok(a.as_str().unwrap_or("").contains(b.as_str().unwrap_or("")))
                }
                (Some(a), b) if a.is_array() => Ok(a.as_array().unwrap_or(&vec![]).contains(b)),
                _ => Ok(false),
            },
            ValueOperator::NotContains => Ok(!Self::evaluate_context_condition(
                context,
                key,
                &ValueOperator::Contains,
                expected,
            )?),
        }
    }
```

</details>
