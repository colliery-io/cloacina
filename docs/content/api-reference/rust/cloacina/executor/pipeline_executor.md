# cloacina::executor::pipeline_executor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Workflow execution engine for workflow orchestration.

This module provides the core functionality for executing workflows,
managing their lifecycle, and handling execution results. It includes support for
both synchronous and asynchronous execution, status monitoring, and error handling.

**Examples:**

```rust,ignore
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

### `cloacina::executor::pipeline_executor::TaskResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Represents the outcome of a single task execution within a pipeline.

This struct contains detailed information about a task's execution, including
timing information, status, and any error messages.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_name` | `String` | Name of the task that was executed |
| `status` | `TaskState` | Final status of the task execution |
| `start_time` | `Option < DateTime < Utc > >` | When the task started execution |
| `end_time` | `Option < DateTime < Utc > >` | When the task completed execution |
| `duration` | `Option < Duration >` | Total duration of the task execution |
| `attempt_count` | `i32` | Number of attempts made to execute the task |
| `error_message` | `Option < String >` | Error message if the task failed |



### `cloacina::executor::pipeline_executor::WorkflowExecutionResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
| `start_time` | `DateTime < Utc >` | When the workflow started execution |
| `end_time` | `Option < DateTime < Utc > >` | When the workflow completed execution |
| `duration` | `Option < Duration >` | Total duration of the workflow execution |
| `final_context` | `Context < serde_json :: Value >` | Final state of the execution context |
| `task_results` | `Vec < TaskResult >` | Results of all tasks in the workflow |
| `error_message` | `Option < String >` | Error message if the workflow failed |



### `cloacina::executor::pipeline_executor::WorkflowExecution`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Handle for managing an asynchronous workflow execution.

This struct provides methods to monitor and control a running workflow execution.
It can be used to check status, wait for completion, or cancel the execution.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `Uuid` | Unique identifier for this execution |
| `workflow_name` | `String` | Name of the workflow being executed |
| `executor` | `crate :: runner :: DefaultRunner` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (execution_id : Uuid , workflow_name : String , executor : crate :: runner :: DefaultRunner ,) -> Self
```

Creates a new workflow execution handle.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `execution_id` | `-` | Unique identifier for the execution |
| `workflow_name` | `-` | Name of the workflow being executed |
| `executor` | `-` | The executor instance managing this execution |


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



##### `wait_for_completion` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn wait_for_completion (self ,) -> Result < WorkflowExecutionResult , WorkflowExecutionError >
```

Waits for the workflow to complete execution.

This method blocks until the workflow reaches a terminal state.

**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the workflow execution * `Err(WorkflowExecutionError)` - If the execution fails or encounters an error

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



##### `wait_for_completion_with_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn wait_for_completion_with_timeout (self , timeout : Option < Duration > ,) -> Result < WorkflowExecutionResult , WorkflowExecutionError >
```

Waits for completion with a specified timeout.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `timeout` | `-` | Optional duration after which to timeout the wait |


**Returns:**

* `Ok(WorkflowExecutionResult)` - The final result of the workflow execution * `Err(WorkflowExecutionError)` - If the execution fails, times out, or encounters an error

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



##### `get_status` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn get_status (& self) -> Result < WorkflowStatus , WorkflowExecutionError >
```

Gets the current status of the workflow execution.

**Returns:**

* `Ok(WorkflowStatus)` - The current status of the execution * `Err(WorkflowExecutionError)` - If the status cannot be retrieved

<details>
<summary>Source</summary>

```rust
    pub async fn get_status(&self) -> Result<WorkflowStatus, WorkflowExecutionError> {
        self.executor.get_execution_status(self.execution_id).await
    }
```

</details>



##### `cancel` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn cancel (& self) -> Result < () , WorkflowExecutionError >
```

Cancels the workflow execution.

This method attempts to gracefully stop the workflow execution.

**Returns:**

* `Ok(())` - If the cancellation was successful * `Err(WorkflowExecutionError)` - If the cancellation failed

<details>
<summary>Source</summary>

```rust
    pub async fn cancel(&self) -> Result<(), WorkflowExecutionError> {
        self.executor.cancel_execution(self.execution_id).await
    }
```

</details>



##### `pause` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn pause (& self , reason : Option < & str >) -> Result < () , WorkflowExecutionError >
```

Pauses the workflow execution.

When paused, no new tasks will be scheduled, but in-flight tasks will
complete normally. The workflow can be resumed later.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `reason` | `-` | Optional reason for pausing the execution |


**Returns:**

* `Ok(())` - If the pause was successful * `Err(WorkflowExecutionError)` - If the pause failed

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



##### `resume` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn resume (& self) -> Result < () , WorkflowExecutionError >
```

Resumes a paused workflow execution.

The scheduler will resume scheduling tasks for this workflow on the next
poll cycle.

**Returns:**

* `Ok(())` - If the resume was successful * `Err(WorkflowExecutionError)` - If the resume failed

<details>
<summary>Source</summary>

```rust
    pub async fn resume(&self) -> Result<(), WorkflowExecutionError> {
        self.executor.resume_execution(self.execution_id).await
    }
```

</details>





## Enums

### `cloacina::executor::pipeline_executor::WorkflowExecutionError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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



### `cloacina::executor::pipeline_executor::WorkflowStatus` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
