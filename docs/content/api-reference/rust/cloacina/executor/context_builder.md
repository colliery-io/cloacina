# cloacina::executor::context_builder <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Shared dependency-context builder (CLOACI-I-0114 / task T-0633).

Extracted verbatim from `ThreadTaskExecutor::build_task_context` +
`merge_context_values` so the upcoming `FleetExecutor` resolves the merged
dependency context the **exact** same way the thread executor does. Without
this seam the two executors would inevitably drift on what context a task
sees — mirroring the same drift risk `TaskResultHandler` (T-0630) closed
for the post-execution path.
The builder is intentionally backend-agnostic and holds only a `DAL`; the
caller supplies the task's dependency namespaces (the thread executor gets
them from the locally-loaded `Task::dependencies()`; the fleet executor
from the same server-side `Runtime`).

## Structs

### `cloacina::executor::context_builder::TaskContextBuilder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`

Builds a task's input context by loading + merging its dependency contexts (or the workflow's initial context when the task has no dependencies).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (dal : DAL) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(dal: DAL) -> Self {
        Self { dal }
    }
```

</details>



##### `build` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn build (& self , claimed_task : & ClaimedTask , dependencies : & [TaskNamespace] ,) -> Result < Context < serde_json :: Value > , ExecutorError >
```

Build the execution context for `claimed_task` given its `dependencies`.

- **No dependencies**: load the workflow's initial context (if any).
- **With dependencies**: batch-load each dependency's persisted context
and smart-merge (latest-wins for primitives, recursive for objects,
dedup-concat for arrays). A dependency context that fails to parse is
a hard `ContextLoadFailed` (COR-11) — never a silent partial context.

<details>
<summary>Source</summary>

```rust
    pub async fn build(
        &self,
        claimed_task: &ClaimedTask,
        dependencies: &[TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        debug!(
            "Building context for task '{}' with {} dependencies: {:?}",
            claimed_task.task_name,
            dependencies.len(),
            dependencies
        );

        let mut context = Context::new();

        // Load initial workflow context if task has no dependencies.
        if dependencies.is_empty() {
            if let Ok(workflow_execution) = self
                .dal
                .workflow_execution()
                .get_by_id(claimed_task.workflow_execution_id)
                .await
            {
                if let Some(context_id) = workflow_execution.context_id {
                    if let Ok(initial_context) = self
                        .dal
                        .context()
                        .read::<serde_json::Value>(context_id)
                        .await
                    {
                        for (key, value) in initial_context.data() {
                            let _ = context.insert(key, value.clone());
                        }
                        debug!(
                            "Loaded initial workflow context with {} keys",
                            initial_context.data().len()
                        );
                    }
                }
            }
            return Ok(context);
        }

        // Batch-load dependency contexts (eager loading strategy).
        debug!(
            "Loading dependency contexts for {} dependencies: {:?}",
            dependencies.len(),
            dependencies
        );
        let dep_metadata_with_contexts = self
            .dal
            .task_execution_metadata()
            .get_dependency_metadata_with_contexts(claimed_task.workflow_execution_id, dependencies)
            .await
            .map_err(|e| {
                error!(
                    "Failed to load dependency contexts for task '{}': {}",
                    claimed_task.task_name, e
                );
                ExecutorError::ContextLoadFailed(format!(
                    "dependency context load failed for '{}': {}",
                    claimed_task.task_name, e
                ))
            })?;

        debug!(
            "Found {} dependency metadata records",
            dep_metadata_with_contexts.len()
        );
        for (_task_metadata, context_json) in dep_metadata_with_contexts {
            if let Some(json_str) = context_json {
                // COR-11: a parse failure here fails the task explicitly rather
                // than silently continuing with a partial context.
                let dep_context = match Context::<serde_json::Value>::from_json(json_str) {
                    Ok(c) => c,
                    Err(e) => {
                        metrics::counter!(
                            "cloacina_context_merge_failures_total",
                            "kind" => "parse",
                        )
                        .increment(1);
                        return Err(ExecutorError::ContextLoadFailed(format!(
                            "dependency context JSON parse failed for task '{}': {}",
                            claimed_task.task_name, e
                        )));
                    }
                };
                debug!(
                    "Merging dependency context with {} keys: {:?}",
                    dep_context.data().len(),
                    dep_context.data().keys().collect::<Vec<_>>()
                );
                for (key, value) in dep_context.data() {
                    if let Some(existing_value) = context.get(key) {
                        let merged_value = Self::merge_context_values(existing_value, value);
                        if context.update(key, merged_value).is_err() {
                            metrics::counter!(
                                "cloacina_context_merge_failures_total",
                                "kind" => "merge",
                            )
                            .increment(1);
                        }
                    } else if context.insert(key, value.clone()).is_err() {
                        metrics::counter!(
                            "cloacina_context_merge_failures_total",
                            "kind" => "merge",
                        )
                        .increment(1);
                    }
                }
            }
        }

        debug!(
            "Final context for task {} has {} keys: {:?}",
            claimed_task.task_name,
            context.data().len(),
            context.data().keys().collect::<Vec<_>>()
        );
        Ok(context)
    }
```

</details>



##### `merge_context_values` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn merge_context_values (existing : & serde_json :: Value , new : & serde_json :: Value ,) -> serde_json :: Value
```

Smart-merge two context values: arrays concat+dedup, objects merge recursively, everything else latest-wins.

<details>
<summary>Source</summary>

```rust
    pub fn merge_context_values(
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> serde_json::Value {
        use serde_json::Value;

        match (existing, new) {
            (Value::Array(existing_arr), Value::Array(new_arr)) => {
                let mut merged = existing_arr.clone();
                for item in new_arr {
                    if !merged.contains(item) {
                        merged.push(item.clone());
                    }
                }
                Value::Array(merged)
            }
            (Value::Object(existing_obj), Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    if let Some(existing_value) = merged.get(key) {
                        merged.insert(
                            key.clone(),
                            Self::merge_context_values(existing_value, value),
                        );
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(merged)
            }
            (_, new_value) => new_value.clone(),
        }
    }
```

</details>
