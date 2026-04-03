---
title: "10 - Task Deferral"
description: "Release concurrency slots while waiting for external conditions with TaskHandle and defer_until"
weight: 20
---

## Overview

This tutorial walks through using `TaskHandle` and its `defer_until` method to release a concurrency slot while a task polls an external condition. When the condition is met, the task reclaims a slot and resumes execution. This pattern is useful for tasks that spend most of their time waiting on I/O, external services, or file availability without occupying a concurrency slot the whole time.

## Prerequisites

Before starting this tutorial, you should:

- Have completed [Tutorial 1 - First Workflow]({{< ref "/tutorials/01-first-workflow/" >}})
- Be comfortable with the `#[task]` and `#[workflow]` macros
- Understand async/await patterns in Rust

## Time Estimate

15-20 minutes

## What You Will Learn

- How to accept a `TaskHandle` in a task function
- How `defer_until` releases and reclaims concurrency slots
- How to compose deferred and non-deferred tasks in a single workflow
- How the executor manages slots across deferred tasks

## Key Concepts

### TaskHandle

`TaskHandle` is an optional second parameter that a `#[task]` function can accept. The macro system detects parameters named `handle` or `task_handle` and automatically arranges for the executor to provide a `TaskHandle` at runtime.

The handle provides access to concurrency slot management. Tasks that do not need this capability omit the parameter entirely and behave as before.

### defer_until

`defer_until` is the primary method on `TaskHandle`. It:

1. Releases the executor's concurrency slot so other tasks can run
2. Polls a user-supplied async condition at a given interval
3. Reclaims a slot when the condition returns `true`
4. Returns control to the task, which continues executing with the slot held

While deferred, the task's async future stays parked in the tokio runtime consuming minimal resources.

## Walkthrough: The Deferred Tasks Example

The example lives in `examples/features/deferred-tasks/`. It defines a two-task pipeline where the first task defers until simulated external data is ready, then the second task processes that data.

### Step 1: Define a Task with TaskHandle

```rust
use cloacina::{task, workflow, Context, TaskError, TaskHandle};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[workflow(
    name = "deferred_pipeline",
    description = "Pipeline demonstrating deferred task execution"
)]
pub mod deferred_pipeline {
    use super::*;

    #[task(id = "wait_for_data", dependencies = [])]
    pub async fn wait_for_data(
        context: &mut Context<serde_json::Value>,
        handle: &mut TaskHandle,
    ) -> Result<(), TaskError> {
        // ...
    }
}
```

The key difference from a normal task is the second parameter: `handle: &mut TaskHandle`. The macro detects this by name (`handle` or `task_handle`) and sets `requires_handle() = true` on the generated `Task` trait implementation.

### Step 2: Use defer_until to Wait for a Condition

Inside `wait_for_data`, the task calls `defer_until` with a condition closure and a poll interval:

```rust
let poll_count = Arc::new(AtomicUsize::new(0));
let pc = poll_count.clone();

handle
    .defer_until(
        move || {
            let pc = pc.clone();
            async move {
                let n = pc.fetch_add(1, Ordering::SeqCst);
                info!("polling external source (attempt {})", n + 1);
                // Simulate: data becomes ready after 3 polls
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
```

While this loop is running:

- The task's concurrency slot is released after the first call to `defer_until`
- Other tasks in the executor can use that freed slot
- Once the condition returns `true` (after 3 polls here), a slot is reclaimed
- Execution continues normally after the `await`

### Step 3: Write Results and Chain to a Downstream Task

After resuming, the task writes data into the context for downstream consumers:

```rust
context.insert("external_data", json!({"status": "ready", "records": 42}))?;
Ok(())
```

A second task depends on the deferred task and processes the data:

```rust
#[task(id = "process_data", dependencies = ["wait_for_data"])]
pub async fn process_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    let data = context
        .get("external_data")
        .ok_or_else(|| TaskError::ExecutionFailed {
            message: "external_data not found in context".into(),
            task_id: "process_data".into(),
            timestamp: chrono::Utc::now(),
        })?
        .clone();

    let records = data.get("records").and_then(|v| v.as_u64()).unwrap_or(0);
    context.insert("processed_count", json!(records))?;
    context.insert("processing_complete", json!(true))?;
    Ok(())
}
```

This task does not take a `TaskHandle` -- it runs as a normal task and executes once `wait_for_data` completes.

### Step 4: Run the Pipeline

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner =
        DefaultRunner::with_config("sqlite://deferred-tasks.db", DefaultRunnerConfig::default())
            .await?;

    let result = runner.execute("deferred_pipeline", Context::new()).await?;

    println!("Status: {:?}", result.status);
    println!("Processed: {} records", result.final_context.get("processed_count").unwrap());

    runner.shutdown().await?;
    Ok(())
}
```

The workflow is auto-registered by the `#[workflow]` macro. The executor detects that `wait_for_data` requires a handle and provides one at runtime.

## Condition Function Requirements

The condition closure passed to `defer_until` must:

- Return `impl Future<Output = bool>`
- Be callable multiple times (it is invoked every `poll_interval`)
- Return `true` when the task should resume

Common real-world conditions include:

| Pattern | Example |
|---------|---------|
| File existence | `Path::new("/data/input.csv").exists()` |
| API readiness | `client.get(url).send().await?.status().is_success()` |
| Queue message | `queue.peek().await.is_some()` |
| Database flag | `db.query("SELECT ready FROM jobs WHERE id = $1", &[&id]).await?.ready` |

## Error Handling

`defer_until` returns `Result<(), ExecutorError>`. It can fail if:

- The executor's semaphore is closed (typically during shutdown)
- The slot cannot be reclaimed

Map the error to `TaskError` to propagate it through the normal task error path, as shown in the example above.

## When Not to Use defer_until

- **Short waits**: If the expected wait is under a few seconds, the overhead of releasing and reclaiming a slot may not be worth it. Just `tokio::time::sleep` instead.
- **CPU-bound polling**: The condition should be cheap. Expensive computation in the condition will block a tokio worker thread.
- **Single-task pipelines**: If only one task is running, releasing the slot provides no benefit since no other task can use it.

## See Also

- [Task Deferral Architecture]({{< ref "/explanation/task-deferral" >}}) -- internal mechanics of slot tokens and the defer_until lifecycle
- [Macro Reference]({{< ref "/reference/macros" >}}) -- full `#[task]` attribute reference including handle detection
- [Python TaskHandle]({{< ref "/python-bindings/api-reference/task" >}}) -- using `TaskHandle` from Python
