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

    #[task(id = "validate_file", dependencies = [])]
    pub async fn validate_file(ctx: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
        let filename = ctx.get("filename").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
        ctx.insert("validated", serde_json::json!(true))?;
        info!("File '{}' validated", filename);
        Ok(())
    }

    #[task(id = "process_file", dependencies = ["validate_file"])]
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
with cloaca.WorkflowBuilder("file_processor") as builder:
    builder.description("Process incoming files")

    @cloaca.task(id="process_file")
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
use async_trait::async_trait;
use cloacina::trigger::{Trigger, TriggerError, TriggerResult};
use cloacina::Context;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct FileWatcherTrigger {
    name: String,
    poll_interval: Duration,
    watch_path: String,
}

impl FileWatcherTrigger {
    pub fn new(name: &str, watch_path: &str, poll_interval: Duration) -> Self {
        Self {
            name: name.to_string(),
            poll_interval,
            watch_path: watch_path.to_string(),
        }
    }

    async fn check_for_new_files(&self) -> Option<String> {
        let count = FILE_COUNTER.fetch_add(1, Ordering::SeqCst);
        if count % 5 == 4 {
            Some(format!("data_{}.csv", chrono::Utc::now().timestamp()))
        } else {
            None
        }
    }
}

#[async_trait]
impl Trigger for FileWatcherTrigger {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        false // Don't process same file twice
    }

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        if let Some(filename) = self.check_for_new_files().await {
            let mut ctx = Context::new();
            ctx.insert("filename", serde_json::json!(filename))
                .map_err(|e| TriggerError::PollError { message: e.to_string() })?;
            ctx.insert("watch_path", serde_json::json!(self.watch_path.clone()))
                .map_err(|e| TriggerError::PollError { message: e.to_string() })?;
            Ok(TriggerResult::Fire(Some(ctx)))
        } else {
            Ok(TriggerResult::Skip)
        }
    }
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca
import random

@cloaca.trigger(
    workflow="file_processor",
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
polls it.

{{< tabs "trigger-register" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::trigger::register_trigger;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = DefaultRunnerConfig::default();
    config.enable_trigger_scheduling = true;
    config.trigger_base_poll_interval = Duration::from_secs(1);

    let runner = DefaultRunner::with_config("sqlite://triggers.db?mode=rwc", config).await?;

    let trigger = FileWatcherTrigger::new("file_watcher", "/data/inbox", Duration::from_secs(2));
    register_trigger(trigger.clone());

    let dal = runner.dal();
    dal.trigger_schedule().upsert(
        cloacina::models::trigger_schedule::NewTriggerSchedule::new(
            "file_watcher",
            "file_processing",
            Duration::from_secs(2),
        )
    ).await?;

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

runner = cloaca.DefaultRunner("sqlite://triggers.db")

# List registered trigger schedules
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

The runner polls `file_watcher` every two seconds; on the fifth poll it fires
`file_processing` with the new filename in context, and `validate_file` →
`process_file` run with it.

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
