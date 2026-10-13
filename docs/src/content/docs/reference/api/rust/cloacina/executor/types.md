---
title: "cloacina::executor::types"
description: "Core types and structures for the Cloacina task execution system."
editUrl: false
sidebar:
  order: 125
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Core types and structures for the Cloacina task execution system.

This module defines the fundamental types used throughout the executor system,
including execution scopes, dependency management, and configuration structures.
These types are used to coordinate task execution, manage dependencies between tasks,
and configure the behavior of the execution engine.

## Structs

### <span id="struct.ExecutionScope"></span><code style="font-size: 0.85em">struct ExecutionScope</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Execution scope information for a context

This structure holds metadata about the current execution context, including
identifiers for both workflow and task executions. It is used to track and
correlate execution contexts throughout the system.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_execution_id` | `UniversalUuid` | Unique identifier for the workflow execution |
| `task_execution_id` | `Option<UniversalUuid>` | Optional unique identifier for the specific task execution |
| `task_name` | `Option<String>` | Optional name of the task being executed |

### <span id="struct.DependencyLoader"></span><code style="font-size: 0.85em">struct DependencyLoader</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

Dependency loader for automatic context merging with lazy loading

This structure manages the loading and caching of task dependencies,
implementing a "latest wins" strategy for context merging. It provides
thread-safe access to dependency contexts through a read-write lock.

#### Methods

##### <span id="DependencyLoader.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    database: Database,
    workflow_execution_id: UniversalUuid,
    dependency_tasks: Vec<crate::task::TaskNamespace>,
) -> Self
```

Creates a new dependency loader instance

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database connection for loading dependencies |
| `workflow_execution_id` | `UniversalUuid` | ID of the workflow execution |
| `dependency_tasks` | `Vec<crate::task::TaskNamespace>` | List of task namespaces that this loader depends on |

<details>
<summary>Source</summary>

```rust
    pub fn new(
        database: Database,
        workflow_execution_id: UniversalUuid,
        dependency_tasks: Vec<crate::task::TaskNamespace>,
    ) -> Self {
        Self {
            database,
            workflow_execution_id,
            dependency_tasks,
            loaded_contexts: RwLock::new(HashMap::new()),
        }
    }
```

</details>

##### <span id="DependencyLoader.load_from_dependencies"></span><code>load_​from_​dependencies</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn load_from_dependencies(
    &self,
    key: &str,
) -> Result<Option<serde_json::Value>, ExecutorError>
```

Loads a value from dependency contexts using a "latest wins" strategy

This method searches through all dependency contexts in reverse order,
returning the first matching value found. If no value is found, returns None.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `&str` | The key to look up in the dependency contexts |

**Returns:**

* `Result<Option<serde_json::Value>, ExecutorError>` - The found value or None if not found

<details>
<summary>Source</summary>

```rust
    pub async fn load_from_dependencies(
        &self,
        key: &str,
    ) -> Result<Option<serde_json::Value>, ExecutorError> {
        // Search dependencies in reverse order (latest wins for overwrites)
        for dep_task_namespace in self.dependency_tasks.iter().rev() {
            let dep_task_name = dep_task_namespace.to_string();
            // Check cache first (read lock)
            {
                let cache = self.loaded_contexts.read().await;
                if let Some(context_data) = cache.get(&dep_task_name) {
                    if let Some(value) = context_data.get(key) {
                        return Ok(Some(value.clone())); // Found! (overwrite strategy)
                    }
                }
            }

            // Lazy load dependency context if not cached (write lock)
            {
                let mut cache = self.loaded_contexts.write().await;
                if !cache.contains_key(&dep_task_name) {
                    let dep_context_data = self
                        .load_dependency_context_data(dep_task_namespace)
                        .await?;
                    cache.insert(dep_task_name.clone(), dep_context_data);
                }

                // Check the newly loaded context
                if let Some(context_data) = cache.get(&dep_task_name) {
                    if let Some(value) = context_data.get(key) {
                        return Ok(Some(value.clone())); // Found! (overwrite strategy)
                    }
                }
            }
        }

        Ok(None) // Key not found in any dependency
    }
```

</details>

### <span id="struct.ExecutorConfig"></span><code style="font-size: 0.85em">struct ExecutorConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration settings for the executor

This structure defines various parameters that control the behavior of the
task execution system, including concurrency limits and timing parameters.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_concurrent_tasks` | `usize` | Maximum number of tasks that can run concurrently |
| `task_timeout` | `std::time::Duration` | Maximum time a task is allowed to run before timing out |
| `enable_claiming` | `bool` | Enable runner-level task claiming for horizontal scaling. When enabled, the executor claims tasks before executing and heartbeats during. |
| `heartbeat_interval` | `std::time::Duration` | Heartbeat interval for claimed tasks (only used when claiming is enabled). |

### <span id="struct.ClaimedTask"></span><code style="font-size: 0.85em">struct ClaimedTask</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

Represents a task that has been claimed for execution

This structure contains the metadata for a task that has been claimed
by an executor instance and is ready to be processed.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` | Unique identifier for this task execution |
| `workflow_execution_id` | `UniversalUuid` | ID of the workflow execution this task belongs to |
| `task_name` | `String` | Name of the task being executed |
| `attempt` | `i32` | Current attempt number for this task execution |
