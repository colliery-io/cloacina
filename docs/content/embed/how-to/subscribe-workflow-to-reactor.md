---
title: "Subscribe a workflow to a reactor"
description: "Wire a workflow trigger to a reactor's firings via the DB-backed subscription fan-out (CLOACI-I-0100). Durable, at-least-once, optionally filtered with CEL."
weight: 18
---

# Subscribe a workflow to a reactor

This recipe wires a **workflow** to fire whenever a named **reactor** (a computation-graph reactor) fires. The wire is the DB-backed subscription fan-out shipped in CLOACI-I-0100 — durable across restarts, at-least-once delivery, optionally filtered by a CEL predicate (CLOACI-T-0602).

Use this when the workflow needs to run as a reaction to upstream event-driven activity that's already being correlated by a reactor, and where durability matters (the subscription survives server restart and any subscriber-side outage).

## Background

Per CLOACI-S-0011 (post-2026-04-24 topology amendment), a **reactor** is a standalone publisher of firing events. Multiple subscribers can attach to one reactor: any number of computation graphs (in-process fast path) and any number of workflow triggers (this recipe — durable path) can declare the reactor as their upstream.

The two paths coexist:

| Path | Mechanism | Use when |
|---|---|---|
| **In-process fast path** | `#[computation_graph(trigger = reactor("name"))]` | The downstream is a CG and you want minimum-latency firing. |
| **DB-backed subscription** (this recipe) | `#[trigger(upstream = reactor("name"))]` on a workflow | The downstream is a workflow and you want durability. |

Both paths are dispatched off the same reactor firing — the in-process fast path fires synchronously inside the reactor's loop; the DB path writes a row into `reactor_firings` and a separate poll loop dispatches the workflow subscribers. The reactor itself doesn't know who its subscribers are; subscribers declare the reactor as their upstream by name. This is the **upstream-declaration pattern**.

For the conceptual rationale — why DB-backed, what at-least-once means, how it composes with the in-process path — see [Subscription fan-out]({{< ref "/engine/explanation/subscription-fan-out" >}}).

## Prerequisites

- A reactor exists somewhere in your deployment (declared in a packaged or embedded `#[reactor(...)]`). You'll reference it by string name.
- A workflow you want to fire on the reactor's firings.
- The reactor must be loaded by the time the workflow trigger registers — the subscription registers eagerly and will error if the reactor name doesn't resolve at startup. (For cross-package deployments, package load order matters; see [reactor-lifecycle]({{< ref "/engine/explanation/reactor-lifecycle" >}}).)

## Steps

### 1. Declare the trigger with `upstream = reactor("name")`

```rust
use cloacina::{trigger, workflow, task, Context, TaskError};
use serde_json::Value;

#[trigger(
    name = "price_signal_workflow_trigger",
    upstream = reactor("pricing_pipeline_reactor"),
)]
pub struct PriceSignalTrigger;

#[task(id = "process_signal")]
pub async fn process_signal(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // The reactor's firing payload is injected into the task context
    // as the `reactor_payload` key. Extract whatever you need:
    let payload = ctx.get("reactor_payload").cloned();
    // ... do work ...
    Ok(())
}

#[workflow(
    name = "price_signal_processing",
    triggers = [PriceSignalTrigger],
)]
pub mod price_signal_processing {
    pub use super::process_signal;
}
```

The `upstream = reactor("pricing_pipeline_reactor")` clause registers the workflow's trigger as a DB-backed subscriber of the named reactor. The reactor itself needs no modification — it just fires; the subscription writes a row per firing.

### 2. (Optional) Add a CEL predicate filter (T-0602)

To fire the workflow only on a subset of reactor firings, register the subscription with a CEL predicate:

```rust
// Programmatic registration (alternative to the macro-only form above)
// when you need a filter predicate.
runner.subscribe_workflow_to_reactor(
    "pricing_pipeline_reactor",        // reactor name
    "price_signal_processing",         // workflow name
    "price_signal_workflow_trigger",   // trigger name
    Some("payload.value > 100"),       // CEL predicate (None = fire every time)
).await?;
```

CEL variables available to the predicate:

- `payload` — the reactor firing's payload (a JSON object).
- `reactor` — metadata about the firing reactor (name, etc.).
- `tenant` — the tenant the firing is scoped to.

CEL predicates are **compiled at subscribe time** — a malformed predicate errors at registration, not on every firing. Evaluation is **fail-closed** — if the CEL evaluation itself errors at firing time, the firing is skipped (logged with the error) rather than dispatched. See `examples/features/computation-graphs/filtered-reactor` for a runnable worked example.

### 3. Verify the subscription is registered

```sh
cloacinactl --profile prod trigger list --tenant my-tenant | grep price_signal
```

The trigger should appear with `upstream = reactor("pricing_pipeline_reactor")` in its metadata.

### 4. Verify the firing path end-to-end

Cause a reactor firing (push a boundary event, or use the WebSocket force-fire path per `/v1/ws/reactor/{name}` with `{"type":"ForceFire"}`), then check the workflow execution log:

```sh
cloacinactl --profile prod execution list --tenant my-tenant --workflow price_signal_processing
```

You should see one workflow execution per reactor firing (or per firing that passed the CEL filter, if you set one).

## Configuration knobs

Per-runner knobs that govern subscription dispatch:

| Knob | Default | Notes |
|---|---|---|
| `reactor_poll_interval` | `1s` | How often the subscription poll loop runs. Lower = lower fire-to-dispatch latency, more DB load. |
| `reactor_poll_batch_limit` | `100` | Max firings dispatched per poll tick. Bound on burst dispatch. |
| `reactor_firings_prune_interval` | `1h` | How often the durable event log is pruned. |
| `reactor_firings_retention` | `7days` | Retention window for `reactor_firings` rows. Past this, dispatched firings are GC'd. Increase for longer audit windows. |

These are set on the per-tenant `DefaultRunner` config. See [Configuration Reference]({{< ref "/reference/configuration" >}}) for the full knob set.

## Metrics

Two counters track subscription throughput (full catalog in [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}})):

- `cloacina_reactor_firings_total{graph, reactor}` — per-reactor firings recorded in the durable log.
- `cloacina_reactor_firings_pruned_total` — firings GC'd by the prune loop.

For the workflow execution side, the existing `cloacina_workflows_total{status, reason}` counter is incremented per workflow execution as normal.

## When to use this vs the in-process Trigger trait

| You want | Reach for |
|---|---|
| A workflow that fires on a reactor firing, durable across restart | This recipe (`upstream = reactor("name")` on `#[trigger]`) |
| A computation graph that fires on a reactor firing, in-process minimum-latency | `#[computation_graph(trigger = reactor("name"))]` (see [Tutorial 07]({{< ref "/computation-graphs/tutorials/library/07-computation-graph" >}})) |
| A workflow that fires on a custom poll / event / file watch | Implement the [Trigger trait]({{< ref "/service/tutorials/09-event-triggers" >}}) — no reactor involved |

## What this how-to does NOT cover

- **Reactor authoring.** This recipe assumes the reactor exists. See [Tutorial 07 — Your First Computation Graph]({{< ref "/computation-graphs/tutorials/library/07-computation-graph" >}}) for the reactor side.
- **Tearing down a subscription.** Removing the `#[trigger]` from the workflow and re-deploying the package unregisters the subscription. There is no separate CLI for it today.
- **Replaying past firings.** The `reactor_firings` log is forward-looking; replay would require a separate mechanism (not currently shipped).

## See also

- [Subscription fan-out]({{< ref "/engine/explanation/subscription-fan-out" >}}) — conceptual model (deferred from DOC-F; may be a stub at time of reading).
- [Filter reactor firings with CEL]({{< ref "/engine/computation-graphs/how-to/filter-reactor-firings-with-cel" >}}) — focused recipe for the predicate side (DOC-F deliverable).
- [Reactor-triggered workflows]({{< ref "/engine/computation-graphs/how-to/reactor-triggered-workflows" >}}) — the existing CG-side how-to with the dual-path topology overview.
- [Tutorial 09 — Event Triggers]({{< ref "/service/tutorials/09-event-triggers" >}}) — the in-process `Trigger` trait alternative.
- **CLOACI-I-0100** — DB-backed reactor → workflow subscription fan-out.
- **CLOACI-T-0602** — CEL predicate filtering on subscriptions.
- **CLOACI-S-0011** — primitive nomenclature spec (post-2026-04-24 topology amendment).
