---
title: "Task Deferral"
description: "How TaskHandle and defer_until manage concurrency slots during long-running waits"
weight: 32
---

# Task Deferral

## The Problem Task Deferral Solves

Traditional workflow executors allocate a concurrency slot to each running task and hold it for the task's entire lifetime. This works well for CPU-bound or short-lived I/O tasks, but falls apart when tasks must wait for external conditions: an API callback arriving, a file being uploaded by another system, a human clicking an approval button.

Consider an executor with 4 concurrency slots:

```text
Slot 1: [task A — waiting for webhook ...........................]
Slot 2: [task B — waiting for file upload ......................]
Slot 3: [task C — waiting for human approval ...................]
Slot 4: [task D — waiting for API response .....................]

New tasks E, F, G:  BLOCKED (no slots available)
```

All four slots are occupied by futures that are merely sleeping between poll attempts. Meanwhile, tasks E, F, and G -- which could do real work -- cannot execute. Throughput drops to zero for new work even though the machine is idle.

The fundamental tension is that the task holds state it needs after the wait completes (variables, context, partially-computed results), so you cannot simply kill it and restart it later. You need a mechanism that says: "park this future cheaply, release the concurrency slot, and wake me up when the condition is met."

That mechanism is `defer_until`.

```text
Slot 1: [task A — defer] [task E — working] [task A resumes]
Slot 2: [task B — defer] [task F — working] [task B resumes]
Slot 3: [task C — defer] [task G — working] [task C resumes]
Slot 4: [task D — defer] [task H — working] [task D resumes]

Deferred tasks park as cheap futures. Slots serve real work.
```

## How Deferral Works

### Opting In via TaskHandle

`TaskHandle` is an optional second parameter on `#[task]` functions. Tasks that need deferral request it by adding the parameter:

```rust
#[task(id = "wait_for_data", dependencies = [])]
pub async fn wait_for_data(
    context: &mut Context<serde_json::Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    // ...
}
```

The `#[task]` proc macro detects parameters named `handle` or `task_handle` at compile time. When present, the generated `Task` trait implementation returns `true` from `requires_handle()`, signaling the executor to create and inject a `TaskHandle` at runtime.

Tasks that do not need deferral omit the parameter entirely -- no behavioral change, no overhead.

### The defer_until Lifecycle

When a task calls `handle.defer_until(condition, interval)`, the following sequence occurs:

```text
1. Task is executing, holding concurrency slot
2. Task calls handle.defer_until(condition, interval)
3. TaskHandle sets sub_status = "Deferred" in the database (if DAL present)
4. SlotToken.release() drops the semaphore permit
   — slot is now available for other tasks —
5. Loop:
   a. Sleep for poll_interval
   b. Call condition().await
   c. If true, break; otherwise repeat
6. SlotToken.reclaim() acquires a new permit (may wait if at capacity)
7. TaskHandle sets sub_status = "Active" in the database (if DAL present)
8. defer_until returns Ok(())
9. Task continues executing with slot held
```

During step 5, the task's async future is parked in the tokio runtime. It wakes only to poll the condition, then parks again. It consumes no concurrency slot and minimal memory. Other tasks can acquire the freed slot and do real work.

### From the Scheduler's Perspective

The task transitions through three states:

| State | Slot Held | Future Status | Visible sub_status |
|-------|-----------|---------------|-------------------|
| Running | Yes | Actively polling | `"Active"` |
| Deferred | No | Parked (wakes on interval) | `"Deferred"` |
| Resuming | Yes (re-acquired) | Actively polling | `"Active"` |

The scheduler does not need to track deferred tasks specially. They are ordinary tokio futures that happen to be sleeping. The only coordination is through the semaphore.

## SlotToken Mechanics

The concurrency model is built on `SlotToken`, a wrapper around a tokio semaphore permit.

### Structure

```rust
pub struct SlotToken {
    permit: Option<OwnedSemaphorePermit>,
    semaphore: Arc<Semaphore>,
}
```

The `Option` enables the release/reclaim pattern. When the token holds `Some(permit)`, a concurrency slot is occupied. When `None`, the slot has been released.

### Operations

- **`release()`** -- Drops the inner permit via `self.permit.take()`. The semaphore immediately sees one more available permit. Returns `true` if a permit was released, `false` if already released.
- **`reclaim()`** -- Calls `self.semaphore.clone().acquire_owned().await` to get a new permit. If all slots are occupied, this awaits until one frees. Once acquired, the token is back in the held state.
- **`is_held()`** -- Returns `self.permit.is_some()`.
- **Drop behavior** -- Dropping a `SlotToken` that still holds a permit returns it to the semaphore automatically. No leaks even if a task panics.

### Why a Wrapper?

`SlotToken` decouples `TaskHandle` from tokio's specific permit types, enabling future extensions (weighted slots, cross-executor management, alternative backends) without changing the `TaskHandle` API.

## Concurrency Implications

### Slot Arithmetic

Given an executor with N concurrency slots:

- If K tasks defer, K slots become immediately available
- New tasks can fill those K slots while the deferred tasks wait
- Maximum active work at any instant = N (some mix of new tasks and resumed tasks)
- If all N tasks defer simultaneously, all N slots open for new work

### Fairness

When a deferred task's condition becomes true, it calls `reclaim()` which acquires a semaphore permit through the standard `acquire_owned()` path. This means resumed tasks compete for slots on equal footing with newly-scheduled tasks. There is no priority queue -- it is first-come, first-served at the semaphore level.

If all slots are occupied when a deferred task tries to resume, it waits. This is by design: the system guarantees it never exceeds N concurrent executing tasks.

### Throughput Model

System throughput is proportional to active (non-deferred) tasks:

```text
Effective throughput = min(N, pending_tasks) - deferred_tasks_trying_to_resume
```

In practice, deferred tasks resume quickly (reclaim is fast when a slot is free) so the steady-state is that deferred tasks are either waiting on their condition or executing, rarely blocked on reclaim.

## Task-Local Storage Mechanism

The executor cannot pass a `TaskHandle` through the `Task::execute()` trait method (which has a fixed signature for all tasks). Instead, the system uses tokio task-local storage:

```rust
tokio::task_local! {
    static TASK_HANDLE_SLOT: RefCell<Option<TaskHandle>>;
}
```

The protocol is:

1. The executor creates a `TaskHandle` and places it in task-local storage via `with_task_handle(handle, future)`.
2. The macro-generated `execute()` body calls `take_task_handle()` to retrieve the handle.
3. The handle is passed to the user's function as `&mut TaskHandle`.
4. After the user function returns, the macro-generated code calls `return_task_handle(handle)` to restore it.
5. `with_task_handle` extracts the returned handle so the executor can reclaim the slot token.

If the user function drops the handle (unlikely but possible), `with_task_handle` returns `None` and the executor handles cleanup gracefully -- the semaphore permit was already freed when the handle was dropped.

## Deferral in Practice

A typical deferred pipeline has two phases: a task calls `defer_until` with a condition closure
and a poll interval, releasing its slot while the condition is periodically checked. Once the
condition returns `true`, the slot is reclaimed and the task continues execution with all
local state intact. Downstream tasks see no difference -- they simply receive context written
by the deferred task as usual.

The condition closure has the signature `Fn() -> impl Future<Output = bool>` and is called
repeatedly at the poll interval until it returns `true`.

For a complete working example, see
[Tutorial 10: Task Deferral]({{< ref "/workflows/tutorials/service/10-task-deferral" >}}).

## When to Use Deferral

The following scenarios are the primary use cases for `defer_until`. Each represents a
situation where holding a concurrency slot during a long wait would waste executor capacity.

- **External API polling** -- A task submits a job to an external service and must wait for
  it to complete. Deferral frees the slot while periodically checking the job status endpoint,
  typically on a 10-30 second interval.

- **File watching** -- A task waits for another system to produce an output file (e.g., an
  ETL upload or a rendered report). A short poll interval (1-5 seconds) balances
  responsiveness against filesystem overhead.

- **Human-in-the-loop** -- A task pauses until a human performs an action such as clicking
  an approval button or updating a database flag. Poll intervals of 10-30 seconds are typical
  since human response times are measured in minutes.

- **Rate limiting / backoff** -- A task needs to wait before retrying a rate-limited operation.
  Unlike `tokio::time::sleep`, deferral releases the slot so other tasks can use it during
  the backoff window.

## Comparison with Alternatives

### vs `tokio::time::sleep`

| | `tokio::time::sleep` | `defer_until` |
|---|---|---|
| Concurrency slot | Held during sleep | Released during wait |
| Use case | Brief delays (seconds) | Long waits (minutes to hours) |
| Overhead | None | Semaphore release + reclaim |
| Other tasks | Blocked if slots full | Can use the freed slot |

If the wait is under a few seconds, `sleep` is simpler and the slot cost is negligible. For anything longer, `defer_until` prevents slot starvation.

### vs Splitting into Multiple Tasks

You could model "wait then process" as two separate tasks with a trigger mechanism:

```text
Option A (deferral):   [wait_and_process] — single task, defers in the middle
Option B (split):      [check_readiness] → [process_when_ready]
```

Deferral keeps task-local state intact. Variables declared before `defer_until` are still in scope afterward. With split tasks, you must serialize all intermediate state into the context or database and deserialize it in the second task. For complex state this is error-prone and verbose.

### vs External Message Queues

A common pattern is to park work externally (Redis, RabbitMQ, SQS) and re-enqueue when a condition is met. This introduces a broker dependency, state serialization, exactly-once delivery concerns, and operational complexity.

`defer_until` keeps everything in-process. The parked future is the state. No broker, no serialization, no delivery semantics. The tradeoff is that state is lost if the process crashes (see Limitations).

## Sub-Status Tracking

When a DAL (database access layer) is configured, `defer_until` persists sub-status transitions:

| Phase | sub_status value |
|-------|-----------------|
| Task starts executing | `"Active"` (set by executor) |
| `defer_until` called | `"Deferred"` |
| Condition met, slot reclaimed | `"Active"` |
| Task completes | Cleared by executor |

These values are visible through the task execution query APIs and can drive monitoring dashboards, alerting on tasks that remain deferred longer than expected.

## Limitations

### Memory Consumption

Deferred tasks still consume tokio runtime memory. Each parked future retains its stack frame (all local variables, the captured closure, the condition function). For most tasks this is kilobytes, but tasks with large buffers allocated before deferral will hold that memory for the entire deferred duration. If you have thousands of simultaneously deferred tasks, memory usage may become significant.

### No Persistence Across Restarts

If the runner process shuts down or crashes, all deferred tasks are cancelled. Their futures are dropped, and the work is lost. `defer_until` does not persist the task's execution state to disk. If you need deferral that survives process restarts, use the split-task pattern with a database-backed condition check, or implement checkpoint/restore at the application level.

### Poll Interval Latency

The condition is checked on a fixed interval. If the condition becomes true immediately after a poll, the task will not notice until the next poll cycle. This means:

- `poll_interval = 5s` implies up to 5 seconds of unnecessary latency
- `poll_interval = 100ms` is more responsive but wakes the future 10 times per second

Choose the interval based on your latency requirements. For human-in-the-loop workflows, 10-30 seconds is typical. For file watching, 1-5 seconds. For API callbacks where you control both sides, consider using a shared `AtomicBool` or `tokio::sync::Notify` that the callback sets immediately.

### Condition Function Constraints

The condition closure must be:

- **Cheap to evaluate**: It runs on a tokio worker thread. Expensive computation blocks the runtime.
- **Infallible** (returns `bool`, not `Result`): If your condition can fail, handle errors inside the closure and return `false` to keep polling.
- **Callable multiple times**: It is invoked every poll interval until it returns `true`.
- **`Send + 'static`**: Required by the async runtime for the spawned future.

## Python Bindings

The Python `TaskHandle` class (`PyTaskHandle` in `crates/cloacina/src/python/task.rs`) exposes the same `defer_until` interface. The condition is a Python callable returning `bool`, and `poll_interval_ms` is specified in milliseconds:

```python
@task(id="wait_for_approval")
async def wait_for_approval(context, handle):
    await handle.defer_until(
        condition=lambda: check_approval_status(context["job_id"]),
        poll_interval_ms=10000,
    )
    # Slot reclaimed, continue processing
```

See the [Python TaskHandle reference]({{< ref "/python/api-reference/task" >}}) for the full API.

## See Also

- [Tutorial 10 - Task Deferral]({{< ref "/workflows/tutorials/service/10-task-deferral" >}}) -- step-by-step walkthrough with the deferred-tasks example
- [Macro Reference]({{< ref "/workflows/reference/macros" >}}) -- `#[task]` attribute reference including handle detection
- [Task Execution Sequence]({{< ref "/workflows/explanation/task-execution-sequence" >}}) -- how a task moves from scheduling through execution
- [Dispatcher Architecture]({{< ref "/workflows/explanation/dispatcher-architecture" >}}) -- how the executor receives and processes task events
