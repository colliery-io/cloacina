---
title: "10 - Task Handles"
description: "Release concurrency slots while waiting for external conditions with TaskHandle and defer_until"
weight: 20
reviewer: "dstorey"
review_date: "2025-03-13"
---

## Overview

When a workflow task needs to wait for an external condition -- a file appearing on disk, an API returning a ready status, a message landing in a queue -- it should not hold a concurrency slot the entire time. A held slot prevents other tasks from executing even though the waiting task is doing no real work.

`TaskHandle` solves this problem. It is an optional second parameter that the `#[task]` macro can inject into your task function. Through its `defer_until` method, a task can **release** its concurrency slot, **poll** a user-defined condition at a fixed interval, and **reclaim** a slot once the condition is satisfied. The task's async future stays parked in the tokio runtime during the deferred window, consuming minimal resources.

## Prerequisites

Before starting this tutorial, you should:

- Have completed [Tutorial 03 - Complex Workflows]({{< ref "/tutorials/03-complex-workflows/" >}})
- Be familiar with async/await patterns in Rust
- Understand closures that return futures (`Fn() -> impl Future<Output = bool>`)
- Have the Rust toolchain installed (rustc, cargo)

## Time Estimate

20-25 minutes

## What You'll Learn

- How to accept a `TaskHandle` in a task function
- How `defer_until` releases and reclaims concurrency slots
- How the `#[task]` macro detects the handle parameter by name
- How `SlotToken` manages semaphore permits under the hood
- How to build and run a workflow that uses deferred execution

## Key Concepts

### TaskHandle

`TaskHandle` is a per-execution control object created by the executor. It wraps a `SlotToken` (which itself wraps a tokio semaphore permit) and exposes methods for concurrency slot management. Tasks receive it as a mutable reference when they declare a second parameter named `handle` or `task_handle`.

### defer_until

The primary method on `TaskHandle`:

```rust
pub async fn defer_until<F, Fut>(
    &mut self,
    condition: F,
    poll_interval: Duration,
) -> Result<(), ExecutorError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = bool>,
```

- **`condition`** -- A closure that returns a future resolving to `bool`. Return `true` when the task should resume.
- **`poll_interval`** -- A `Duration` controlling how often the condition is checked.

The lifecycle of a single `defer_until` call:

1. The concurrency slot is **released** (semaphore permit dropped).
2. The condition is **polled** every `poll_interval`.
3. When the condition returns `true`, a slot is **reclaimed** (new permit acquired -- this may wait if all slots are occupied).
4. Control returns to the task function with the slot re-held.

### SlotToken

`SlotToken` is the internal abstraction that wraps a `tokio::sync::OwnedSemaphorePermit`. It provides `release()` and `reclaim()` methods that `TaskHandle::defer_until` calls on your behalf. You never interact with `SlotToken` directly -- it is `pub(crate)`.

### Macro Detection

The `#[task]` macro inspects parameter names at compile time. If the second parameter is named `handle` or `task_handle`, the macro sets `requires_handle()` to return `true` in the generated `Task` impl and emits code to retrieve the handle from task-local storage before calling your function.

## Setting Up Your Project

Create a new Rust project:

```bash
mkdir -p my-cloacina-projects
cd my-cloacina-projects
cargo new deferred-tutorial
cd deferred-tutorial
```

Add the required dependencies to your `Cargo.toml`:

```toml
[package]
name = "deferred-tutorial"
version = "0.1.0"
edition = "2021"

[dependencies]
cloacina = { path = "../../cloacina" }
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
ctor = "0.2"
chrono = "0.4"
```

{{< hint type=warning title=Important >}}
Normally you'd use `cloacina = "0.1.0"` in Cargo.toml. For these tutorials, we're using path dependencies to vendor code locally.

The path must be relative to your project. Adjust accordingly:
- Next to Cloacina: `path = "../cloacina"`
- In a subdirectory: `path = "../../../cloacina"`

Note: Use `version = "0.1.0"` when available on crates.io.
{{< /hint >}}

## Step 1: Define the Deferred Task

Create `src/main.rs` and start with the imports and the task that uses `defer_until`:

```rust
use cloacina::{task, workflow, Context, TaskError, TaskHandle};
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
```

Now define `wait_for_data`. This task simulates polling an external data source. It accepts `handle: &mut TaskHandle` as its second parameter, which tells the macro to wire up task-handle support:

```rust
#[task(id = "wait_for_data", dependencies = [])]
async fn wait_for_data(
    context: &mut Context<serde_json::Value>,
    handle: &mut TaskHandle,
) -> Result<(), TaskError> {
    info!("wait_for_data: Starting — will defer until data is ready");

    // Simulate an external readiness check.
    // In production this would call an API, check a file, poll a queue, etc.
    let poll_count = Arc::new(AtomicUsize::new(0));
    let pc = poll_count.clone();

    handle
        .defer_until(
            move || {
                let pc = pc.clone();
                async move {
                    let n = pc.fetch_add(1, Ordering::SeqCst);
                    info!("wait_for_data: polling external source (attempt {})", n + 1);
                    // Data becomes ready after 3 polls (indices 0, 1, 2)
                    n >= 2
                }
            },
            Duration::from_millis(500),
        )
        .await
        .map_err(|e| TaskError::ExecutionFailed {
            message: format!("defer_until failed: {e}"),
            task_id: "wait_for_data".into(),
            timestamp: chrono::Utc::now(),
        })?;

    info!(
        "wait_for_data: Data is ready after {} polls — slot reclaimed",
        poll_count.load(Ordering::SeqCst)
    );

    // Write the "received" data into context for downstream tasks
    context.insert("external_data", json!({"status": "ready", "records": 42}))?;
    Ok(())
}
```

There are a few things to note in this function:

- **The parameter name matters.** The macro checks for `handle` or `task_handle`. Any other name will not trigger handle injection.
- **The condition closure must be `Fn() -> Fut`**, not `FnOnce`. It is called repeatedly until it returns `true`. Clone any captured `Arc` values inside the closure body.
- **`defer_until` returns `Result<(), ExecutorError>`**, which you map into your task's `TaskError`.

{{< hint type=info title="Polling semantics" >}}
`defer_until` sleeps for `poll_interval` *before* each poll. This means the first check happens after one interval has elapsed, not immediately. If you need an immediate check, test the condition before calling `defer_until`.
{{< /hint >}}

## Step 2: Define the Downstream Task

The `process_data` task runs after `wait_for_data` completes. It does not need a `TaskHandle` because it performs no deferred work:

```rust
#[task(id = "process_data", dependencies = ["wait_for_data"])]
async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let data = context
        .get("external_data")
        .ok_or_else(|| TaskError::ExecutionFailed {
            message: "external_data not found in context".into(),
            task_id: "process_data".into(),
            timestamp: chrono::Utc::now(),
        })?
        .clone();

    info!("process_data: Processing external data: {}", data);

    let records = data.get("records").and_then(|v| v.as_u64()).unwrap_or(0);

    context.insert("processed_count", json!(records))?;
    context.insert("processing_complete", json!(true))?;

    info!("process_data: Processed {} records", records);
    Ok(())
}
```

## Step 3: Wire Up the Workflow and Runner

Add the `main` function to register the workflow and execute it:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("deferred_tutorial=info,cloacina=info")
        .init();

    info!("=== Deferred Tasks Tutorial ===");

    // Initialize runner with SQLite
    let runner = DefaultRunner::with_config(
        "sqlite://deferred-tutorial.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000",
        DefaultRunnerConfig::default(),
    )
    .await?;

    // Register the workflow
    let _workflow = workflow! {
        name: "deferred_pipeline",
        description: "Pipeline demonstrating deferred task execution",
        tasks: [wait_for_data, process_data]
    };

    info!("Executing deferred_pipeline...");
    let result = runner.execute("deferred_pipeline", Context::new()).await?;

    info!("=== Pipeline Complete ===");
    info!("Status: {:?}", result.status);

    if let Some(count) = result.final_context.get("processed_count") {
        info!("Processed {} records", count);
    }
    if let Some(complete) = result.final_context.get("processing_complete") {
        info!("Processing complete: {}", complete);
    }

    runner.shutdown().await?;
    Ok(())
}
```

## Running the Tutorial

### Option 1: Using Angreal (Recommended)

If you have the Cloacina repository checked out, you can run the deferred-tasks example directly:

```bash
# From the Cloacina repository root
cargo run --example deferred-tasks
```

Or if an angreal demo is configured:

```bash
angreal demos deferred-tasks
```

### Option 2: Manual Setup

From your project directory:

```bash
cargo run
```

You should see output similar to:

```text
INFO deferred_tutorial: === Deferred Tasks Tutorial ===
INFO deferred_tutorial: Executing deferred_pipeline...
INFO deferred_tutorial: wait_for_data: Starting — will defer until data is ready
INFO deferred_tutorial: wait_for_data: polling external source (attempt 1)
INFO deferred_tutorial: wait_for_data: polling external source (attempt 2)
INFO deferred_tutorial: wait_for_data: polling external source (attempt 3)
INFO deferred_tutorial: wait_for_data: Data is ready after 3 polls — slot reclaimed
INFO deferred_tutorial: process_data: Processing external data: {"records":42,"status":"ready"}
INFO deferred_tutorial: process_data: Processed 42 records
INFO deferred_tutorial: === Pipeline Complete ===
INFO deferred_tutorial: Status: Completed
INFO deferred_tutorial: Processed 42 records
INFO deferred_tutorial: Processing complete: true
```

Notice the three polling attempts before the data becomes ready. During those 1.5 seconds of polling, the concurrency slot was free for other tasks to use.

## What Happens Under the Hood

Here is the full lifecycle of a task-handle-enabled execution:

1. **Executor acquires a semaphore permit** and wraps it in a `SlotToken`, then creates a `TaskHandle` containing that token.
2. **Executor calls `with_task_handle(handle, task.execute(context))`**, which stores the handle in a `tokio::task_local` slot and runs the task's `execute` future.
3. **Macro-generated `execute()` calls `take_task_handle()`** to retrieve the handle from task-local storage and passes it as `&mut TaskHandle` to your function.
4. **Your function calls `handle.defer_until(condition, interval)`**:
   - `SlotToken::release()` drops the semaphore permit, freeing the slot.
   - The condition is polled every `interval`. The task's async future is parked between polls.
   - When the condition returns `true`, `SlotToken::reclaim()` acquires a new permit (waiting if none are available).
5. **Your function returns.** The macro-generated code calls `return_task_handle(handle)` to put the handle back into task-local storage.
6. **`with_task_handle` completes**, returning both the task result and the handle. The executor reclaims the `SlotToken` from the handle and the permit is returned to the semaphore when the token is dropped.

{{< hint type=info title="Why task-local storage?" >}}
The `Task` trait's `execute` method has a fixed signature that does not include `TaskHandle`. Task-local storage bridges this gap: the executor sets the handle before calling `execute`, and the macro-generated code retrieves it. This keeps the trait backward-compatible while allowing opt-in handle support.
{{< /hint >}}

## The requires_handle Flag

When the `#[task]` macro detects a handle parameter, the generated `Task` implementation includes:

```rust
fn requires_handle(&self) -> bool {
    true
}
```

The executor checks this flag before task execution. If `requires_handle()` returns `true`, the executor creates a `TaskHandle` and uses `with_task_handle` to make it available. If `false` (the default), no handle is created and the task runs with the standard code path.

This means there is zero overhead for tasks that do not use handles.

## Best Practices

### Keep Condition Checks Lightweight

The condition closure runs inside the tokio runtime on a worker thread. Avoid blocking I/O or heavy computation:

```rust
// Good: quick async check
handle.defer_until(
    || async { reqwest::get(url).await.map(|r| r.status().is_success()).unwrap_or(false) },
    Duration::from_secs(5),
).await?;

// Bad: CPU-heavy work inside the condition
handle.defer_until(
    || async { compute_hash_of_large_file().await == expected },
    Duration::from_secs(1),
).await?;
```

### Choose an Appropriate Poll Interval

The interval controls the tradeoff between responsiveness and resource usage:

- **External APIs**: 5-30 seconds (respect rate limits)
- **File system checks**: 1-5 seconds
- **In-memory flags** (testing): 10-100 milliseconds

### Handle the Error from defer_until

`defer_until` returns `Result<(), ExecutorError>`. The error case occurs when the semaphore is closed (executor shutting down). Always map this into your task's error type:

```rust
handle.defer_until(condition, interval)
    .await
    .map_err(|e| TaskError::ExecutionFailed {
        message: format!("defer_until failed: {e}"),
        task_id: "my_task".into(),
        timestamp: chrono::Utc::now(),
    })?;
```

### Check Before Deferring

If the condition might already be true, check it before calling `defer_until` to avoid an unnecessary sleep-then-poll cycle:

```rust
if !data_is_ready().await {
    handle.defer_until(|| async { data_is_ready().await }, Duration::from_secs(5)).await?;
}
```

## Summary

You've learned how to:

1. Accept a `TaskHandle` in a task function by naming the second parameter `handle` or `task_handle`
2. Use `defer_until` to release a concurrency slot while polling an external condition
3. Build a workflow where a deferred task feeds data into a downstream task
4. Understand the under-the-hood lifecycle: `SlotToken`, task-local storage, and macro-generated glue code

## Next Steps

- Review the [deferred-tasks example](https://github.com/colliery-io/cloacina/tree/main/examples/features/deferred-tasks) for a complete working project
- Explore [Tutorial 04 - Error Handling]({{< ref "/tutorials/04-error-handling/" >}}) to learn about retry strategies that complement deferred execution
- Check the [API Documentation]({{< ref "/reference/api/" >}}) for the full `TaskHandle` and `SlotToken` API surface
