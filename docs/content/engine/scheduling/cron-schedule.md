---
title: "Cron schedule"
description: "Time-based scheduling of a workflow, with skip / run-all catch-up policies."
weight: 32
---

# Cron schedule

A **Cron schedule** runs a [Workflow]({{< ref "/engine/workflows/workflow" >}}) on a
time-based schedule (a cron expression + timezone), with a **catch-up policy** for
runs missed while the process was down: **skip** (ignore them) or **run-all**
(replay them, up to a bound).

## Interfaces

{{< tabs "cron-define" >}}
{{< tab "Rust" >}}
Authored declaratively as a cron [Trigger]({{< ref "/engine/scheduling/trigger" >}})
in a package:

```rust
#[trigger(on = "daily_report", cron = "0 9 * * *", timezone = "UTC")]
struct DailyReport;
```
{{< /tab >}}
{{< tab "Python" >}}
Managed at the **runner API** — register and manage schedules at runtime; read
operations return dicts:

```python
runner = cloaca.DefaultRunner("postgresql://…")

schedule_id = runner.register_cron_workflow(
    "daily_report",   # workflow name
    "0 9 * * *",      # cron expression
    "UTC",            # timezone
)
runner.list_cron_schedules(enabled_only=True)   # list[dict]
runner.set_cron_schedule_enabled(schedule_id, False)
runner.delete_cron_schedule(schedule_id)
```
{{< /tab >}}
{{< /tabs >}}

## Key facts

- **Catch-up policy:** `skip` or `run-all` (run-all is bounded by a max-catchup
  count).
- **Schedule dicts (Python):** `id`, `workflow_name`, `cron_expression`,
  `timezone`, `enabled`, `catchup_policy`, `next_run_at`, `last_run_at`,
  `created_at`, `updated_at`.
- **Parity:** Python has the full runtime cron API but **no packaged cron-trigger
  decorator** (Rust-only) — tracked in [CLOACI-T-0688].

## See also

- [Trigger]({{< ref "/engine/scheduling/trigger" >}}) · [Cron scheduling (design)]({{< ref "/workflows/explanation/cron-scheduling" >}})
