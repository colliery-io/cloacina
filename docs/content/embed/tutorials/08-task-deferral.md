---
title: "08 — Task Deferral"
description: "Release concurrency slots while waiting for external conditions with TaskHandle and defer_until"
weight: 18
aliases:
  - "/workflows/tutorials/service/10-task-deferral/"

---

# 08 — Task Deferral

When a task spends most of its time waiting on I/O, an external service, or a
file that hasn't arrived yet, you don't want it to hold a concurrency slot the
whole time. A `#[task]` function can accept a `TaskHandle` and call
`defer_until` to release its slot, poll a condition, and reclaim a slot once the
condition is met — freeing the executor to run other tasks in the meantime.

{{< hint type=note title="Shown in Rust" >}}
This tutorial is shown in Rust only. `TaskHandle::defer_until` is a Rust-side
capability; consult the [Python TaskHandle reference]({{< ref "/reference/python-api/task" >}})
for the current Python parity status before relying on it from `cloaca`.
{{< /hint >}}

## Prerequisites

- Completion of [Tutorial 01 — First Workflow]({{< ref "/embed/tutorials/01-first-workflow/" >}})
- Comfort with the `#[task]` and `#[workflow]` macros and Rust async/await

## A deferring task and its downstream consumer

`wait_for_data` takes a second parameter, `handle: &mut TaskHandle`. The macro
detects a parameter named `handle` (or `task_handle`) and arranges for the
executor to supply a `TaskHandle` at runtime. Inside the task, `defer_until`
releases the slot and polls until the condition returns `true`. `process_data`
is an ordinary task that runs once the deferred task completes.

```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError, TaskHandle};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

#[workflow(
    name = "deferred_pipeline",
    description = "Pipeline demonstrating deferred task execution"
)]
pub mod deferred_pipeline {
    use super::*;

    #[task]
    pub async fn wait_for_data(
        context: &mut Context<serde_json::Value>,
        handle: &mut TaskHandle,
    ) -> Result<(), TaskError> {
        info!("wait_for_data: Starting — will defer until data is ready");

        // Simulate an external readiness check.
        // In production this would call an API, check a file, etc.
        let poll_count = Arc::new(AtomicUsize::new(0));
        let pc = poll_count.clone();

        handle
            .defer_until(
                move || {
                    let pc = pc.clone();
                    async move {
                        let n = pc.fetch_add(1, Ordering::SeqCst);
                        info!("wait_for_data: polling external source (attempt {})", n + 1);
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

        info!(
            "wait_for_data: Data is ready after {} polls — slot reclaimed",
            poll_count.load(Ordering::SeqCst)
        );

        // Write the "received" data into context for downstream tasks
        context.insert("external_data", json!({"status": "ready", "records": 42}))?;
        Ok(())
    }

    #[task(dependencies = ["wait_for_data"])]
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

        info!("process_data: Processed {} records", records);
        Ok(())
    }
}
```

`defer_until` takes a condition closure that returns `impl Future<Output = bool>`
and is invoked once every poll interval; it returns `Result<(), ExecutorError>`,
which the example maps onto `TaskError`. The condition releases the slot on entry
and reclaims one when it first returns `true` — here, after three polls.

## Run the pipeline

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("deferred_tasks=info,cloacina=info")
        .init();

    let _ = std::fs::remove_file("deferred-tasks.db");

    let runner =
        DefaultRunner::with_config("sqlite://deferred-tasks.db", DefaultRunnerConfig::default())
            .await?;

    // Workflow is auto-registered by the #[workflow] macro.
    let result = runner.execute("deferred_pipeline", Context::new()).await?;

    info!("Status: {:?}", result.status);
    if let Some(count) = result.final_context.get("processed_count") {
        info!("Processed {} records", count);
    }

    runner.shutdown().await?;
    Ok(())
}
```

The executor detects that `wait_for_data` requires a handle and provides one at
runtime. Running it produces output like:

```text
wait_for_data: Starting — will defer until data is ready
wait_for_data: polling external source (attempt 1)
wait_for_data: polling external source (attempt 2)
wait_for_data: polling external source (attempt 3)
wait_for_data: Data is ready after 3 polls — slot reclaimed
process_data: Processed 42 records
Status: Completed
Processed 42 records
```

The three polling lines confirm the task deferred and resumed; `Status:
Completed` with 42 processed records confirms the slot was reclaimed and the
downstream task ran. The full source lives at
`examples/features/workflows/deferred-tasks/`.

For the internal mechanics of slot tokens and the `defer_until` lifecycle, when
deferral is and isn't worth the overhead, and patterns for real-world conditions,
see [Task Deferral]({{< ref "/engine/explanation/task-deferral" >}}).

## Prev / Next

- Prev: [07 — Event Triggers]({{< ref "/embed/tutorials/07-event-triggers/" >}})
- Next: [09 — Working with the Workflow Registry]({{< ref "/embed/tutorials/09-workflow-registry/" >}})
