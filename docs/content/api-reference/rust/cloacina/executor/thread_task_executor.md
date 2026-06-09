# cloacina::executor::thread_task_executor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task Executor Module

This module provides the core task execution functionality for the Cloacina workflow system.
The ThreadTaskExecutor implements the `TaskExecutor` trait for dispatcher-based execution.
The executor is responsible for:
- Executing tasks with proper timeout handling
- Managing task retries and error handling
- Maintaining task execution state
- Handling task dependencies and context management

## Structs

### `cloacina::executor::thread_task_executor::ThreadTaskExecutor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


ThreadTaskExecutor is a thread-based implementation of task execution.

This executor runs tasks in the current thread/process and manages:
- Task execution with timeout handling
- Context management and dependency resolution
- Error handling and retry logic
- State persistence
The executor maintains its own instance ID for tracking and logging purposes
and uses a task registry to resolve task implementations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database connection pool for task state persistence |
| `dal` | `DAL` | Data Access Layer for database operations |
| `task_registry` | `Arc < TaskRegistry >` | Registry of available task implementations |
| `runtime` | `Arc < Runtime >` | Scoped runtime for task lookup (used in dispatcher execute path) |
| `instance_id` | `UniversalUuid` | Unique identifier for this executor instance |
| `config` | `ExecutorConfig` | Configuration parameters for executor behavior |
| `semaphore` | `Arc < Semaphore >` | Semaphore controlling concurrent task execution slots |
| `total_executed` | `Arc < AtomicU64 >` | Metrics: total tasks executed. `Arc` so clones — and the shared
[`crate::executor::TaskResultHandler`] (T-0630) — see the same counter. |
| `total_failed` | `Arc < AtomicU64 >` | Metrics: total tasks failed. |
| `result_handler` | `crate :: executor :: TaskResultHandler` | Shared post-execution handler (T-0630). Holds the same DAL + counters
+ runner_id as this executor; the upcoming `FleetExecutor` (T-0633)
will construct an analogous handler so thread and fleet paths share
one state-write sequence. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (database : Database , task_registry : Arc < TaskRegistry > , config : ExecutorConfig ,) -> Self
```

Creates a new ThreadTaskExecutor instance.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | Database connection pool for task state persistence |
| `task_registry` | `-` | Registry containing available task implementations |
| `config` | `-` | Configuration parameters for executor behavior |


**Returns:**

A new TaskExecutor instance with a randomly generated instance ID

<details>
<summary>Source</summary>

```rust
    pub fn new(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        config: ExecutorConfig,
    ) -> Self {
        Self::with_runtime_and_registry(database, task_registry, Arc::new(Runtime::new()), config)
    }
```

</details>



##### `with_runtime_and_registry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_runtime_and_registry (database : Database , task_registry : Arc < TaskRegistry > , runtime : Arc < Runtime > , config : ExecutorConfig ,) -> Self
```

Creates a new ThreadTaskExecutor with a specific runtime.

<details>
<summary>Source</summary>

```rust
    pub fn with_runtime_and_registry(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        runtime: Arc<Runtime>,
        config: ExecutorConfig,
    ) -> Self {
        let dal = DAL::new(database.clone());
        let max_concurrent = config.max_concurrent_tasks;
        let instance_id = UniversalUuid::new_v4();
        let total_executed = Arc::new(AtomicU64::new(0));
        let total_failed = Arc::new(AtomicU64::new(0));
        // `runner_id` for claim-guarded transitions only applies when claiming
        // is enabled; mirror the same logic the inline `claim_runner_id` had.
        let runner_id = if config.enable_claiming {
            Some(instance_id)
        } else {
            None
        };
        let result_handler = crate::executor::TaskResultHandler::new(
            dal.clone(),
            total_executed.clone(),
            total_failed.clone(),
            runner_id,
        );

        Self {
            database,
            dal,
            task_registry,
            runtime,
            instance_id,
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            total_executed,
            total_failed,
            result_handler,
        }
    }
```

</details>



##### `with_runtime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_runtime (mut self , runtime : Arc < Runtime >) -> Self
```

Sets the runtime for this executor, replacing the default.

<details>
<summary>Source</summary>

```rust
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = runtime;
        self
    }
```

</details>



##### `semaphore` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn semaphore (& self) -> & Arc < Semaphore >
```

Returns a reference to the concurrency semaphore.

Used by TaskHandle to release and reclaim concurrency slots
during deferred execution.

<details>
<summary>Source</summary>

```rust
    pub fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }
```

</details>



##### `build_task_context` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn build_task_context (& self , claimed_task : & ClaimedTask , dependencies : & [crate :: task :: TaskNamespace] ,) -> Result < Context < serde_json :: Value > , ExecutorError >
```

Builds the execution context for a task by loading its dependencies.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The task to build context for |
| `dependencies` | `-` | Task dependencies |


**Returns:**

Result containing the task's execution context

<details>
<summary>Source</summary>

```rust
    async fn build_task_context(
        &self,
        claimed_task: &ClaimedTask,
        dependencies: &[crate::task::TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // CLOACI-T-0633: delegate to the shared TaskContextBuilder so the
        // thread executor and the fleet executor resolve dependency context
        // identically (same drift-elimination pattern as TaskResultHandler).
        crate::executor::TaskContextBuilder::new(self.dal.clone())
            .build(claimed_task, dependencies)
            .await
    }
```

</details>



##### `merge_context_values` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn merge_context_values (existing : & serde_json :: Value , new : & serde_json :: Value ,) -> serde_json :: Value
```

Merges two context values using smart merging strategy.

For arrays: concatenates unique values maintaining order
For objects: merges recursively (latest wins for conflicting keys)
For primitives: latest wins

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `existing` | `-` | The existing value in the context |
| `new` | `-` | The new value from dependency context |


**Returns:**

The merged value CLOACI-T-0633: forwards to the shared [`crate::executor::TaskContextBuilder::merge_context_values`]. Test-only wrapper so the existing `merge_*` unit tests below keep exercising the canonical implementation through the thread executor's surface; the production path now goes through `TaskContextBuilder` directly.

<details>
<summary>Source</summary>

```rust
    fn merge_context_values(
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> serde_json::Value {
        crate::executor::TaskContextBuilder::merge_context_values(existing, new)
    }
```

</details>



##### `execute_with_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn execute_with_timeout (& self , task : & dyn Task , context : Context < serde_json :: Value > ,) -> Result < Context < serde_json :: Value > , ExecutorError >
```

Executes a task with timeout protection.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `-` | The task implementation to execute |
| `context` | `-` | The execution context |


**Returns:**

Result containing either the updated context or an error

<details>
<summary>Source</summary>

```rust
    async fn execute_with_timeout(
        &self,
        task: &dyn Task,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        match tokio::time::timeout(self.config.task_timeout, task.execute(context)).await {
            Ok(result) => result.map_err(ExecutorError::TaskExecution),
            Err(_) => Err(ExecutorError::TaskTimeout),
        }
    }
```

</details>



##### `execute_with_cancellation` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn execute_with_cancellation (& self , task : & dyn Task , context : Context < serde_json :: Value > , mut cancel_rx : tokio :: sync :: watch :: Receiver < bool > ,) -> Result < Context < serde_json :: Value > , ExecutorError >
```

Runs [`execute_with_timeout`] racing against a cancellation signal fed by the heartbeat loop. If the heartbeat detects `ClaimLost`, it flips the channel to `true`, the task future is dropped, and this returns [`ExecutorError::ClaimLost`]. This is the "Layer 1" cancellation of T-0487 — cooperative observation via `TaskHandle` is layered on top for tasks that need graceful cleanup.

<details>
<summary>Source</summary>

```rust
    async fn execute_with_cancellation(
        &self,
        task: &dyn Task,
        context: Context<serde_json::Value>,
        mut cancel_rx: tokio::sync::watch::Receiver<bool>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // Convert the watch signal into a bool *before* entering the select!
        // arm body so we don't hold a `watch::Ref` (which is !Send) across
        // the subsequent await.
        let wait_cancelled = async { cancel_rx.wait_for(|&v| v).await.is_ok() };
        // `biased;` gives the task arm priority. When the watch fires, both
        // arms can become ready on the same poll (the task's own
        // `TaskHandle::cancelled()` observes the same signal). Without
        // `biased`, `select!` picks randomly — which races Layer 2's
        // cooperative cleanup against Layer 1's drop. With `biased`, a task
        // that cooperatively handles cancellation runs to completion; a
        // task that ignores the signal still falls through to Layer 1
        // because its arm stays `Pending` while the cancel arm is ready.
        tokio::select! {
            biased;
            r = self.execute_with_timeout(task, context) => r,
            fired = wait_cancelled => {
                if fired {
                    Err(ExecutorError::ClaimLost)
                } else {
                    // Sender dropped without firing — the heartbeat was
                    // aborted via the success/failure path. Never resolve on
                    // this arm so the task future can complete normally.
                    std::future::pending().await
                }
            }
        }
    }
```

</details>





## Functions

### `cloacina::executor::thread_task_executor::failure_reason`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn failure_reason (err : & ExecutorError) -> & 'static str
```

Bounded reason value for `cloacina_tasks_total{status="failed", reason=...}`.

Cardinality is closed: the set of returned values is fixed here so label
explosion is impossible. Currently used only by the test that pins the
label set; production code path that emitted these labels was removed
in T-0563. Kept as a behavioral spec test.

<details>
<summary>Source</summary>

```rust
fn failure_reason(err: &ExecutorError) -> &'static str {
    match err {
        ExecutorError::TaskTimeout => "timeout",
        ExecutorError::TaskExecution(_) => "task_error",
        ExecutorError::Validation(_) => "validation_failed",
        ExecutorError::ClaimLost => "claim_lost",
        ExecutorError::Database(_)
        | ExecutorError::ConnectionPool(_)
        | ExecutorError::Context(_) => "infrastructure",
        // COR-11: ContextLoadFailed now reports as its own bounded
        // reason value so operators can distinguish "task failed
        // because we couldn't load its dependency context" from
        // generic infrastructure issues.
        ExecutorError::ContextLoadFailed(_) => "context_load_failed",
        ExecutorError::TaskNotFound(_) | ExecutorError::WorkflowExecutionNotFound(_) => {
            "task_not_found"
        }
        ExecutorError::Serialization(_)
        | ExecutorError::InvalidScope(_)
        | ExecutorError::Semaphore(_) => "unknown",
    }
}
```

</details>
