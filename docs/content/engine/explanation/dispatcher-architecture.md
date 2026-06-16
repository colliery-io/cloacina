---
title: "Dispatcher Architecture"
description: "Understanding the dispatcher pattern for pluggable task execution backends"
weight: 35
aliases:
  - "/workflows/explanation/dispatcher-architecture/"

---

## Overview

Cloacina uses a dispatcher architecture to decouple task scheduling from task execution. This enables pluggable execution backends - you can implement custom executors for Kubernetes jobs, serverless functions, remote workers, or any other execution environment.

## Architecture

```mermaid
flowchart TB
    subgraph Scheduler["Task Scheduler"]
        SM[State Manager]
        SL[Scheduler Loop]
    end

    subgraph Dispatcher["Dispatcher"]
        D[DefaultDispatcher]
    end

    subgraph Executors["Executors"]
        THREAD[ThreadTaskExecutor]
        FLEET[FleetExecutor]
        CUSTOM[CustomExecutor]
    end

    SM -->|TaskReadyEvent| D
    D -->|dispatch to configured key| THREAD
    D -->|dispatch to configured key| FLEET
    D -->|dispatch to configured key| CUSTOM

    THREAD -->|ExecutionResult| D
    FLEET -->|ExecutionResult| D
    CUSTOM -->|ExecutionResult| D

    D -->|state update| DB[(Database)]
```

The dispatcher does not match tasks against patterns. Every task is dispatched to a single configured executor — the **default executor** key, a server-level deployment knob (default `default`; e.g. `fleet` when the execution-agent fleet is deployed). Choosing which node or compute a task lands on is an executor-internal concern, not a scheduler/dispatcher one.

### Key Components

| Component | Purpose |
|-----------|---------|
| **TaskReadyEvent** | Event emitted when a task becomes ready for execution |
| **Dispatcher** | Sends every event to the single configured default executor |
| **TaskExecutor** | Trait implemented by execution backends |
| **ExecutionResult** | Outcome of task execution (success, failure, retry) |

## The Dispatcher Trait

The `Dispatcher` trait defines the interface for handing task events to the configured executor:

```rust
pub trait Dispatcher: Send + Sync {
    /// Dispatch a task-ready event to the configured default executor.
    fn dispatch(&self, event: TaskReadyEvent) -> Result<(), DispatchError>;

    /// Register an executor with a given key.
    fn register_executor(&self, key: &str, executor: Arc<dyn TaskExecutor>);

    /// Check if the configured executor has capacity.
    fn has_capacity(&self) -> bool;
}
```

There is no per-task routing decision: `dispatch` always forwards to the executor registered under the configured default-executor key. The configured key is hard-matched against the registered executors at server startup, so an unknown key fails fast rather than falling back silently.

## The TaskExecutor Trait

To implement a custom executor, implement the `TaskExecutor` trait:

```rust
pub trait TaskExecutor: Send + Sync {
    /// Execute a task and return the result.
    fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError>;

    /// Check if this executor has capacity for more tasks.
    fn has_capacity(&self) -> bool;

    /// Get current executor metrics.
    fn metrics(&self) -> ExecutorMetrics;

    /// Get the executor's name for logging/debugging.
    fn name(&self) -> &str;
}
```

## TaskReadyEvent

When the scheduler determines a task is ready, it emits a `TaskReadyEvent`:

```rust
pub struct TaskReadyEvent {
    /// The pipeline execution this task belongs to
    pub pipeline_execution_id: UniversalUuid,
    /// Unique identifier for this task execution record
    pub task_execution_id: UniversalUuid,
    /// Full task namespace (e.g., "public::embedded::workflow::task_name")
    pub task_namespace: String,
    /// Current attempt number (1-based)
    pub attempt: i32,
    /// Maximum allowed attempts
    pub max_attempts: i32,
}
```

Note: The event does not include context data. Executors should load context from the database at execution time to ensure they have the latest state.

## Implementing a Custom Executor

Here's a template for implementing a custom executor:

```rust
use cloacina::dispatcher::{
    TaskExecutor, TaskReadyEvent, ExecutionResult, ExecutionStatus,
    ExecutorMetrics, DispatchError,
};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MyCustomExecutor {
    name: String,
    max_concurrent: usize,
    active_tasks: AtomicU64,
    total_executed: AtomicU64,
    total_failed: AtomicU64,
    // Your custom fields here (client connections, config, etc.)
}

impl MyCustomExecutor {
    pub fn new(name: &str, max_concurrent: usize) -> Self {
        Self {
            name: name.to_string(),
            max_concurrent,
            active_tasks: AtomicU64::new(0),
            total_executed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }
}

impl TaskExecutor for MyCustomExecutor {
    fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
        self.active_tasks.fetch_add(1, Ordering::SeqCst);

        // 1. Load context from database using event.pipeline_execution_id
        // 2. Resolve the task implementation from registry
        // 3. Execute the task in your custom environment
        // 4. Handle success/failure and update database

        let result = match self.run_task(&event) {
            Ok(()) => {
                self.total_executed.fetch_add(1, Ordering::SeqCst);
                ExecutionResult {
                    task_execution_id: event.task_execution_id,
                    status: ExecutionStatus::Completed,
                    error_message: None,
                    should_retry: false,
                }
            }
            Err(e) => {
                self.total_failed.fetch_add(1, Ordering::SeqCst);
                let should_retry = event.attempt < event.max_attempts;
                ExecutionResult {
                    task_execution_id: event.task_execution_id,
                    status: if should_retry {
                        ExecutionStatus::Retry
                    } else {
                        ExecutionStatus::Failed
                    },
                    error_message: Some(e.to_string()),
                    should_retry,
                }
            }
        };

        self.active_tasks.fetch_sub(1, Ordering::SeqCst);
        Ok(result)
    }

    fn has_capacity(&self) -> bool {
        self.active_tasks.load(Ordering::SeqCst) < self.max_concurrent as u64
    }

    fn metrics(&self) -> ExecutorMetrics {
        ExecutorMetrics {
            active_tasks: self.active_tasks.load(Ordering::SeqCst),
            total_executed: self.total_executed.load(Ordering::SeqCst),
            total_failed: self.total_failed.load(Ordering::SeqCst),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

## Selecting the Executor

Execution topology is a single server-level deployment knob: the **default executor** key. Every task is dispatched to that one executor; there is no per-task matching, no glob rules, and no precedence chain.

The key defaults to `default` (the in-process `ThreadTaskExecutor`). Set it to another registered key — e.g. `fleet` for the [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}) — to send all work there. The preferred surface is `[server].default_executor` in `~/.cloacina/config.toml`:

```toml
[server]
default_executor = "fleet"
```

For ad-hoc or direct runs, override it on the binary or via the environment (precedence: explicit CLI/env > `config.toml` > built-in `default`):

```bash
cloacina-server --default-executor fleet
CLOACINA_DEFAULT_EXECUTOR=fleet cloacina-server
```

The configured key is hard-matched against the registered executors at startup. A typo or an unknown key fails fast with an error listing the valid keys (e.g. `default`, plus `fleet` when the fleet is deployed) — there is no silent fallback.

> **Why no per-task routing?** Choosing which node or compute a task runs on is an executor-internal concern (a future capability — executors will route work to specific nodes/capabilities). The scheduler and dispatcher do not make that decision per task, so routing was removed from the scheduler rather than merely demoted.

## Registering Custom Executors

Register executors with the dispatcher before starting the runner. Whichever key matches the configured default executor receives all dispatched tasks:

```rust
use cloacina::runner::DefaultRunner;

// The default ThreadTaskExecutor is registered automatically as "default".
// Register additional executors under their own keys; the configured
// default-executor key selects which one receives work.
let runner = DefaultRunner::builder()
    .database_url("postgresql://localhost/cloacina")
    .build()
    .await?;
```

## Execution Flow

1. **Scheduler** evaluates task dependencies and trigger rules
2. **State Manager** marks task as Ready and emits `TaskReadyEvent`
3. **Dispatcher** receives event and hands it to the configured default executor
4. **Executor** receives event, executes task, returns `ExecutionResult`
5. **Dispatcher** processes result, updates database state

```mermaid
sequenceDiagram
    participant S as Scheduler
    participant D as Dispatcher
    participant E as Executor
    participant DB as Database

    S->>DB: Mark task Ready
    S->>D: TaskReadyEvent
    D->>E: execute(event) via configured default executor
    E->>DB: Load context
    E->>E: Run task
    E->>DB: Save context
    E-->>D: ExecutionResult
    D->>DB: Update task state
```

## Error Handling

The dispatcher handles errors at multiple levels:

| Error Type | Handling |
|------------|----------|
| **DispatchError::ExecutorNotFound** | No executor registered under the configured default-executor key. Normally pre-empted at startup: the boot-time hard-match rejects an unknown default-executor key before any task is dispatched |
| **DispatchError::NoCapacity** | All executors at capacity (task stays Ready) |
| **DispatchError::ExecutionFailed** | Task execution failed (retry or fail based on policy) |

## Best Practices

1. **Idempotency**: Design tasks to be idempotent since they may be retried
2. **Context Loading**: Always load fresh context at execution time
3. **Metrics**: Track active tasks, success/failure counts for observability
4. **Capacity**: Implement `has_capacity()` accurately to prevent overload
5. **Timeouts**: Implement execution timeouts in your executor
6. **Error Messages**: Return descriptive error messages for debugging

## Example: Kubernetes Job Executor

A K8s executor might:

1. Create a Kubernetes Job spec from the task configuration
2. Submit the job to the cluster
3. Wait for completion (or timeout)
4. Retrieve logs and results
5. Return appropriate `ExecutionResult`

```rust
impl TaskExecutor for K8sExecutor {
    fn execute(&self, event: TaskReadyEvent) -> Result<ExecutionResult, DispatchError> {
        // Create job spec
        let job = self.create_job_spec(&event)?;

        // Submit to K8s
        let job_name = self.k8s_client.create_job(job).await?;

        // Wait for completion
        match self.wait_for_job(&job_name, self.timeout).await {
            Ok(()) => Ok(ExecutionResult::success(event.task_execution_id)),
            Err(e) => Ok(ExecutionResult::failed(
                event.task_execution_id,
                e.to_string(),
                event.attempt < event.max_attempts,
            )),
        }
    }
    // ...
}
```

## See Also

- [Task Execution Sequence]({{< relref "task-execution-sequence.md" >}}) - Detailed task lifecycle
- [Guaranteed Execution Architecture]({{< relref "guaranteed-execution-architecture.md" >}}) - Reliability guarantees
- [Performance Characteristics]({{< relref "performance-characteristics.md" >}}) - Tuning executors
