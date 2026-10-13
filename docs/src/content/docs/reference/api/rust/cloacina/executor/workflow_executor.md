---
title: "cloacina::executor::workflow_executor"
description: "Workflow execution engine for workflow orchestration."
editUrl: false
sidebar:
  order: 126
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Workflow execution engine for workflow orchestration.

This module provides the core functionality for executing workflows,
managing their lifecycle, and handling execution results. It includes support for
both synchronous and asynchronous execution, status monitoring, and error handling.

**Key Components**

- `WorkflowExecutor`: Core trait defining the execution engine interface
- `WorkflowExecution`: Handle for managing asynchronous workflow executions
- `WorkflowStatus`: Represents the current state of a workflow
- `WorkflowExecutionResult`: Contains the final outcome of a workflow execution
- `TaskResult`: Represents the outcome of individual task execution

**Examples:**

```rust
use cloacina::executor::WorkflowExecutor;
use cloacina::Context;

async fn run_workflow(executor: &impl WorkflowExecutor) {
    let context = Context::new(serde_json::json!({}));
    match executor.execute("my_workflow", context).await {
        Ok(result) => println!("Workflow completed: {:?}", result),
        Err(e) => println!("Workflow failed: {}", e),
    }
}
```

## Structs

### <span id="struct.TaskResult"></span><code style="font-size: 0.85em">struct TaskResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Represents the outcome of a single task execution within a workflow.

This struct contains detailed information about a task's execution, including
timing information, status, and any error messages.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_name` | `String` | Name of the task that was executed |
| `status` | `TaskState` | Final status of the task execution |
| `start_time` | `Option<DateTime<Utc>>` | When the task started execution |
| `end_time` | `Option<DateTime<Utc>>` | When the task completed execution |
| `duration` | `Option<Duration>` | Total duration of the task execution |
| `attempt_count` | `i32` | Number of attempts made to execute the task |
| `error_message` | `Option<String>` | Error message if the task failed |

### <span id="struct.WorkflowExecutionResult"></span><code style="font-size: 0.85em">struct WorkflowExecutionResult</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`

Contains the complete result of a workflow execution.

This struct provides comprehensive information about a completed workflow execution,
including timing information, final context state, and results of all tasks.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | Unique identifier for this execution |
| `workflow_name` | `String` | Name of the workflow that was executed |
| `status` | `WorkflowStatus` | Final status of the workflow |
| `start_time` | `DateTime<Utc>` | When the workflow started execution |
| `end_time` | `Option<DateTime<Utc>>` | When the workflow completed execution |
| `duration` | `Option<Duration>` | Total duration of the workflow execution |
| `final_context` | `Context<serde_json::Value>` | Final state of the execution context |
| `task_results` | `Vec<TaskResult>` | Results of all tasks in the workflow |
| `error_message` | `Option<String>` | Error message if the workflow failed |

### <span id="struct.WorkflowExecution"></span><code style="font-size: 0.85em">struct WorkflowExecution</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Handle for managing an asynchronous workflow execution.

This struct provides methods to monitor and control a running workflow execution.
It can be used to check status, wait for completion, or cancel the execution.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | Unique identifier for this execution |
| `workflow_name` | `String` | Name of the workflow being executed |

#### Methods

##### <span id="WorkflowExecution.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    execution_id: Uuid,
    workflow_name: String,
    executor: crate::runner::DefaultRunner,
) -> Self
```

Creates a new workflow execution handle.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | Unique identifier for the execution |
| `workflow_name` | `String` | Name of the workflow being executed |
| `executor` | `crate::runner::DefaultRunner` | The executor instance managing this execution |

<details>
<summary>Source</summary>

```rust
    pub fn new(
        execution_id: Uuid,
        workflow_name: String,
        executor: crate::runner::DefaultRunner,
    ) -> Self {
        Self {
            execution_id,
            workflow_name,
            executor,
        }
    }
```

</details>

##### <span id="WorkflowExecution.wait_for_completion"></span><code>wait_​for_​completion</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn wait_for_completion(
    self,
) -> Result<WorkflowExecutionResult, WorkflowExecutionError>
```

Waits for the workflow to complete execution.

This method blocks until the workflow reaches a terminal state.

**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
* `Err(WorkflowExecutionError)` - If the execution fails or encounters an error

<details>
<summary>Source</summary>

```rust
    pub async fn wait_for_completion(
        self,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        self.wait_for_completion_with_timeout(None).await
    }
```

</details>

##### <span id="WorkflowExecution.wait_for_completion_with_timeout"></span><code>wait_​for_​completion_​with_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn wait_for_completion_with_timeout(
    self,
    timeout: Option<Duration>,
) -> Result<WorkflowExecutionResult, WorkflowExecutionError>
```

Waits for completion with a specified timeout.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `timeout` | `Option<Duration>` | Optional duration after which to timeout the wait |

**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
* `Err(WorkflowExecutionError)` - If the execution fails, times out, or encounters an error

<details>
<summary>Source</summary>

```rust
    pub async fn wait_for_completion_with_timeout(
        self,
        timeout: Option<Duration>,
    ) -> Result<WorkflowExecutionResult, WorkflowExecutionError> {
        let start_time = std::time::Instant::now();

        loop {
            // Check timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() > timeout_duration {
                    return Err(WorkflowExecutionError::Timeout {
                        timeout_seconds: timeout_duration.as_secs(),
                    });
                }
            }

            // Check status
            match self
                .executor
                .get_execution_status(self.execution_id)
                .await?
            {
                status if status.is_terminal() => {
                    return self.executor.get_execution_result(self.execution_id).await;
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
```

</details>

##### <span id="WorkflowExecution.get_status"></span><code>get_​status</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_status(
    &self,
) -> Result<WorkflowStatus, WorkflowExecutionError>
```

Gets the current status of the workflow execution.

**Returns:**

* `Ok(WorkflowStatus)` - The current status of the execution
* `Err(WorkflowExecutionError)` - If the status cannot be retrieved

<details>
<summary>Source</summary>

```rust
    pub async fn get_status(&self) -> Result<WorkflowStatus, WorkflowExecutionError> {
        self.executor.get_execution_status(self.execution_id).await
    }
```

</details>

##### <span id="WorkflowExecution.cancel"></span><code>cancel</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn cancel(&self) -> Result<(), WorkflowExecutionError>
```

Cancels the workflow execution.

This method attempts to gracefully stop the workflow execution.

**Returns:**

* `Ok(())` - If the cancellation was successful
* `Err(WorkflowExecutionError)` - If the cancellation failed

<details>
<summary>Source</summary>

```rust
    pub async fn cancel(&self) -> Result<(), WorkflowExecutionError> {
        self.executor.cancel_execution(self.execution_id).await
    }
```

</details>

##### <span id="WorkflowExecution.pause"></span><code>pause</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn pause(
    &self) -> Result<(,
    reason: Option<&str>) -> Result<(,
) -> Result<(), WorkflowExecutionError>
```

Pauses the workflow execution.

When paused, no new tasks will be scheduled, but in-flight tasks will
complete normally. The workflow can be resumed later.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `reason` | `Option<&str>` | Optional reason for pausing the execution |

**Returns:**

* `Ok(())` - If the pause was successful
* `Err(WorkflowExecutionError)` - If the pause failed

<details>
<summary>Source</summary>

```rust
    pub async fn pause(&self, reason: Option<&str>) -> Result<(), WorkflowExecutionError> {
        self.executor
            .pause_execution(self.execution_id, reason)
            .await
    }
```

</details>

##### <span id="WorkflowExecution.resume"></span><code>resume</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn resume(&self) -> Result<(), WorkflowExecutionError>
```

Resumes a paused workflow execution.

The scheduler will resume scheduling tasks for this workflow on the next
poll cycle.

**Returns:**

* `Ok(())` - If the resume was successful
* `Err(WorkflowExecutionError)` - If the resume failed

<details>
<summary>Source</summary>

```rust
    pub async fn resume(&self) -> Result<(), WorkflowExecutionError> {
        self.executor.resume_execution(self.execution_id).await
    }
```

</details>

## Enums

### <span id="enum.WorkflowExecutionError"></span><code style="font-size: 0.85em">enum WorkflowExecutionError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Unified error type for workflow execution operations.

This enum represents all possible error conditions that can occur during
workflow execution, including database errors, workflow not found errors,
execution failures, timeouts, and various other error types.

#### Variants

- **`DatabaseConnection`**
- **`WorkflowNotFound`**
- **`ExecutionFailed`**
- **`Timeout`**
- **`Validation`**
- **`TaskExecution`**
- **`Executor`**
- **`Configuration`**

### <span id="enum.WorkflowStatus"></span><code style="font-size: 0.85em">enum WorkflowStatus</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Represents the current state of a workflow execution.

The status transitions through these states during the lifecycle of a workflow:
Pending -> Running -> (Completed | Failed | Cancelled)
                   <-> Paused (can resume back to Running)

#### Variants

- **`Pending`** - Workflow is queued but not yet started
- **`Running`** - Workflow is currently executing
- **`Completed`** - Workflow completed successfully
- **`Failed`** - Workflow failed during execution
- **`Cancelled`** - Workflow was cancelled before completion
- **`Paused`** - Workflow is paused and can be resumed

#### Methods

##### <span id="WorkflowStatus.is_terminal"></span><code>is_​terminal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn is_terminal(&self) -> bool
```

Determines if this status represents a terminal state.

Terminal states are those from which the workflow cannot transition to another state.

**Returns:**

`true` if the status is Completed, Failed, or Cancelled

<details>
<summary>Source</summary>

```rust
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled
        )
    }
```

</details>

##### <span id="WorkflowStatus.parse_status"></span><code>parse_​status</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn parse_status(s: &str) -> Result<Self, String>
```

Parse a `WorkflowStatus` from its string representation.

Fallible — an unrecognized string returns `Err(value)` containing
the offending input. CLOACI-I-0110 / COR-18: callers must
surface unknown statuses explicitly rather than silently
coercing them to `Failed`, which used to mask schema drift.

<details>
<summary>Source</summary>

```rust
    pub fn parse_status(s: &str) -> Result<Self, String> {
        match s {
            "Pending" => Ok(WorkflowStatus::Pending),
            "Running" => Ok(WorkflowStatus::Running),
            "Completed" => Ok(WorkflowStatus::Completed),
            "Failed" => Ok(WorkflowStatus::Failed),
            "Cancelled" => Ok(WorkflowStatus::Cancelled),
            "Paused" => Ok(WorkflowStatus::Paused),
            other => Err(other.to_string()),
        }
    }
```

</details>

## Traits

### <span id="trait.StatusCallback"></span><code style="font-size: 0.85em">trait StatusCallback</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Callback trait for receiving real-time status updates during workflow execution.

Implement this trait to receive notifications about status changes in a running workflow.
This is useful for monitoring progress, updating UI, or triggering dependent actions.

#### Methods

##### <span id="StatusCallback.on_status_change"></span><code>on_​status_​change</code>

```rust
fn on_status_change(&self, status: WorkflowStatus)
```

Called whenever the workflow status changes.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `status` | `WorkflowStatus` | The new status of the workflow |

<details>
<summary>Source</summary>

```rust
    fn on_status_change(&self, status: WorkflowStatus);
```

</details>

### <span id="trait.WorkflowExecutor"></span><code style="font-size: 0.85em">trait WorkflowExecutor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Core trait defining the interface for workflow execution engines.

This trait provides the fundamental operations for executing and managing
workflows. Implementations should handle the actual execution
logic, state management, and error handling.

#### Methods

##### <span id="WorkflowExecutor.execute"></span><code>execute</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn execute(
    &self,
    workflow_name: &str,
    context: Context<serde_json::Value>,
) -> Result<WorkflowExecutionResult, WorkflowExecutionError>
```

Executes a workflow and waits for completion.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `&str` | Name of the workflow to execute |
| `context` | `Context<serde_json::Value>` | Initial context for the workflow execution |

**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
* `Err(WorkflowExecutionError)` - If the execution fails or encounters an error

<details>
<summary>Source</summary>

```rust
    async fn execute(
```

</details>

##### <span id="WorkflowExecutor.execute_async"></span><code>execute_​async</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn execute_async(
    &self,
    workflow_name: &str,
    context: Context<serde_json::Value>,
) -> Result<WorkflowExecution, WorkflowExecutionError>
```

Executes a workflow asynchronously.

This method returns immediately with a handle to the execution,
allowing the caller to monitor progress.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `&str` | Name of the workflow to execute |
| `context` | `Context<serde_json::Value>` | Initial context for the workflow execution |

**Returns:**

* `Ok(WorkflowExecution)` - Handle to the running execution
* `Err(WorkflowExecutionError)` - If the execution cannot be started

<details>
<summary>Source</summary>

```rust
    async fn execute_async(
```

</details>

##### <span id="WorkflowExecutor.get_execution_status"></span><code>get_​execution_​status</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_execution_status(
    &self,
    execution_id: Uuid,
) -> Result<WorkflowStatus, WorkflowExecutionError>
```

Gets the current status of a running execution.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | ID of the execution to check |

**Returns:**

* `Ok(WorkflowStatus)` - Current status of the execution
* `Err(WorkflowExecutionError)` - If the status cannot be retrieved

<details>
<summary>Source</summary>

```rust
    async fn get_execution_status(
```

</details>

##### <span id="WorkflowExecutor.get_execution_result"></span><code>get_​execution_​result</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn get_execution_result(
    &self,
    execution_id: Uuid,
) -> Result<WorkflowExecutionResult, WorkflowExecutionError>
```

Gets the final result of a completed execution.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | ID of the execution to get results for |

**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the execution
* `Err(WorkflowExecutionError)` - If the result cannot be retrieved

<details>
<summary>Source</summary>

```rust
    async fn get_execution_result(
```

</details>

##### <span id="WorkflowExecutor.cancel_execution"></span><code>cancel_​execution</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn cancel_execution(
    &self,
    execution_id: Uuid,
) -> Result<(), WorkflowExecutionError>
```

Cancels a running execution.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | ID of the execution to cancel |

**Returns:**

* `Ok(())` - If the cancellation was successful
* `Err(WorkflowExecutionError)` - If the cancellation failed

<details>
<summary>Source</summary>

```rust
    async fn cancel_execution(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError>;
```

</details>

##### <span id="WorkflowExecutor.pause_execution"></span><code>pause_​execution</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn pause_execution(
    &self,
    execution_id: Uuid,
    reason: Option<&str>,
) -> Result<(), WorkflowExecutionError>
```

Pauses a running workflow execution.

When paused, no new tasks will be scheduled, but in-flight tasks will
complete normally. The workflow can be resumed later.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | ID of the execution to pause |
| `reason` | `Option<&str>` | Optional reason for pausing the execution |

**Returns:**

* `Ok(())` - If the pause was successful
* `Err(WorkflowExecutionError)` - If the pause failed (e.g., workflow not running)

<details>
<summary>Source</summary>

```rust
    async fn pause_execution(
```

</details>

##### <span id="WorkflowExecutor.resume_execution"></span><code>resume_​execution</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn resume_execution(
    &self,
    execution_id: Uuid,
) -> Result<(), WorkflowExecutionError>
```

Resumes a paused workflow execution.

The scheduler will resume scheduling tasks for this workflow on the next
poll cycle.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | ID of the execution to resume |

**Returns:**

* `Ok(())` - If the resume was successful
* `Err(WorkflowExecutionError)` - If the resume failed (e.g., workflow not paused)

<details>
<summary>Source</summary>

```rust
    async fn resume_execution(&self, execution_id: Uuid) -> Result<(), WorkflowExecutionError>;
```

</details>

##### <span id="WorkflowExecutor.execute_with_callback"></span><code>execute_​with_​callback</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn execute_with_callback(
    &self,
    workflow_name: &str,
    context: Context<serde_json::Value>,
    callback: Box<dyn StatusCallback>,
) -> Result<WorkflowExecutionResult, WorkflowExecutionError>
```

Executes a workflow with status updates via callback.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `&str` | Name of the workflow to execute |
| `context` | `Context<serde_json::Value>` | Initial context for the workflow execution |
| `callback` | `Box<dyn StatusCallback>` | Callback to receive status updates |

**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the workflow execution
* `Err(WorkflowExecutionError)` - If the execution fails or encounters an error

<details>
<summary>Source</summary>

```rust
    async fn execute_with_callback(
```

</details>

##### <span id="WorkflowExecutor.list_executions"></span><code>list_​executions</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn list_executions(
    &self,
) -> Result<Vec<WorkflowExecutionResult>, WorkflowExecutionError>
```

Lists recent workflow executions.

**Returns:**

* `Ok(Vec<WorkflowExecutionResult>)` - List of recent execution results
* `Err(WorkflowExecutionError)` - If the list cannot be retrieved

<details>
<summary>Source</summary>

```rust
    async fn list_executions(&self)
```

</details>

##### <span id="WorkflowExecutor.shutdown"></span><code>shutdown</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn shutdown(&self) -> Result<(), WorkflowExecutionError>
```

Shuts down the executor and its background services.

This method should be called before the application exits to ensure
proper cleanup of resources.

**Returns:**

* `Ok(())` - If shutdown was successful
* `Err(WorkflowExecutionError)` - If shutdown failed

<details>
<summary>Source</summary>

```rust
    async fn shutdown(&self) -> Result<(), WorkflowExecutionError>;
```

</details>
