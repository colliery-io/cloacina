---
title: "07 — Event Triggers"
description: "Fire workflows on custom conditions with in-process triggers."
weight: 17
aliases:
  - "/python/workflows/tutorials/07-event-triggers/"
  - "/workflows/tutorials/service/09-event-triggers/"

---

# 07 — Event Triggers

Cron fires workflows on a clock. A [trigger]({{< ref "/engine/explanation/trigger-rules" >}})
fires one when a condition you define becomes true — a file arrives, a queue
fills, a service goes unhealthy. The runner polls your trigger on an interval; you
return `Fire` (with optional context) to run the workflow, or `Skip` to keep waiting.

## Define the workflow

The trigger fires this workflow. `process_file` reads the `filename` the trigger
passes in via context.

{{< tabs "trigger-workflow" >}}
{{< tab "Rust" >}}
```rust
#[workflow(name = "file_processing", description = "Process incoming files")]
pub mod file_processing {
    use super::*;

    #[task]
    pub async fn validate_file(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = ctx.get("filename").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        ctx.insert("validated", serde_json::json!(true))?;
        info!("File '{}' validated", filename);
        Ok(())
    }

    #[task(dependencies = ["validate_file"])]
    pub async fn process_file(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = ctx.get("filename").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        ctx.insert("processed", serde_json::json!(true))?;
        info!("File '{}' processed", filename);
        Ok(())
    }
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
with cloaca.WorkflowBuilder("file_processing") as builder:
    builder.description("Process incoming files")

    @cloaca.task()
    def validate_file(context):
        filename = context.get("filename", "unknown")
        context.set("validated", True)
        print(f"Validated: {filename}")
        return context

    @cloaca.task(dependencies=["validate_file"])
    def process_file(context):
        filename = context.get("filename", "unknown")
        print(f"Processing: {filename}")
        context.set("processed", True)
        return context
```
{{< /tab >}}
{{< /tabs >}}

## Define the trigger

A trigger checks a condition and returns a `TriggerResult`: `Fire(Some(ctx))` to run
the workflow with that context, or `Skip` to poll again. Here a file watcher "finds" a
file every fifth poll.

{{< tabs "trigger-define" >}}
{{< tab "Rust" >}}
```rust
use cloacina::trigger;
use cloacina::{Context, TriggerError, TriggerResult};
use std::sync::atomic::{AtomicUsize, Ordering};

// The poll fn has no `self`, so polling state lives in a module-level static.
static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

// `#[trigger]` generates the `Trigger` impl plus a zero-arg constructor and
// submits it to the runtime inventory at compile time, so `Runtime::new()`
// (inside `DefaultRunner`) auto-registers it — no explicit `register_trigger`
// call. `on = "..."` names the workflow this trigger fires.
#[trigger(
    name = "file_watcher",
    on = "file_processing",
    poll_interval = "2s",
    allow_concurrent = false
)]
async fn file_watcher() -> Result<TriggerResult, TriggerError> {
    // Pretend a new file lands on disk every fifth poll. Real code would
    // `std::fs::read_dir` or call an object-storage API.
    let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
    if count % 5 != 4 {
        return Ok(TriggerResult::Skip);
    }

    let filename = format!("data_{}.csv", chrono::Utc::now().timestamp());
    let mut ctx = Context::new();
    ctx.insert("filename", serde_json::json!(filename))
        .map_err(|e| TriggerError::PollError { message: e.to_string() })?;
    ctx.insert("watch_path", serde_json::json!("/data/inbox"))
        .map_err(|e| TriggerError::PollError { message: e.to_string() })?;
    Ok(TriggerResult::Fire(Some(ctx)))
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca
import random

@cloaca.trigger(
    name="file_watcher",
    poll_interval="5s",
    allow_concurrent=False
)
def file_watcher():
    # Check for new files (simulated)
    if random.randint(1, 10) == 5:
        ctx = cloaca.Context({"filename": "data_123.csv"})
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```
{{< /tab >}}
{{< /tabs >}}

With `allow_concurrent = false`, the runner hashes the fired context and skips a
fire whose `(trigger_name, context_hash)` is already running — so the same file
isn't processed twice. Put identifying data (a filename, an order id) in the context
so dedup can tell fires apart.

## Register and run

Enable trigger scheduling on the runner, then register the trigger so the runner
polls it. In Rust you bind the trigger to its target workflow explicitly; in
Python the `@cloaca.trigger` decorator registers `file_watcher` at import time, so
constructing the runner is enough to start polling — after that you manage the
trigger at runtime.

{{< tabs "trigger-register" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable trigger scheduling so the runner spins up its poll loop.
    let config = DefaultRunnerConfig::builder()
        .enable_trigger_scheduling(true)
        .trigger_base_poll_interval(Duration::from_secs(1))
        .build()
        .unwrap();

    let runner = DefaultRunner::with_config("sqlite://triggers.db?mode=rwc", config).await?;

    // `#[trigger]` already auto-registered `file_watcher` into the runtime
    // inventory, so look it up by name rather than constructing it.
    let trigger = runner
        .runtime()
        .get_trigger("file_watcher")
        .ok_or("trigger 'file_watcher' not in runtime inventory")?;

    // Persist the trigger -> workflow schedule row so the unified scheduler
    // dispatches `file_processing` whenever the trigger fires.
    let scheduler = runner
        .unified_scheduler()
        .await
        .ok_or("unified scheduler not enabled — check enable_trigger_scheduling()")?;
    scheduler
        .register_trigger(trigger.as_ref(), "file_processing")
        .await?;

    info!("Trigger registered. Running for 30 seconds...");
    tokio::time::sleep(Duration::from_secs(30)).await;

    runner.shutdown().await?;
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca

# The @cloaca.trigger decorator already registered file_watcher at import
# time, so constructing the runner starts the poll loop for it.
runner = cloaca.DefaultRunner("sqlite://triggers.db")

# List the registered trigger schedules
for schedule in runner.list_trigger_schedules():
    print(f"{schedule['trigger_name']} -> {schedule['workflow_name']}")

# Enable or disable a trigger at runtime
runner.set_trigger_enabled("file_watcher", False)
runner.set_trigger_enabled("file_watcher", True)

# Inspect execution history
for execution in runner.get_trigger_execution_history("file_watcher"):
    print(f"Started: {execution['started_at']}")

runner.shutdown()
```
{{< /tab >}}
{{< /tabs >}}

The runner polls `file_watcher` on its interval. In the Rust example the
`register_trigger` call binds it to `file_processing`, so on the fifth poll the
trigger fires that workflow with the new filename in context and `validate_file` →
`process_file` run with it. In Python the decorated `file_watcher` is registered
the moment the runner starts; `list_trigger_schedules`, `set_trigger_enabled`, and
`get_trigger_execution_history` then let you inspect and control it at runtime.

## Next

- Back: **[06 — Multi-tenancy]({{< ref "/embed/tutorials/06-multi-tenancy" >}})**
- Next: **[08 — Task deferral]({{< ref "/embed/tutorials/08-task-deferral" >}})**
- A trigger fires one workflow on a condition. For **whole-graph, event-driven
  traversal** — where the engine reacts to results and walks the graph — see the
  Computation Graph tutorials:
  [10]({{< ref "/embed/tutorials/10-computation-graph" >}}),
  [11]({{< ref "/embed/tutorials/11-accumulators" >}}),
  [12]({{< ref "/embed/tutorials/12-full-pipeline" >}}),
  [13]({{< ref "/embed/tutorials/13-routing" >}}).
- Concepts: [Trigger Rules]({{< ref "/engine/explanation/trigger-rules" >}}),
  [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}),
  [Boundary]({{< ref "/engine/computation-graphs/boundary" >}})
