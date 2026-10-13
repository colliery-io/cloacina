---
title: "cloacina::dispatcher::traits"
description: "Trait definitions for dispatcher and executor abstractions."
editUrl: false
sidebar:
  order: 108
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Trait definitions for dispatcher and executor abstractions.

This module defines the core traits that enable pluggable executor backends.
Implementors can create custom executors (Kubernetes, serverless, message queues)
that integrate seamlessly with the scheduler.

## Traits

### <span id="trait.Dispatcher"></span><code style="font-size: 0.85em">trait Dispatcher</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Dispatcher hands task-ready events to the configured executor.

The dispatcher acts as a thin layer between the scheduler and executors,
sending every task to a single server-configured executor key
(CLOACI-T-0640). Selecting which node/compute a task lands on is an
executor-internal concern.

**Implementation Requirements**

Implementors must ensure:
- Thread safety (Send + Sync)
- Proper executor registration and lookup

**Examples:**

```rust
use cloacina::dispatcher::{Dispatcher, TaskReadyEvent, TaskExecutor};

struct MyDispatcher {
    executors: HashMap<String, Arc<dyn TaskExecutor>>,
}

#[async_trait]
impl Dispatcher for MyDispatcher {
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError> {
        let executor = self.resolve_executor(&event.task_name);
        executor.execute(event).await?;
        Ok(())
    }
    // ...
}
```

#### Methods

##### <span id="Dispatcher.dispatch"></span><code>dispatch</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn dispatch(
    &self) -> Result<(,
    event: TaskReadyEvent) -> Result<(,
) -> Result<(), DispatchError>
```

Dispatch a ready task to the appropriate executor.

This method routes the task to an executor based on routing rules
and initiates execution. The actual execution may be synchronous
or asynchronous depending on the executor implementation.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `event` | `TaskReadyEvent` | The task ready event containing all execution context |

**Returns:**

* `Ok(())` - Task was successfully dispatched
* `Err(DispatchError)` - Dispatch failed (executor not found, no capacity, etc.)

<details>
<summary>Source</summary>

```rust
    async fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError>;
```

</details>

##### <span id="Dispatcher.register_executor"></span><code>register_​executor</code>

```rust
fn register_executor(&self, key: &str, executor: Arc<dyn TaskExecutor>)
```

Register an executor backend.

Executors are registered with a string key that is used by routing rules
to direct tasks to the appropriate backend.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `&str` | Unique identifier for this executor |
| `executor` | `Arc<dyn TaskExecutor>` | The executor implementation |

<details>
<summary>Source</summary>

```rust
    fn register_executor(&self, key: &str, executor: Arc<dyn TaskExecutor>);
```

</details>

##### <span id="Dispatcher.has_capacity"></span><code>has_​capacity</code>

```rust
fn has_capacity(&self) -> bool
```

Check if the dispatcher has any executor with available capacity.

This can be used by the scheduler to throttle task marking when
all executors are at capacity.

<details>
<summary>Source</summary>

```rust
    fn has_capacity(&self) -> bool;
```

</details>

##### <span id="Dispatcher.resolve_executor_key"></span><code>resolve_​executor_​key</code>

```rust
fn resolve_executor_key(&self, task_name: &str) -> String
```

Get the executor key that would handle a given task.

Useful for debugging and monitoring which executor will handle a task.

<details>
<summary>Source</summary>

```rust
    fn resolve_executor_key(&self, task_name: &str) -> String;
```

</details>

##### <span id="Dispatcher.has_executor"></span><code>has_​executor</code>

```rust
fn has_executor(&self, key: &str) -> bool
```

Returns `true` if an executor is registered under `key`.

Used to validate a configured `default_executor` key at startup so a
typo fails fast instead of silently sending all work to a nonexistent
executor (CLOACI-T-0640).

<details>
<summary>Source</summary>

```rust
    fn has_executor(&self, key: &str) -> bool;
```

</details>

### <span id="trait.TaskExecutor"></span><code style="font-size: 0.85em">trait TaskExecutor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Supertraits:** `Send + Sync`

Executor receives task-ready events and executes them.

This trait represents a task execution backend. Implementations can vary
from local thread pools to remote execution on Kubernetes, serverless
platforms, or message queues.

**Implementation Requirements**

Implementors must ensure:
- Thread safety (Send + Sync)
- Proper resource management (concurrency limits)
- Accurate capacity reporting
- Clean execution isolation

**Examples:**

```rust
use cloacina::dispatcher::{TaskExecutor, TaskReadyEvent, ExecutionResult};

struct KubernetesExecutor {
    client: kube::Client,
    namespace: String,
}

#[async_trait]
impl TaskExecutor for KubernetesExecutor {
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
        // Create a Kubernetes Job for the task
        let job = self.create_job(&event).await?;
        let result = self.wait_for_completion(job).await?;
        Ok(result)
    }
    // ...
}
```

#### Methods

##### <span id="TaskExecutor.execute"></span><code>execute</code>

<span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn execute(
    &self,
    event: TaskReadyEvent,
) -> Result<ExecutionResult, DispatchError>
```

Execute a task from a ready event.

The executor is responsible for:
- Running the task according to its paradigm
- Managing execution timeouts
- Capturing output context
- Reporting completion status

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `event` | `TaskReadyEvent` | The task ready event with execution context |

**Returns:**

* `Ok(ExecutionResult)` - Task completed (success or failure captured in result)
* `Err(DispatchError)` - Execution infrastructure failed

<details>
<summary>Source</summary>

```rust
    async fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>;
```

</details>

##### <span id="TaskExecutor.has_capacity"></span><code>has_​capacity</code>

```rust
fn has_capacity(&self) -> bool
```

Check if this executor can accept more work.

Used by the dispatcher to route work only to executors with available capacity.

<details>
<summary>Source</summary>

```rust
    fn has_capacity(&self) -> bool;
```

</details>

##### <span id="TaskExecutor.metrics"></span><code>metrics</code>

```rust
fn metrics(&self) -> ExecutorMetrics
```

Get executor metrics for monitoring.

Returns current statistics about the executor's state and performance.

<details>
<summary>Source</summary>

```rust
    fn metrics(&self) -> ExecutorMetrics;
```

</details>

##### <span id="TaskExecutor.name"></span><code>name</code>

```rust
fn name(&self) -> &str
```

Get a human-readable name for this executor type.

Used for logging and debugging purposes.

<details>
<summary>Source</summary>

```rust
    fn name(&self) -> &str;
```

</details>
