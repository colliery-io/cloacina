---
title: "Monitoring Executions"
description: "How to observe workflow executions from an embedded runner: status polling, callbacks, cron and trigger history in Rust and Python"
weight: 60
aliases:
  - "/workflows/how-to-guides/monitoring-executions/"

---

# Monitoring Executions

This guide shows how to observe workflow executions **from inside your own process** when you embed the Cloacina runner — polling status, listing recent runs, receiving real-time callbacks, and querying cron/trigger execution history.

If you run the standalone API server, monitor over HTTP instead: the execution and trigger routes are documented in the [HTTP API Reference]({{< ref "/reference/http-api" >}}), and the [service SDKs]({{< ref "/reference/sdks" >}}) (`cloacina-client` for Python, `@cloacina/client` for TypeScript) wrap them. For the server's `/metrics`, logs, and tracing surfaces, see [Observe Execution State]({{< ref "/embed/how-to/observe-execution-state" >}}).

## Prerequisites

- An embedded `DefaultRunner` — Rust (`cloacina` crate) or Python (`pip install cloaca`). Note the Python package is `cloaca`; `cloacina-client` is the separate HTTP service SDK and is not used here.
- Workflows registered with the runner (see [Tutorial 01]({{< ref "/embed/tutorials/01-first-workflow" >}})).

## Checking a Single Execution

`execute_async` returns a handle immediately; poll it or wait on it:

```rust
use cloacina::prelude::*;

let execution = runner.execute_async("data-ingest", context).await?;

// Non-blocking status check
let status = execution.get_status().await?;   // Pending | Running | Completed | Failed | Cancelled | Paused
println!("status: {:?}, terminal: {}", status, status.is_terminal());

// Or block until it finishes (optionally with a timeout)
let result = execution
    .wait_for_completion_with_timeout(Some(std::time::Duration::from_secs(600)))
    .await?;
println!("{} finished as {:?} in {:?}", result.workflow_name, result.status, result.duration);
for task in &result.task_results {
    println!("  {}: {:?} ({} attempt(s))", task.task_name, task.status, task.attempt_count);
}
```

If you only have an execution ID (e.g. persisted from an earlier run), query it directly:

```rust
let status = runner.get_execution_status(execution_id).await?;
let result = runner.get_execution_result(execution_id).await?;
```

`WorkflowExecutionResult` carries `execution_id`, `workflow_name`, `status`, `start_time`, `end_time`, `duration`, `final_context`, `task_results`, and `error_message`.

In Python, `execute` is blocking and returns a `WorkflowResult` with `status`, `start_time`, `end_time`, `final_context`, and `error_message`:

```python
import cloaca

runner = cloaca.DefaultRunner("sqlite://app.db")
result = runner.execute("data_ingest", cloaca.Context({"date": "2026-08-02"}))
print(result.status, result.error_message)
```

## Listing Recent Executions

```rust
// Capped at the 100 most recent executions
let recent = runner.list_executions().await?;
let failed: Vec<_> = recent
    .iter()
    .filter(|r| r.status == WorkflowStatus::Failed)
    .collect();
for r in &failed {
    println!("[ALERT] {} ({}) failed: {:?}", r.workflow_name, r.execution_id, r.error_message);
}
```

## Real-Time Status Callbacks

To react to status transitions as they happen (progress reporting, alerting), implement `StatusCallback` and use `execute_with_callback`:

```rust
use cloacina::prelude::*;

struct LogCallback;

impl StatusCallback for LogCallback {
    fn on_status_change(&self, status: WorkflowStatus) {
        tracing::info!("workflow status changed: {:?}", status);
    }
}

let result = runner
    .execute_with_callback("data-ingest", context, Box::new(LogCallback))
    .await?;
```

## Monitoring Cron Schedules

The cron API on the runner exposes schedules, per-schedule history, and aggregate stats. All of these error if `enable_cron_scheduling` is false (it is on by default).

{{< tabs "monitor-cron" >}}
{{< tab "Rust" >}}
```rust
// Schedules
let schedules = runner.list_cron_schedules(true /* enabled_only */, 50, 0).await?;

// Per-schedule execution history
let history = runner.get_cron_execution_history(schedule_id, 10, 0).await?;

// Aggregate stats since a point in time
let since = chrono::Utc::now() - chrono::Duration::hours(24);
let stats = runner.get_cron_execution_stats(since).await?;
```
{{< /tab >}}
{{< tab "Python" >}}
```python
from datetime import datetime, timedelta, timezone
import cloaca

runner = cloaca.DefaultRunner("sqlite://app.db")

# Schedules — list of dicts with keys: id, workflow_name, cron_expression,
# timezone, enabled, catchup_policy, next_run_at, last_run_at, created_at, updated_at
schedules = runner.list_cron_schedules(enabled_only=True, limit=50, offset=0)
for s in schedules:
    print(f"{s['workflow_name']}: {s['cron_expression']} (next: {s['next_run_at']})")

# Per-schedule execution history — dicts with keys: id, schedule_id,
# scheduled_time, claimed_at, workflow_execution_id, created_at, updated_at
schedule_id = schedules[0]["id"]
for h in runner.get_cron_execution_history(schedule_id, limit=10, offset=0):
    print(f"  {h['scheduled_time']} -> execution {h['workflow_execution_id']}")

# Aggregate stats — `since` is an ISO 8601 / RFC 3339 string
since = (datetime.now(timezone.utc) - timedelta(hours=24)).isoformat()
stats = runner.get_cron_execution_stats(since)
print(f"Total: {stats['total_executions']}, "
      f"OK: {stats['successful_executions']}, "
      f"Lost: {stats['lost_executions']}, "
      f"Success rate: {stats['success_rate']}")
```
{{< /tab >}}
{{< /tabs >}}

## Monitoring Poll Triggers

The Python runner exposes trigger schedules and their firing history directly:

```python
# Trigger schedules — dicts with keys: id, trigger_name, workflow_name,
# poll_interval_ms, allow_concurrent, enabled, last_poll_at, created_at, updated_at
for t in runner.list_trigger_schedules(enabled_only=True, limit=50, offset=0):
    print(f"{t['trigger_name']}: polling every {t['poll_interval_ms']}ms "
          f"(last poll: {t['last_poll_at']})")

# One trigger by name (returns None if unknown)
schedule = runner.get_trigger_schedule("check_s3_bucket")

# Firing history — dicts with keys: id, schedule_id, context_hash,
# workflow_execution_id, started_at, completed_at, created_at
history = runner.get_trigger_execution_history("check_s3_bucket", limit=10, offset=0)

# Pause / resume a trigger
runner.set_trigger_enabled("check_s3_bucket", False)
```

In Rust there is no dedicated trigger-monitoring method on `DefaultRunner`; the schedule rows are reachable through the data-access layer via `runner.dal()` (the same store the Python methods read). Prefer the Python surface or logs unless you need programmatic access from Rust.

## Building a Simple Watchdog

A minimal in-process watchdog that alerts on failures:

```python
import time
import cloaca

runner = cloaca.DefaultRunner("sqlite://app.db")

while True:
    for t in runner.list_trigger_schedules(enabled_only=True):
        if t["last_poll_at"] is None:
            print(f"[WARN] trigger '{t['trigger_name']}' has never polled")
    for s in runner.list_cron_schedules(enabled_only=True, limit=100, offset=0):
        if s["last_run_at"] is None:
            print(f"[WARN] schedule for '{s['workflow_name']}' has never fired")
    time.sleep(60)
```

For production alerting, feed these into your existing observability stack — and see [Observe Execution State]({{< ref "/embed/how-to/observe-execution-state" >}}) for the metrics/logs/tracing surfaces of the server and daemon deployments.

## See Also

- [Observe Execution State]({{< ref "/embed/how-to/observe-execution-state" >}}) — metrics, logs, and tracing for server/daemon deployments.
- [HTTP API Reference]({{< ref "/reference/http-api" >}}) — monitoring the standalone server over HTTP.
- [Service SDKs]({{< ref "/reference/sdks" >}}) — `cloacina-client` / `@cloacina/client` for HTTP monitoring from code.
- [Tutorial 05 — Cron Scheduling]({{< ref "/embed/tutorials/05-cron-scheduling" >}}) — registering the schedules monitored here.
