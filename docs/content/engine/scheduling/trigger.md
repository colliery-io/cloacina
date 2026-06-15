---
title: "Trigger"
description: "Fires a workflow when a condition is met — a poll function or a cron expression."
weight: 31
---

# Trigger

A **Trigger** starts a [Workflow]({{< ref "/engine/workflows/workflow" >}}) without
a manual call. It names a target workflow and a firing rule. Two kinds:

- **Poll** — a function the scheduler runs on an interval; it decides whether to
  fire (and with what context).
- **Cron** — fires on a [cron schedule]({{< ref "/engine/scheduling/cron-schedule" >}})
  (expression + timezone).

## Mental model

- A trigger is registered with the runner's scheduler, which polls it on its
  interval and fires the workflow when the rule says so.
- Firing is deduplicated (a context hash) so the same fire doesn't double-run.

## Interfaces

{{< tabs "trigger-define" >}}
{{< tab "Rust" >}}
Both kinds are authored with `#[trigger]` and bind to a workflow via `on`:

```rust
// poll trigger
#[trigger(on = "my_workflow", poll_interval = "30s")]
async fn my_poll(/* ... */) -> TriggerResult { /* Fire(ctx) | Skip */ }

// cron trigger
#[trigger(on = "my_workflow", cron = "0 9 * * *", timezone = "UTC")]
struct DailyTrigger;
```
{{< /tab >}}
{{< tab "Python" >}}
Python exposes **poll** triggers as a decorator:

```python
import cloaca

@cloaca.trigger(name="my_poll", poll_interval=30)
def my_poll(context):
    return context   # return to fire; raise/None semantics per the API
```

{{< hint type=warning title="Parity gap" >}}
There is **no packaged/decorator cron trigger** in Python — `#[trigger(cron=…)]`
is Rust-only. Python does full cron **at the runner API** instead
(`register_cron_workflow`, …); see [Cron schedule]({{< ref "/engine/scheduling/cron-schedule" >}}).
Tracked in [CLOACI-T-0688].
{{< /hint >}}
{{< /tab >}}
{{< /tabs >}}

## See also

- [Cron schedule]({{< ref "/engine/scheduling/cron-schedule" >}}) · [Workflow]({{< ref "/engine/workflows/workflow" >}})
