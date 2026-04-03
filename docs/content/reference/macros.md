---
title: "Macro Reference"
description: "Complete reference for #[task], #[workflow], and #[trigger] attribute macros"
weight: 8
---

# Macro Reference

Cloacina provides three procedural attribute macros for defining tasks, workflows, and triggers. These macros generate trait implementations, registration code, and compile-time validation.

```rust
use cloacina::{task, workflow, Context, TaskError};
use serde_json::Value;
```

## #[task]

Applied to an `async fn` to define a task with retry policies, dependency declarations, trigger rules, and lifecycle callbacks.

### Syntax

```rust
#[task(
    id = "my_task",
    dependencies = ["dep_a", "dep_b"],
    retry_attempts = 3,
    retry_backoff = "exponential",
    retry_delay_ms = 1000,
    retry_max_delay_ms = 30000,
    retry_condition = "all",
    retry_jitter = true,
    trigger_rules = always,
    on_success = my_success_handler,
    on_failure = my_failure_handler,
)]
pub async fn my_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    Ok(())
}
```

### Attributes

| Attribute | Type | Required | Default | Description |
|---|---|---|---|---|
| `id` | string literal | yes | -- | Unique identifier for the task within its workflow. Used for dependency references. |
| `dependencies` | array of string literals | no | `[]` | List of task IDs that must complete before this task runs. |
| `retry_attempts` | integer | no | `3` | Maximum number of retry attempts on failure. |
| `retry_backoff` | string literal | no | `"exponential"` | Backoff strategy between retries. See [Backoff Strategies](#backoff-strategies). |
| `retry_delay_ms` | integer | no | `1000` | Initial delay between retries in milliseconds. |
| `retry_max_delay_ms` | integer | no | `30000` | Maximum delay between retries in milliseconds (caps exponential/linear growth). |
| `retry_condition` | string literal | no | `"all"` | When to retry. See [Retry Conditions](#retry-conditions). |
| `retry_jitter` | boolean | no | `true` | Whether to add random jitter to retry delays to avoid thundering herd. |
| `trigger_rules` | expression | no | `always` | Trigger rule expression controlling when the task should execute. See [Trigger Rules](#trigger-rules). |
| `on_success` | expression (path) | no | -- | Async callback on success. Signature: `async fn(&str, &Context<Value>) -> Result<(), E>` |
| `on_failure` | expression (path) | no | -- | Async callback on failure. Signature: `async fn(&str, &TaskError, &Context<Value>) -> Result<(), E>` |

### Backoff Strategies

| Value | Behavior |
|---|---|
| `"fixed"` | Constant delay of `retry_delay_ms` between every attempt |
| `"linear"` | Delay increases by `retry_delay_ms` each attempt (1x, 2x, 3x, ...) |
| `"exponential"` | Delay doubles each attempt (base 2, multiplier 1.0), capped at `retry_max_delay_ms` |

### Retry Conditions

| Value | Behavior |
|---|---|
| `"never"` | Never retry, regardless of error type |
| `"all"` | Retry on all errors |
| `"transient"` | Retry only on transient errors |
| `"pattern1,pattern2"` | Retry only when the error message matches one of the comma-separated patterns |

### Trigger Rules

Trigger rules are compile-time expressions that control conditional task execution:

| Expression | Description |
|---|---|
| `always` | Task always runs when dependencies are satisfied |
| `task_success("task_id")` | Run only if the named task succeeded |
| `task_failed("task_id")` | Run only if the named task failed |
| `task_skipped("task_id")` | Run only if the named task was skipped |
| `context_value("key", operator, value)` | Run based on a context value comparison |
| `all(cond1, cond2, ...)` | Run when all conditions are true |
| `any(cond1, cond2, ...)` | Run when any condition is true |
| `none(cond1, cond2, ...)` | Run when no conditions are true |

**Context value operators:** `equals`, `not_equals`, `greater_than`, `less_than`, `contains`, `not_contains`, `exists`, `not_exists`

**Example:**

```rust
#[task(
    id = "cleanup",
    dependencies = ["process"],
    trigger_rules = any(
        task_failed("process"),
        context_value("force_cleanup", equals, true)
    )
)]
pub async fn cleanup(context: &mut Context<Value>) -> Result<(), TaskError> {
    Ok(())
}
```

### Function Signature

The task function must:

1. Have a `context` parameter (or `_context`) of type `&mut Context<Value>`
2. Return `Result<(), TaskError>` (or any error type convertible to `TaskError`)
3. Be `async` (or synchronous -- both are supported)

An optional second parameter named `handle` or `task_handle` provides access to a `TaskHandle` for concurrency slot management. When the macro detects a parameter with one of these names, it sets `requires_handle() = true` on the generated `Task` trait implementation. The executor then creates a `TaskHandle` and injects it via task-local storage at runtime.

```rust
#[task(id = "wait_for_file")]
pub async fn wait_for_file(
    context: &mut Context<Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    handle.defer_until(
        || async { std::path::Path::new("/data/input.csv").exists() },
        Duration::from_secs(5),
    ).await.map_err(|e| TaskError::ExecutionFailed {
        message: format!("defer_until failed: {e}"),
        task_id: "wait_for_file".into(),
        timestamp: chrono::Utc::now(),
    })?;

    // File exists -- slot has been reclaimed, proceed with work
    Ok(())
}
```

`TaskHandle` methods:

| Method | Signature | Description |
|--------|-----------|-------------|
| `defer_until` | `async fn(&mut self, condition: F, poll_interval: Duration) -> Result<(), ExecutorError>` | Release the concurrency slot, poll `condition` at `poll_interval`, reclaim when `true` |
| `is_slot_held` | `fn(&self) -> bool` | Whether the handle currently holds a concurrency slot |
| `task_execution_id` | `fn(&self) -> UniversalUuid` | The task execution ID for this invocation |

See [Task Deferral Architecture]({{< ref "/explanation/task-deferral" >}}) for the full lifecycle and [Tutorial 10]({{< ref "/tutorials/10-task-deferral" >}}) for a walkthrough.

### Generated Code

The macro generates:

1. The original function (preserved for direct testing)
2. A `{PascalCase}Task` struct implementing `cloacina_workflow::Task`
3. A `{fn_name}_task()` constructor function
4. Static methods: `dependency_task_ids()`, `code_fingerprint()`, `create_retry_policy()`, `trigger_rules()`

## #[workflow]

Applied to a `pub mod` containing `#[task]` functions. Auto-discovers tasks, validates dependencies, and generates registration code.

### Syntax

```rust
#[workflow(name = "etl_pipeline", description = "Extract, transform, and load data")]
pub mod etl_pipeline {
    use super::*;

    #[task(id = "extract", dependencies = [])]
    pub async fn extract(context: &mut Context<Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "transform", dependencies = ["extract"])]
    pub async fn transform(context: &mut Context<Value>) -> Result<(), TaskError> {
        Ok(())
    }

    #[task(id = "load", dependencies = ["transform"])]
    pub async fn load(context: &mut Context<Value>) -> Result<(), TaskError> {
        Ok(())
    }
}
```

### Attributes

| Attribute | Type | Required | Default | Description |
|---|---|---|---|---|
| `name` | string literal | yes | -- | Unique workflow identifier. Used for registration and execution. |
| `description` | string literal | no | -- | Human-readable description of the workflow. |
| `tenant` | string literal | no | `"public"` | Tenant identifier for multi-tenant deployments. |
| `author` | string literal | no | -- | Author information. |

### Delivery Modes

The `#[workflow]` macro generates different code depending on compilation features:

| Mode | Feature Flag | Behavior |
|---|---|---|
| **Embedded** | (default) | Generates `#[ctor]` auto-registration. The workflow is registered in the global registry at program startup. |
| **Packaged** | `features = ["packaged"]` | Generates FFI exports for `.cloacina` packages. The workflow is loaded dynamically at runtime. |

### Compile-Time Validation

The workflow macro performs these validations at compile time:

- **Duplicate task IDs**: Two tasks with the same `id` in the same workflow produce a compile error
- **Cycle detection**: Circular dependencies (e.g., A depends on B, B depends on A) produce a compile error
- **Similar name suggestions**: If a dependency references a non-existent task, the compiler suggests similar task names

## #[trigger]

Applied to an `async fn` to define a trigger that fires a workflow on a schedule or condition. Two modes are available: custom poll triggers and cron triggers.

### Custom Poll Trigger

The function body contains the poll logic. Called at `poll_interval` frequency.

```rust
#[trigger(on = "inbox_processor", poll_interval = "5s")]
pub async fn check_inbox() -> Result<TriggerResult, TriggerError> {
    if has_new_messages().await? {
        Ok(TriggerResult::Fire(Some(context)))
    } else {
        Ok(TriggerResult::Skip)
    }
}
```

### Cron Trigger

The cron expression provides the schedule. The function body is ignored (consumed by the macro).

```rust
#[trigger(on = "daily_report", cron = "0 2 * * *", timezone = "America/New_York")]
pub async fn nightly_report() {}
```

### Attributes

| Attribute | Type | Required | Default | Description |
|---|---|---|---|---|
| `on` | string literal | yes | -- | Name of the workflow to trigger. |
| `poll_interval` | string literal | one of `poll_interval` or `cron` | -- | Poll frequency. Format: `100ms`, `5s`, `2m`, `1h`. |
| `cron` | string literal | one of `poll_interval` or `cron` | -- | Cron expression (5-7 fields). Validated at compile time. |
| `timezone` | string literal | no | `"UTC"` | IANA timezone for cron evaluation (e.g., `"America/New_York"`). Only applies to cron triggers. |
| `allow_concurrent` | boolean | no | `false` | Whether multiple trigger firings can overlap. |
| `name` | string literal | no | function name | Override the trigger name (used for registration and schedule records). |

**Validation rules:**

- Exactly one of `poll_interval` or `cron` must be specified (not both, not neither)
- Cron expressions must have 5-7 fields with valid characters (`0-9`, `,`, `-`, `*`, `/`)
- Poll interval must use a recognized unit suffix (`ms`, `s`, `m`, `h`)

### Poll Interval Format

| Suffix | Unit | Example |
|---|---|---|
| `ms` | Milliseconds | `100ms` |
| `s` | Seconds | `5s` |
| `m` | Minutes | `2m` |
| `h` | Hours | `1h` |

## Code Fingerprinting

Every `#[task]` function has a code fingerprint -- a 16-character hexadecimal hash computed at compile time. The fingerprint is used to detect when a task's implementation has changed.

### What Is Hashed

The fingerprint includes:

1. **Function parameter types** (excluding parameter names)
2. **Return type**
3. **Function body** (the complete token stream of the block)
4. **Async-ness** (whether the function is `async`)

### What Is NOT Hashed

- The function name
- Attributes (retry policy, dependencies, etc.)
- Comments and whitespace (after tokenization)
- Items outside the function body

### When Fingerprints Change

A new fingerprint is generated when:

- The function body changes (any logic change)
- The function signature changes (parameter types or return type)
- The function changes from sync to async or vice versa

A fingerprint does NOT change when:

- Only the task attributes change (e.g., updating `retry_attempts`)
- Only the function name changes
- Only comments change

### Usage

Fingerprints are available via the generated struct:

```rust
// Static method
let fp = MyTaskTask::code_fingerprint();

// Via Task trait
let task = my_task_task();
let fp = task.code_fingerprint(); // Returns Option<String>
```

## See Also

- [Cron Scheduling Architecture]({{< ref "/explanation/cron-scheduling" >}}) -- how cron triggers are evaluated
- [Errors Reference]({{< ref "errors" >}}) -- `TaskError`, `TriggerError` variants
- [cloacina-testing API]({{< ref "testing-crate" >}}) -- testing tasks without a database
