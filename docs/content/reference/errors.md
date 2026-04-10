---
title: "Error Reference"
description: "Complete reference for all error enums and variants in the Cloacina framework"
weight: 9
---

# Error Reference

This page documents every error type in the Cloacina framework. All error types implement the standard `Error` trait via `thiserror` and provide structured variants for pattern matching.

```rust
use cloacina::error::*;
use cloacina::cron_evaluator::CronError;
use cloacina::trigger::TriggerError;
use cloacina::executor::pipeline_executor::PipelineError;
```

## ContextError

Errors from context operations (inserting, reading, and serializing key-value data in the execution context).

**Defined in:** `cloacina::error::ContextError`

| Variant | Fields | Description |
|---|---|---|
| `Serialization` | `serde_json::Error` | Failed to serialize or deserialize a context value. |
| `KeyNotFound` | `String` | The requested key does not exist in the context. |
| `TypeMismatch` | `String` | The value for the key exists but cannot be deserialized to the requested type. |
| `KeyExists` | `String` | Attempted to insert a key that already exists (use `update` instead). |
| `Database` | `diesel::result::Error` | A database error occurred during a context persistence operation. |
| `ConnectionPool` | `String` | Failed to acquire a connection from the database pool. |
| `InvalidScope` | `String` | The execution scope is not valid for this context operation. |

**Common causes:**

- `KeyNotFound` -- Accessing a context key that was not set by a preceding task. Check task ordering and dependency declarations.
- `TypeMismatch` -- The value was inserted as one type and read as another. Ensure consistent `serde_json::Value` usage.
- `KeyExists` -- Using `insert` instead of `update` for an existing key.

## TaskError

Errors during task execution. This is the primary error type returned from `#[task]` functions.

**Defined in:** `cloacina_workflow::TaskError` (re-exported from `cloacina::error::TaskError`)

| Variant | Fields | Description |
|---|---|---|
| `ExecutionFailed` | `message: String`, `task_id: String`, `timestamp: DateTime<Utc>` | General task execution failure. The `message` contains the stringified error from the task function. |
| `DependencyNotSatisfied` | `dependency: String`, `task_id: String` | A required dependency has not completed successfully. |
| `Timeout` | `task_id: String`, `timeout_seconds: u64` | The task exceeded its configured timeout. |
| `ContextError` | `task_id: String`, `error: ContextError` | A context operation within the task failed. |
| `ValidationFailed` | `message: String` | Task validation (pre-execution checks) failed. |
| `Unknown` | `task_id: String`, `message: String` | An unclassified error occurred. |
| `ReadinessCheckFailed` | `task_id: String` | The task's readiness check failed (pre-conditions not met). |
| `TriggerRuleFailed` | `task_id: String` | The task's trigger rule evaluated to false, preventing execution. |

**Common causes:**

- `ExecutionFailed` -- Any `Err` returned from a `#[task]` function is wrapped in this variant. Check the `message` for the original error.
- `Timeout` -- Increase `task_timeout` in `DefaultRunnerConfig` or optimize the task logic.
- `DependencyNotSatisfied` -- The dependency graph is correct but a dependency task failed or was skipped.
- `TriggerRuleFailed` -- The task's `trigger_rules` attribute evaluated to false at runtime.

## ValidationError

Errors from workflow graph validation and dependency resolution.

**Defined in:** `cloacina::error::ValidationError`

| Variant | Fields | Description |
|---|---|---|
| `CyclicDependency` | `cycle: Vec<String>` | The dependency graph contains a cycle. Lists the task IDs forming the cycle. |
| `MissingDependency` | `task: String`, `dependency: String` | A task declares a dependency on a task ID that is not registered. |
| `MissingDependencyOld` | `task_id: String`, `dependency: String` | Legacy variant of `MissingDependency`. |
| `CircularDependency` | `cycle: String` | String representation of a circular dependency. |
| `DuplicateTaskId` | `String` | Two tasks in the same workflow have the same ID. |
| `EmptyWorkflow` | -- | The workflow contains no tasks. |
| `InvalidGraph` | `message: String` | General graph structure error. |
| `WorkflowNotFound` | `String` | The requested workflow is not registered. |
| `ExecutionFailed` | `message: String` | A pipeline execution failed during validation-triggered execution. |
| `TaskSchedulingFailed` | `task_id: String` | Failed to schedule a task for execution. |
| `InvalidTriggerRule` | `String` | A trigger rule expression could not be parsed. |
| `InvalidTaskName` | `String` | A task name does not conform to naming rules. |
| `ContextEvaluationFailed` | `key: String` | A context value referenced in a trigger rule could not be evaluated. |
| `RecoveryFailed` | `message: String` | A recovery operation failed. |
| `TaskRecoveryAbandoned` | `task_id: String`, `attempts: i32` | Recovery for this task was abandoned after exceeding the retry limit. |
| `PipelineRecoveryFailed` | `pipeline_id: Uuid` | Pipeline-level recovery failed. |
| `DatabaseConnection` | `message: String` | Could not connect to the database. |
| `DatabaseQuery` | `message: String` | A database query failed. |
| `Database` | `diesel::result::Error` | A diesel database error. |
| `ConnectionPool` | `String` | Failed to acquire a database pool connection. |
| `Context` | `ContextError` | A context error occurred during validation. |

**Common causes:**

- `CyclicDependency` / `CircularDependency` -- Review the `dependencies` arrays in `#[task]` attributes and remove the cycle.
- `MissingDependency` -- A task depends on an ID that does not exist. Check for typos.
- `DuplicateTaskId` -- Two tasks share the same `id`. Task IDs must be unique within a workflow.
- `WorkflowNotFound` -- The workflow was not registered. Ensure the `#[workflow]` module is compiled and linked.

## ExecutorError

Errors during task execution and pipeline management.

**Defined in:** `cloacina::error::ExecutorError`

| Variant | Fields | Description |
|---|---|---|
| `Database` | `diesel::result::Error` | Database error during execution tracking. |
| `ConnectionPool` | `String` | Failed to acquire a database pool connection. |
| `TaskNotFound` | `String` | The task ID is not in the global task registry. |
| `TaskExecution` | `TaskError` | A task returned an error during execution. |
| `Context` | `ContextError` | A context operation failed. |
| `TaskTimeout` | -- | A task exceeded its timeout (no task ID available at this level). |
| `Semaphore` | `tokio::sync::AcquireError` | Failed to acquire the concurrency semaphore. Indicates the runtime is shutting down. |
| `PipelineNotFound` | `Uuid` | No pipeline execution exists with this ID. |
| `Serialization` | `serde_json::Error` | Failed to serialize or deserialize execution state. |
| `InvalidScope` | `String` | Invalid execution scope. |
| `Validation` | `ValidationError` | A validation error surfaced during execution. |

**Common causes:**

- `TaskNotFound` -- The task is defined but not registered. In embedded mode, ensure the workflow module is linked. In packaged mode, ensure the package is loaded.
- `TaskTimeout` -- The task took longer than `task_timeout`. Increase the timeout or optimize the task.
- `Semaphore` -- This typically means the runner is shutting down. Not an error in normal operation.

## RegistrationError

Errors during task registration in the global registry.

**Defined in:** `cloacina::error::RegistrationError`

| Variant | Fields | Description |
|---|---|---|
| `DuplicateTaskId` | `id: String` | A task with this ID is already registered. |
| `InvalidTaskId` | `message: String` | The task ID contains invalid characters or is otherwise malformed. |
| `RegistrationFailed` | `message: String` | General registration failure. |

**Common causes:**

- `DuplicateTaskId` -- Two `#[task]` macros with the same `id` are compiled into the same binary. Task IDs must be globally unique.

## WorkflowError

Errors during workflow construction and management.

**Defined in:** `cloacina::error::WorkflowError`

| Variant | Fields | Description |
|---|---|---|
| `DuplicateTask` | `String` | A task with this ID already exists in the workflow. |
| `TaskNotFound` | `String` | Referenced task ID does not exist in the workflow. |
| `InvalidDependency` | `String` | A dependency reference is invalid. |
| `CyclicDependency` | `Vec<String>` | The dependency graph contains a cycle. |
| `UnreachableTask` | `String` | A task has no path from any root task and will never execute. |
| `RegistryError` | `String` | Error interacting with the workflow registry. |
| `TaskError` | `String` | A task-level error occurred during workflow operations. |
| `ValidationError` | `String` | General validation failure. |

**Common causes:**

- `UnreachableTask` -- A task has dependencies that form an island disconnected from the rest of the graph.
- `CyclicDependency` -- Same as `ValidationError::CyclicDependency`.

## SubgraphError

Errors when creating workflow subgraphs for partial execution or analysis.

**Defined in:** `cloacina::error::SubgraphError`

| Variant | Fields | Description |
|---|---|---|
| `TaskNotFound` | `String` | The requested task is not in the workflow graph. |
| `UnsupportedOperation` | `String` | The requested subgraph operation is not supported. |

## PipelineError

Top-level errors for pipeline execution.

**Defined in:** `cloacina::executor::pipeline_executor::PipelineError`

| Variant | Fields | Description |
|---|---|---|
| `DatabaseConnection` | `message: String` | Failed to connect to the database. |
| `WorkflowNotFound` | `workflow_name: String` | The workflow is not registered. |
| `ExecutionFailed` | `message: String` | The pipeline execution failed. |
| `Timeout` | `timeout_seconds: u64` | The pipeline exceeded its timeout. |
| `Validation` | `ValidationError` | A validation error occurred. |
| `TaskExecution` | `TaskError` | A task execution error bubbled up. |
| `Executor` | `ExecutorError` | An executor-level error occurred. |
| `Configuration` | `message: String` | Invalid runner configuration (e.g., schema name with invalid characters, missing database URL). |

**Common causes:**

- `WorkflowNotFound` -- Pass the correct workflow name to `runner.execute()`. Check registration.
- `Timeout` -- Increase `pipeline_timeout` in `DefaultRunnerConfig`.
- `Configuration` -- Check database URL and schema name formats.

## CronError

Errors from cron expression parsing and evaluation.

**Defined in:** `cloacina::cron_evaluator::CronError`

| Variant | Fields | Description |
|---|---|---|
| `InvalidExpression` | `String` | The cron expression format is invalid. |
| `InvalidTimezone` | `String` | The timezone string is not a valid IANA timezone. |
| `NoNextExecution` | -- | No next execution time could be found (end of time range). |
| `CronParsingError` | `String` | Low-level parsing error from the `croner` crate. |

**Common causes:**

- `InvalidExpression` -- Verify the cron expression has 5-7 fields (minute hour day month weekday [year] [seconds]).
- `InvalidTimezone` -- Use IANA timezone names (e.g., `"America/New_York"`, `"Europe/London"`, `"UTC"`). Not Windows timezone names.
- `NoNextExecution` -- Occurs when searching for executions in a time range that contains none.

## TriggerError

Errors from trigger operations.

**Defined in:** `cloacina::trigger::TriggerError`

| Variant | Fields | Description |
|---|---|---|
| `PollError` | `message: String` | The trigger's poll function returned an error. |
| `ContextError` | `ContextError` | Failed to create or manipulate the context for the triggered workflow. |
| `TriggerNotFound` | `name: String` | The named trigger is not registered. |
| `Database` | `diesel::result::Error` | Database error during trigger scheduling. |
| `ConnectionPool` | `String` | Failed to acquire a database pool connection. |

A separate `TriggerError` also exists in `cloacina_workflow` with a simpler structure:

| Variant | Fields | Description |
|---|---|---|
| `PollError` | `String` | Trigger poll error (used in `#[trigger]` function return types). |
| `ContextError` | `cloacina_workflow::ContextError` | Context creation error. |

**Common causes:**

- `PollError` -- The trigger's poll function failed. Check the error message for details.
- `TriggerNotFound` -- The trigger name in `package.toml` does not match any registered `#[trigger]` function.

## CheckpointError

Errors during task state checkpointing for recovery.

**Defined in:** `cloacina_workflow::CheckpointError` (re-exported from `cloacina::error::CheckpointError`)

| Variant | Fields | Description |
|---|---|---|
| `SaveFailed` | `task_id: String`, `message: String` | Failed to save a checkpoint. |
| `LoadFailed` | `task_id: String`, `message: String` | Failed to load a checkpoint. |
| `Serialization` | `serde_json::Error` | Checkpoint data could not be serialized or deserialized. |
| `StorageError` | `message: String` | The checkpoint storage backend reported an error. |
| `ValidationFailed` | `message: String` | The checkpoint data failed validation. |

## Error Conversion

The following automatic conversions are available via `From` implementations:

| From | To |
|---|---|
| `serde_json::Error` | `ContextError::Serialization` |
| `diesel::result::Error` | `ContextError::Database` |
| `diesel::result::Error` | `ValidationError::Database` |
| `diesel::result::Error` | `ExecutorError::Database` |
| `ContextError` | `TaskError::ContextError` |
| `ContextError` | `ValidationError::Context` |
| `TaskError` | `ExecutorError::TaskExecution` |
| `ValidationError` | `ExecutorError::Validation` |
| `ValidationError` | `PipelineError::Validation` |
| `TaskError` | `PipelineError::TaskExecution` |
| `ExecutorError` | `PipelineError::Executor` |
| `PoolError` | `ContextError::ConnectionPool` |
| `PoolError` | `ValidationError::ConnectionPool` |
| `PoolError` | `ExecutorError::ConnectionPool` |

## See Also

- [Macro Reference]({{< ref "macros" >}}) -- `retry_condition` and `trigger_rules` attributes that affect error handling
- [Configuration Reference]({{< ref "configuration" >}}) -- timeout and recovery settings
- [Cron Scheduling Architecture]({{< ref "/explanation/workflows/cron-scheduling" >}}) -- recovery behavior for cron errors
