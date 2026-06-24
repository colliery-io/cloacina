---
title: "Trigger"
description: "A named fan-out point that fires one or more workflows when a condition is met — a poll function or a cron expression."
weight: 31
---

# Trigger

A **Trigger** starts one or more [Workflows]({{< ref "/engine/workflows/workflow" >}})
without a manual call. It is a *named fan-out point*: it has a firing rule and a
primary workflow (`on`), and any number of workflows — across packages — can
subscribe to its name. Two kinds of firing rule:

- **Poll** — a function the scheduler runs on an interval; it decides whether to
  fire (and with what context).
- **Cron** — fires on a [cron schedule]({{< ref "/engine/scheduling/cron-schedule" >}})
  (expression + timezone).

## Mental model

- A trigger is registered with the runner's scheduler, which polls it on its
  interval and fires when the rule says so.
- Firing is deduplicated (a context hash) so the same fire doesn't double-run.
- **Fan-out** (CLOACI-T-0777 / T-0778). When a trigger fires, *every* workflow
  subscribed to it runs, not just the primary `on` workflow. A workflow
  subscribes by naming the trigger in `#[workflow(triggers = ["my_trigger"])]`,
  and subscribers may live in other packages; they're resolved from registry
  workflow metadata. The **primary** workflow drives the audit record, return
  value, and error propagation; **secondary** subscribers are best-effort (a
  secondary failure is logged, never fails the primary). A plain cron schedule
  (no trigger name) still binds exactly one workflow. The scheduler's auto-poll
  and a manual fire (`POST /v1/tenants/{t}/triggers/{name}/fire`, server mode)
  fan out the same way.

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

@cloaca.trigger(name="my_poll", poll_interval="30s")
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
