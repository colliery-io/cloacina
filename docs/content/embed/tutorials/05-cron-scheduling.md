---
title: "05 — Cron Scheduling"
description: "Run a workflow automatically on a time-based cron schedule."
weight: 15
aliases:
  - "/python/workflows/tutorials/05-cron-scheduling/"
  - "/workflows/tutorials/service/05-cron-scheduling/"

---

# 05 — Cron Scheduling

Cloacina has a built-in cron scheduler that runs inside your own process — no
external crontab or task queue. You register a workflow against a cron
expression, and the runner triggers it on schedule.

## Enable scheduling and register a workflow

Define a one-task workflow, turn on cron scheduling in the runner, then register
the workflow against a cron expression and a timezone. We use a fast demo
interval here so you see executions immediately.

{{< tabs "cron-define" >}}
{{< tab "Rust" >}}
```rust
use cloacina::runner::{DefaultRunner, DefaultRunnerConfig};
use cloacina::{task, workflow, Context, TaskError};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::info;

#[workflow(name = "report_workflow", description = "Daily report generation")]
pub mod report_workflow {
    use super::*;

    #[task]
    pub async fn generate_report(context: &mut Context<Value>) -> Result<(), TaskError> {
        info!("Generating report...");
        let report_id = format!("report_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        context.insert("report_id", json!(report_id))?;
        info!("Report generated: {}", report_id);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    // Turn on cron scheduling in the runner config via the builder.
    let config = DefaultRunnerConfig::builder()
        .enable_cron_scheduling(true)
        .cron_poll_interval(Duration::from_secs(5)) // check for due schedules every 5s
        .build()?;

    let runner = DefaultRunner::with_config("sqlite://cron.db", config).await?;

    // Register the workflow against a cron expression + timezone.
    // "*/2 * * * *" is a 5-field cron: every 2 minutes.
    let schedule_id = runner
        .register_cron_workflow("report_workflow", "*/2 * * * *", "UTC")
        .await?;
    info!("Schedule created (ID: {}) - runs every 2 minutes", schedule_id);

    // Let it run, then shut down.
    tokio::time::sleep(Duration::from_secs(120)).await;
    runner.shutdown().await?;
    Ok(())
}
```
{{< /tab >}}
{{< tab "Python" >}}
```python
import cloaca
import time

with cloaca.WorkflowBuilder("daily_report") as builder:
    builder.description("Daily business analytics report")

    @cloaca.task()
    def daily_report(context):
        from datetime import datetime
        report_data = {"generated_at": datetime.now().isoformat(), "total_orders": 150}
        print(f"Daily report generated: {report_data}")
        context.set("report_data", report_data)
        return context

# Turn on cron scheduling in the runner config.
config = cloaca.DefaultRunnerConfig(
    enable_cron_scheduling=True,
    cron_poll_interval_seconds=5,  # check for due schedules every 5s
)
runner = cloaca.DefaultRunner.with_config(":memory:", config)

# Register the workflow against a cron expression + timezone.
# "*/30 * * * * *" is a 6-field cron (leading seconds): every 30 seconds.
# Returns the new schedule's id.
schedule_id = runner.register_cron_workflow("daily_report", "*/30 * * * * *", "UTC")
print(f"Cron schedule registered (ID: {schedule_id})")

# Let it run, then shut down.
time.sleep(65)
runner.shutdown()
```
{{< /tab >}}
{{< /tabs >}}

Run it. The workflow fires on its own at each interval — you'll see the task's
log line appear repeatedly without anyone triggering it.

Cloacina cron expressions take an **optional leading seconds field**, so both
widths are valid:

- **5 fields** — `minute hour day-of-month month day-of-week`. `*/2 * * * *` is
  every two minutes; `0 9 * * 1-5` is 9 AM on weekdays.
- **6 fields** — `seconds minute hour day-of-month month day-of-week`, adding the
  leading seconds field. `*/30 * * * * *` is every 30 seconds.

That is the only difference between the two examples above: the Rust
`*/2 * * * *` (5-field) fires every two minutes, and the Python `*/30 * * * * *`
(6-field, leading seconds) fires every 30 seconds — same dialect, one extra
leading field. Use `"UTC"` unless business logic genuinely requires a local
timezone.

## Handling missed executions

If your process is down when a schedule was due, Cloacina can automatically
re-run executions that were claimed but never finished. Turn on cron recovery in
the config and cap how many missed intervals it will replay at once.

{{< tabs "cron-recovery" >}}
{{< tab "Rust" >}}
```rust
let config = DefaultRunnerConfig::builder()
    .enable_cron_scheduling(true)
    .cron_enable_recovery(true)                    // re-run lost executions
    .cron_recovery_interval(Duration::from_secs(30))
    .cron_max_catchup_executions(50)               // cap catchup to avoid a storm
    .build()?;
```
{{< /tab >}}
{{< tab "Python" >}}
```python
config = cloaca.DefaultRunnerConfig(
    enable_cron_scheduling=True,
    cron_enable_recovery=True,            # re-run lost executions
    cron_recovery_interval_seconds=30,
    cron_max_catchup_executions=50,       # cap catchup to avoid a storm
)
```
{{< /tab >}}
{{< /tabs >}}

For the full recovery-versus-catchup model, see
[Cron Scheduling]({{< ref "/engine/explanation/cron-scheduling" >}}).

Because scheduled runs are at-least-once, keep scheduled tasks idempotent — see
[04 — Error handling]({{< ref "/embed/tutorials/04-error-handling" >}}).

## What you learned / next

You enabled in-process cron scheduling, registered a workflow against a cron
expression and timezone, and chose how missed executions are handled.

- **Next:** [06 — Multi-tenancy]({{< ref "/embed/tutorials/06-multi-tenancy" >}})
- How scheduling works under the hood: [Cron Scheduling]({{< ref "/engine/explanation/cron-scheduling" >}})
- API surface: [Reference]({{< ref "/reference" >}})
