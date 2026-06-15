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

    #[task(id = "generate_report", dependencies = [])]
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

    // Turn on cron scheduling in the runner config.
    let mut config = DefaultRunnerConfig::default();
    config.enable_cron_scheduling = true;
    config.cron_poll_interval = Duration::from_secs(5); // check for due schedules every 5s

    let runner = DefaultRunner::with_config("sqlite://cron.db", config).await?;

    // Register the workflow against a cron expression + timezone.
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

    @cloaca.task(id="daily_report")
    def daily_report(context):
        from datetime import datetime
        report_data = {"generated_at": datetime.now().isoformat(), "total_orders": 150}
        print(f"Daily report generated: {report_data}")
        context.set("report_data", report_data)
        return context

runner = cloaca.DefaultRunner(":memory:")

# Register the workflow against a cron expression + timezone.
schedule = cloaca.CronSchedule(
    workflow_name="daily_report",
    cron_expression="*/30 * * * * *",  # every 30 seconds for the demo
    timezone="UTC",
    enabled=True,
)
runner.add_cron_schedule(schedule)
print("Cron schedule registered")

# Let it run, then shut down.
time.sleep(65)
runner.shutdown()
```
{{< /tab >}}
{{< /tabs >}}

Run it. The workflow fires on its own at each interval — you'll see the task's
log line appear repeatedly without anyone triggering it.

## Pass context into a scheduled run

A schedule can carry a default [Context]({{< ref "/embed/tutorials/02-context" >}})
so each scheduled execution starts with known inputs.

{{< tabs "cron-context" >}}
{{< tab "Rust" >}}
```rust
// In Rust, the workflow's tasks read whatever defaults you set inside them;
// register the same workflow under several schedules to vary behaviour.
runner
    .register_cron_workflow("report_workflow", "0 9 * * 1-5", "America/New_York")
    .await?; // 9 AM on weekdays, Eastern time
```
{{< /tab >}}
{{< tab "Python" >}}
```python
schedule = cloaca.CronSchedule(
    workflow_name="system_backup",
    cron_expression="0 2 * * SUN",
    timezone="UTC",
    enabled=True,
    context=cloaca.Context({"backup_type": "full"}),
)
runner.add_cron_schedule(schedule)
```
{{< /tab >}}
{{< /tabs >}}

The cron fields are standard: minute, hour, day-of-month, month, day-of-week.
`*/2 * * * *` is every two minutes; `0 9 * * 1-5` is 9 AM on weekdays. Use `"UTC"`
unless business logic genuinely requires a local timezone.

## Handling missed executions

If your process is down when a schedule was due, Cloacina decides what to do via
recovery and catchup. **Recovery** automatically re-runs executions that were
claimed but never finished (e.g. a crash) — it's on by default. **Catchup**
governs intentional downtime: `Skip` ignores intervals missed while down (right
for health checks), while `RunAll` replays them (right for ETL that must process
every interval).

{{< tabs "cron-recovery" >}}
{{< tab "Rust" >}}
```rust
let mut config = DefaultRunnerConfig::default();
config.enable_cron_scheduling = true;
config.cron_enable_recovery = true;          // re-run lost executions
config.cron_recovery_interval = Duration::from_secs(30);
config.cron_max_catchup_executions = 50;     // cap catchup to avoid a storm
```
{{< /tab >}}
{{< tab "Python" >}}
```python
# Recovery is enabled by default. To replay intervals missed during downtime,
# choose a catchup policy on the schedule (Skip ignores them, RunAll replays).
schedule = cloaca.CronSchedule(
    workflow_name="incremental_backup",
    cron_expression="0 3 * * *",
    timezone="UTC",
    enabled=True,
)
```
{{< /tab >}}
{{< /tabs >}}

Because scheduled runs are at-least-once, keep scheduled tasks idempotent — see
[04 — Error handling]({{< ref "/embed/tutorials/04-error-handling" >}}).

## What you learned / next

You enabled in-process cron scheduling, registered a workflow against a cron
expression and timezone, passed default context, and chose how missed executions
are handled.

- **Next:** [06 — Multi-tenancy]({{< ref "/embed/tutorials/06-multi-tenancy" >}})
- How scheduling works under the hood: [Cron Scheduling]({{< ref "/engine/explanation/cron-scheduling" >}})
- API surface: [Reference]({{< ref "/reference" >}})
