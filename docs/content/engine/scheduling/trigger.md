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
- **Operator controls** (server mode): triggers can be paused and resumed over
  REST — `POST /v1/tenants/{t}/triggers/{name}/pause` and `.../resume` — and
  listed/inspected via `GET /v1/tenants/{t}/triggers[/{name}]`. (Reactors, by
  contrast, are paused/resumed over their WebSocket channel only.)

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
Both kinds are decorators in Python too:

```python
import cloaca

# poll trigger — return TriggerResult.fire(ctx) to fire, .skip() otherwise
# (a plain bool also works: True = fire with no context)
@cloaca.trigger(name="my_poll", on="my_workflow", poll_interval="30s")
def my_poll():
    return cloaca.TriggerResult.skip()

# cron trigger — the function body is unused; `on` is required
@cloaca.trigger(on="my_workflow", cron="0 9 * * *", timezone="UTC")
def daily():
    pass
```

Cron support on `@cloaca.trigger` was added in CLOACI-T-0688 — the reconciler
routes cron-bearing triggers to the cron scheduler. Python also has the full
runtime cron API (`register_cron_workflow`, …); see
[Cron schedule]({{< ref "/engine/scheduling/cron-schedule" >}}). Note
`cloacinactl package new` only scaffolds cron packages for Rust — hand-author
the decorator in Python.
{{< /tab >}}
{{< /tabs >}}

{{< hint type=info title="Not the same as reactor subscription fan-out" >}}
Trigger-name fan-out (one trigger *name*, many subscribing workflows) is a
different primitive from the DB-backed **reactor → workflow** subscription
fan-out. The latter has a reactor publisher, a durable `reactor_firings` log, and
at-least-once delivery; this has none of those. See
[Trigger-name fan-out vs reactor subscription fan-out]({{< ref "/engine/explanation/subscription-fan-out#trigger-name-fan-out-vs-reactor-subscription-fan-out" >}})
for the side-by-side.
{{< /hint >}}

## See also

- [Cron schedule]({{< ref "/engine/scheduling/cron-schedule" >}}) · [Workflow]({{< ref "/engine/workflows/workflow" >}})
- [Subscription fan-out]({{< ref "/engine/explanation/subscription-fan-out" >}}) — the *other* fan-out (reactor → workflow)
