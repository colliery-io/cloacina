---
title: "Task Deferral"
description: "How TaskHandle and defer_until manage concurrency slots during long-running waits"
weight: 32
---

# Task Deferral Architecture

## Problem Statement

Workflow tasks sometimes need to wait for external conditions -- a file appearing on disk, an API returning a ready status, a message arriving in a queue. A naive approach keeps the task's concurrency slot held for the entire wait, blocking other tasks from executing. With a limited concurrency pool (common in production to bound resource usage), this leads to underutilization: slots sit idle while the task sleeps.

`TaskHandle` and its `defer_until` method solve this by allowing a task to temporarily release its concurrency slot during a polling wait and reclaim one when the condition is met.

## Component Overview

### SlotToken

`SlotToken` (defined in `crates/cloacina/src/executor/slot_token.rs`) wraps a tokio `OwnedSemaphorePermit` and the `Arc<Semaphore>` it came from. It provides two operations:

- **`release()`** -- Drops the permit, immediately returning it to the semaphore. The token enters a "released" state.
- **`reclaim()`** -- Acquires a new permit from the semaphore. If all slots are occupied, this waits asynchronously until one becomes available.

The token tracks whether it currently holds a permit via an `Option<OwnedSemaphorePermit>`. Dropping a `SlotToken` that still holds a permit returns it to the semaphore automatically.

### TaskHandle

`TaskHandle` (defined in `crates/cloacina/src/executor/task_handle.rs`) wraps a `SlotToken` along with the task execution ID and an optional DAL reference for persisting sub-status updates. It exposes:

- **`defer_until(condition, poll_interval)`** -- The primary method. Releases the slot, polls the condition, reclaims when ready.
- **`is_slot_held()`** -- Reports whether the handle currently holds a concurrency slot.
- **`task_execution_id()`** -- Returns the ID of the current task execution.

### Task-Local Storage

The executor cannot pass a `TaskHandle` directly through the `Task::execute()` trait method (which has a fixed signature). Instead, the system uses tokio task-local storage:

1. The executor creates a `TaskHandle` and places it in task-local storage via `with_task_handle(handle, future)`.
2. The macro-generated `execute()` body calls `take_task_handle()` to retrieve the handle.
3. The handle is passed to the user's function as the second parameter.
4. After the user function returns, the macro-generated code calls `return_task_handle(handle)` to put it back.
5. `with_task_handle` extracts the returned handle so the executor can reclaim the slot token.

If the user function does not return the handle (e.g., it was dropped), `with_task_handle` returns `None` and the executor handles cleanup.

## The defer_until Lifecycle

The full sequence when a task calls `handle.defer_until(condition, interval)`:

```text
1. Task is executing, holding concurrency slot
2. Task calls handle.defer_until(condition, interval)
3. TaskHandle sets sub_status = "Deferred" in the database (if DAL present)
4. SlotToken.release() drops the semaphore permit
   -- slot is now available for other tasks --
5. Loop:
   a. Sleep for poll_interval
   b. Call condition().await
   c. If true, break; otherwise repeat
6. SlotToken.reclaim() acquires a new permit (may wait if at capacity)
7. TaskHandle sets sub_status = "Active" in the database (if DAL present)
8. defer_until returns Ok(())
9. Task continues executing with slot held
```

During step 5, the task's async future is parked in the tokio runtime. It consumes no concurrency slot and minimal memory. Other tasks can acquire the freed slot.

## Macro Integration

The `#[task]` proc macro detects handle parameters by examining the function signature at compile time:

```rust
// In crates/cloacina-macros/src/tasks.rs
let has_handle_param = fn_inputs.iter().any(|arg| {
    if let FnArg::Typed(pat_type) = arg {
        if let Pat::Ident(pat_ident) = &*pat_type.pat {
            let param_name = pat_ident.ident.to_string();
            return param_name == "handle" || param_name == "task_handle";
        }
    }
    false
});
```

When `has_handle_param` is true, the generated `execute()` body wraps the function call with handle take/return:

```rust
// Generated code (simplified)
let mut handle = ::cloacina::take_task_handle();
let result = user_function(&mut context, &mut handle).await;
::cloacina::return_task_handle(handle);
result
```

The generated `Task` implementation also returns `true` from `requires_handle()`, which the executor checks to decide whether to create and inject a `TaskHandle`.

## Executor Integration

In `ThreadTaskExecutor::execute()` (the dispatcher-based entry point), handle-aware tasks receive special treatment:

```rust
if task.requires_handle() {
    let slot_token = SlotToken::new(permit, self.semaphore.clone());
    let handle = TaskHandle::with_dal(slot_token, event.task_execution_id, self.dal.clone());
    let (result, _returned_handle) =
        with_task_handle(handle, self.execute_with_timeout(task, context)).await;
    // ...
}
```

For tasks that do not require a handle, the permit is held directly by the executor and released when the task completes.

## Sub-Status Tracking

When a DAL is available (database-backed execution), `defer_until` persists sub-status changes:

| Phase | sub_status value |
|-------|-----------------|
| Task starts executing | `"Active"` (set by executor) |
| `defer_until` called | `"Deferred"` |
| Condition met, slot reclaimed | `"Active"` |
| Task completes | Cleared by executor |

These sub-status values are visible through the task execution query APIs and can be used for monitoring dashboards.

## When to Use Deferral vs. Other Patterns

| Pattern | Use When | Slot Usage |
|---------|----------|------------|
| `defer_until` | Waiting minutes to hours for external condition | Released during wait |
| `tokio::time::sleep` | Brief delay (seconds) within task logic | Held during sleep |
| Separate trigger + workflow | Condition check decoupled from task execution | No slot held; trigger polls independently |
| Polling task with retries | Task should fail and retry if condition not met | Held per attempt, released between retries |

`defer_until` is the right choice when the wait is long enough that holding a slot is wasteful, but the task needs to continue executing (with context) once the condition is met.

## Python Bindings

The Python `TaskHandle` class (`PyTaskHandle` in `crates/cloacina/src/python/task.rs`) exposes the same `defer_until` interface. The condition is a Python callable returning `bool`, and `poll_interval_ms` is specified in milliseconds. See the [Python TaskHandle reference]({{< ref "/python-bindings/api-reference/task" >}}) for details.

## See Also

- [Tutorial 10 - Task Deferral]({{< ref "/tutorials/workflows/service/10-task-deferral" >}}) -- step-by-step walkthrough with the deferred-tasks example
- [Macro Reference]({{< ref "/reference/macros" >}}) -- `#[task]` attribute reference including handle detection
- [Task Execution Sequence]({{< ref "/explanation/workflows/task-execution-sequence" >}}) -- how a task moves from scheduling through execution
- [Dispatcher Architecture]({{< ref "/explanation/workflows/dispatcher-architecture" >}}) -- how the executor receives and processes task events
