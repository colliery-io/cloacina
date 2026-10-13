---
title: "cloacina::executor::thread_task_executor"
description: "Task Executor Module"
editUrl: false
sidebar:
  order: 124
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Task Executor Module

This module provides the core task execution functionality for the Cloacina workflow system.
The ThreadTaskExecutor implements the `TaskExecutor` trait for dispatcher-based execution.

The executor is responsible for:
- Executing tasks with proper timeout handling
- Managing task retries and error handling
- Maintaining task execution state
- Handling task dependencies and context management

**Dispatcher Integration**

ThreadTaskExecutor implements the `TaskExecutor` trait, allowing it to be registered
with a dispatcher to receive task events directly. The dispatcher routes `TaskReadyEvent`s
to the executor based on routing rules.

## Structs

### <span id="struct.ThreadTaskExecutor"></span><code style="font-size: 0.85em">struct ThreadTaskExecutor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

ThreadTaskExecutor is a thread-based implementation of task execution.

This executor runs tasks in the current thread/process and manages:
- Task execution with timeout handling
- Context management and dependency resolution
- Error handling and retry logic
- State persistence

The executor maintains its own instance ID for tracking and logging purposes
and uses a task registry to resolve task implementations.

**Dispatcher Integration**

ThreadTaskExecutor implements the `TaskExecutor` trait, allowing it to be
registered with a dispatcher to receive task events directly via the
`execute()` method.

#### Methods

##### <span id="ThreadTaskExecutor.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new(
    database: Database,
    task_registry: Arc<TaskRegistry>,
    config: ExecutorConfig,
) -> Self
```

Creates a new ThreadTaskExecutor instance.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database connection pool for task state persistence |
| `task_registry` | `Arc<TaskRegistry>` | Registry containing available task implementations |
| `config` | `ExecutorConfig` | Configuration parameters for executor behavior |

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

##### <span id="ThreadTaskExecutor.with_runtime_and_registry"></span><code>with_​runtime_​and_​registry</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_runtime_and_registry(
    database: Database,
    task_registry: Arc<TaskRegistry>,
    runtime: Arc<Runtime>,
    config: ExecutorConfig,
) -> Self
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
            secret_resolver: None,
        }
    }
```

</details>

##### <span id="ThreadTaskExecutor.with_runtime"></span><code>with_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self
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

##### <span id="ThreadTaskExecutor.with_secret_resolver"></span><code>with_​secret_​resolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_secret_resolver(
    mut self,
    resolver: Option<Arc<dyn cloacina_workflow::secret::SecretResolver>>,
) -> Self
```

Sets the secret resolution side channel (CLOACI-T-0858). Every task this executor runs then receives a context that can resolve secrets via `context.secret(...)`.

<details>
<summary>Source</summary>

```rust
    pub fn with_secret_resolver(
        mut self,
        resolver: Option<Arc<dyn cloacina_workflow::secret::SecretResolver>>,
    ) -> Self {
        self.secret_resolver = resolver;
        self
    }
```

</details>

##### <span id="ThreadTaskExecutor.semaphore"></span><code>semaphore</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn semaphore(&self) -> &Arc<Semaphore>
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
