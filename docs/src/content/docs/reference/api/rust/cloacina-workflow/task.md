---
title: "cloacina-workflow::task"
description: "API reference of the Rust module cloacina-workflow::task."
editUrl: false
sidebar:
  order: 37
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Task Trait and State**

This module provides the core `Task` trait and `TaskState` enum for defining
executable tasks in Cloacina workflows.

## Enums

### <span id="enum.TaskState"></span><code style="font-size: 0.85em">enum TaskState</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Represents the execution state of a task throughout its lifecycle.

Tasks progress through these states during execution, providing visibility
into the current status and enabling proper error handling and recovery.

**State Transitions**

- `Pending` -> `Running`: When task execution begins
- `Running` -> `Completed`: When task completes successfully
- `Running` -> `Failed`: When task encounters an error
- `Failed` -> `Running`: When task is retried

Terminal states (`Completed` and `Failed`) do not transition to other states
unless a retry is attempted.

#### Variants

- **`Pending`** - Task is registered but not yet started
- **`Running`** - Task is currently executing
- **`Completed`** - Task finished successfully
- **`Failed`** - Task encountered an error
- **`Skipped`** - Task was skipped (e.g., trigger rule not satisfied)

#### Methods

##### <span id="TaskState.is_completed"></span><code>is_​completed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_completed(&self) -> bool
```

Returns true if the task is in the completed state

<details>
<summary>Source</summary>

```rust
    pub fn is_completed(&self) -> bool {
        matches!(self, TaskState::Completed { .. })
    }
```

</details>

##### <span id="TaskState.is_failed"></span><code>is_​failed</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_failed(&self) -> bool
```

Returns true if the task is in the failed state

<details>
<summary>Source</summary>

```rust
    pub fn is_failed(&self) -> bool {
        matches!(self, TaskState::Failed { .. })
    }
```

</details>

##### <span id="TaskState.is_running"></span><code>is_​running</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_running(&self) -> bool
```

Returns true if the task is currently running

<details>
<summary>Source</summary>

```rust
    pub fn is_running(&self) -> bool {
        matches!(self, TaskState::Running { .. })
    }
```

</details>

##### <span id="TaskState.is_pending"></span><code>is_​pending</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_pending(&self) -> bool
```

Returns true if the task is pending execution

<details>
<summary>Source</summary>

```rust
    pub fn is_pending(&self) -> bool {
        matches!(self, TaskState::Pending)
    }
```

</details>

##### <span id="TaskState.is_skipped"></span><code>is_​skipped</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_skipped(&self) -> bool
```

Returns true if the task was skipped

<details>
<summary>Source</summary>

```rust
    pub fn is_skipped(&self) -> bool {
        matches!(self, TaskState::Skipped { .. })
    }
```

</details>

## Traits

### <span id="trait.Task"></span><code style="font-size: 0.85em">trait Task</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Core trait that defines an executable task in a pipeline.

Tasks are the fundamental units of work in Cloacina. Most users should use the
`#[task]` macro instead of implementing this trait directly, as the macro provides
automatic registration, code fingerprinting, and convenient syntax.

**Task Execution Model**

Tasks follow a simple but powerful execution model:

1. **Input**: Receive a context containing data from previous tasks
2. **Processing**: Execute the task's business logic
3. **Output**: Update the context with results
4. **Completion**: Return success or failure

**Using the Macro (Recommended)**

```rust
use cloacina_workflow::*;

#[task(id = "my_task", dependencies = [])]
async fn my_task(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    // Your task logic here
    Ok(())
}
```

#### Methods

##### <span id="Task.execute"></span><code>execute</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn execute(
    &self,
    context: Context<serde_json::Value>,
) -> Result<Context<serde_json::Value>, TaskError>
```

Executes the task with the provided context.

This is the main entry point for task execution. The method receives
a context containing data from previous tasks and should return an
updated context with any new or modified data.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `Context<serde_json::Value>` | The execution context containing task data |

**Returns:**

* `Ok(Context)` - Updated context with task results
* `Err(TaskError)` - If the task execution fails

<details>
<summary>Source</summary>

```rust
    async fn execute(
```

</details>

##### <span id="Task.id"></span><code>id</code>

```rust
fn id(&self) -> &str
```

Returns the unique identifier for this task.

The task ID must be unique within a Workflow or TaskRegistry.
It's used for dependency resolution and task lookup.

<details>
<summary>Source</summary>

```rust
    fn id(&self) -> &str;
```

</details>

##### <span id="Task.dependencies"></span><code>dependencies</code>

```rust
fn dependencies(&self) -> &[TaskNamespace]
```

Returns the list of task namespaces that this task depends on.

Dependencies define the execution order - this task will only
execute after all its dependencies have completed successfully.

<details>
<summary>Source</summary>

```rust
    fn dependencies(&self) -> &[TaskNamespace];
```

</details>

##### <span id="Task.checkpoint"></span><code>checkpoint</code>

```rust
fn checkpoint(
    &self,
    _context: &Context<serde_json::Value>,
) -> Result<(), CheckpointError>
```

Saves a checkpoint for this task.

This method is called to save intermediate state during task execution.
The default implementation is a no-op, but tasks can override this
to implement custom checkpointing logic.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `-` | The current execution context |

**Returns:**

* `Ok(())` - If checkpointing succeeds
* `Err(CheckpointError)` - If checkpointing fails

<details>
<summary>Source</summary>

```rust
    fn checkpoint(&self, _context: &Context<serde_json::Value>) -> Result<(), CheckpointError> {
        // Default implementation - tasks can override for custom checkpointing
        Ok(())
    }
```

</details>

##### <span id="Task.retry_policy"></span><code>retry_​policy</code>

```rust
fn retry_policy(&self) -> RetryPolicy
```

Returns the retry policy for this task.

This method defines how the task should behave when it fails, including
the number of retry attempts, backoff strategy, and conditions under
which retries should be attempted.

The default implementation returns a sensible production-ready policy
with exponential backoff and 3 retry attempts.

<details>
<summary>Source</summary>

```rust
    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy::default()
    }
```

</details>

##### <span id="Task.trigger_rules"></span><code>trigger_​rules</code>

```rust
fn trigger_rules(&self) -> serde_json::Value
```

Returns the trigger rules for this task.

Trigger rules define the conditions under which this task should execute
beyond simple dependency satisfaction. The default implementation returns
an "Always" trigger rule, meaning the task executes whenever its dependencies
are satisfied.

**Returns:**

A JSON value representing the trigger rules for this task.

<details>
<summary>Source</summary>

```rust
    fn trigger_rules(&self) -> serde_json::Value {
        serde_json::json!({"type": "Always"})
    }
```

</details>

##### <span id="Task.code_fingerprint"></span><code>code_​fingerprint</code>

```rust
fn code_fingerprint(&self) -> Option<String>
```

Returns a code fingerprint for content-based versioning.

This method should return a hash of the task's implementation code,
enabling automatic detection of changes for Workflow versioning.

The default implementation returns None, indicating that the task
doesn't support code fingerprinting. Tasks generated by the `#[task]`
macro automatically provide fingerprints.

**Returns:**

- `Some(String)` - A hex-encoded hash of the task's code content
- `None` - Task doesn't support code fingerprinting

<details>
<summary>Source</summary>

```rust
    fn code_fingerprint(&self) -> Option<String> {
        None
    }
```

</details>

##### <span id="Task.requires_handle"></span><code>requires_​handle</code>

```rust
fn requires_handle(&self) -> bool
```

Returns whether this task requires a `TaskHandle` for execution control.

Tasks that accept a `TaskHandle` parameter (detected by the `#[task]` macro)
return `true`, signalling the executor to provide a handle via task-local
storage before calling `execute()`.

The default implementation returns `false`.

<details>
<summary>Source</summary>

```rust
    fn requires_handle(&self) -> bool {
        false
    }
```

</details>
