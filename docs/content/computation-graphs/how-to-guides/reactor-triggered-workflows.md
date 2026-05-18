---
title: "Triggering Workflows from Reactor Firings"
description: "How to fan out reactor firings into workflow executions with Cloacina's DB-backed subscription log"
weight: 30
---

# Triggering Workflows from Reactor Firings

A computation graph (CG) reactor fires whenever its boundary criteria are met.
Out of the box, that fire dispatches the in-process graph function. With
*reactor subscriptions* (CLOACI-I-0100), the same fire can also dispatch one
or more workflows asynchronously, fanning the event out across tenants.

This guide covers the registration API, delivery semantics, the TTL gotcha,
and the metrics you should watch.

## When to use this

Reach for reactor subscriptions when:

- The reactor's traversal is one piece of a larger pipeline and you want
  downstream **workflows** (with retries, audit, recovery) to act on each fire.
- You need **fan-out** — multiple workflows reacting to the same firing under
  different tenants.
- You need durability — events must outlive an in-flight crash and be
  re-dispatched on restart.

For pure cron schedules, keep using `register_cron_workflow`. For
single-process glue with no durability requirement, the CG dispatcher already
runs your graph function inline.

## How it works

```
reactor fires ──► row written to `reactor_firings`
                     │
                     ▼
unified scheduler ──► poll subscriptions ──► dispatch workflow
                                              │
                                              ▼
                                        advance watermark
```

- Every fire writes one `reactor_firings` row containing the boundary cache
  the in-process CG consumed (bincode-encoded).
- The unified scheduler ticks once per second by default, polls subscriptions,
  dispatches one workflow per unconsumed firing, and advances each
  subscription's `last_seen_fired_at`.
- A background TTL prune deletes firings older than the retention window
  (7 days by default).

## Registration

### Python

```python
import cloaca

runner = cloaca.DefaultRunner(database_url)

# Subscribe — fires `incident_response` every time `pricing_reactor` fires
# in the `acme` tenant.
sub_id = runner.subscribe_workflow_to_reactor(
    reactor="pricing_reactor",
    workflow="incident_response",
    tenant="acme",  # optional; defaults to "public"
)

# Inspect what's wired up.
for sub in runner.list_reactor_subscriptions(tenant="acme"):
    print(sub["reactor_name"], "→", sub["workflow_name"])

# Tear down.
runner.unsubscribe_workflow_from_reactor(
    reactor="pricing_reactor",
    workflow="incident_response",
    tenant="acme",
)
```

### Rust

```rust
use cloacina::DefaultRunner;

let runner = DefaultRunner::new(&database_url).await?;

let sub_id = runner
    .subscribe_workflow_to_reactor(
        "pricing_reactor",
        "incident_response",
        Some("acme"),
        None,            // no predicate → fire on every firing
    )
    .await?;

let subs = runner
    .list_reactor_subscriptions(Some("acme"))
    .await?;
```

`subscribe_workflow_to_reactor` is **idempotent** on the
`(reactor, workflow, tenant)` triple — calling it twice upserts; the
later call's predicate (if any) replaces the earlier one's.

## Filtering firings with CEL (T-0602)

Reactors typically fire much more often than you want to dispatch a
workflow. Pass a [CEL](https://github.com/google/cel-spec) expression as
the optional `when` / fourth argument; the scheduler evaluates it against
the firing payload and only dispatches when it returns `true`. The
watermark advances whether or not the firing dispatches, so a "rejected"
firing won't be re-evaluated.

Variables available in the expression:

| Variable   | Meaning                                                      |
|------------|--------------------------------------------------------------|
| `payload`  | Map keyed by boundary source name; values are JSON-decoded   |
| `reactor`  | The reactor name (string)                                    |
| `tenant`   | The tenant id (string)                                       |

```python
# Fire only when the latest quote's price > 100 in us-east.
runner.subscribe_workflow_to_reactor(
    "pricing_reactor",
    "incident_response",
    tenant="acme",
    when="payload.quote.price > 100 && payload.quote.region == 'us-east'",
)
```

```rust
runner
    .subscribe_workflow_to_reactor(
        "pricing_reactor",
        "incident_response",
        Some("acme"),
        Some("payload.quote.price > 100 && payload.quote.region == 'us-east'"),
    )
    .await?;
```

The expression is compiled at subscribe time; malformed CEL is rejected
immediately with `InvalidPredicate` / `ValueError` — no row is written.
At dispatch time the scheduler caches the compiled program per
subscription, so the only per-firing cost is evaluation (microseconds).

**Filter exception semantics.** If the predicate evaluates to something
other than `bool` or the runtime hits an error, the scheduler treats it
as `false`: the firing is **not** dispatched and the watermark advances.
Fail-closed by design — a broken filter doesn't fire workflows
indefinitely.

## Input context for the dispatched workflow

The workflow receives a context populated from the firing's payload:

| Key                  | Source                                    |
|----------------------|-------------------------------------------|
| `<source-name>`      | One per accumulator source; JSON-decoded if possible, otherwise hex-encoded raw bytes |
| `reactor_name`       | The reactor that fired                    |
| `reactor_firing_id`  | UUID of the firing row                    |
| `reactor_fired_at`   | Firing timestamp, RFC 3339                |

Use `reactor_firing_id` for idempotency keys when your workflow has external
side effects.

## Delivery semantics: at-least-once

The poller advances the subscription watermark *after* a successful dispatch.
If the runner crashes between dispatch and watermark advance, the next poll
re-delivers the same firing. **Workflows must be idempotent** — same
constraint as cron-triggered workflows.

The poller does not retry within a single tick: a dispatch error stops the
drain for *that* subscription only and the watermark stays put, so the
firing is retried on the next tick.

## TTL prune gotcha

The retention window (default 7 days) bounds at-least-once delivery. If a
subscription is paused, throttled, or wedged for longer than the retention
window, firings older than the cutoff will be **silently dropped** when the
TTL prune sweeps. This is by design — unbounded growth of the firings table
is worse than missed events for paused subscribers.

Mitigations:

- Watch `cloacina_reactor_firings_pruned_total` for unexpected jumps.
- Bump the retention window if you have subscribers that legitimately pause
  for long periods (e.g., maintenance windows).
- Disable a subscription rather than letting its watermark go stale —
  re-subscribe to start fresh.

## Configuration

| Setting | Default | Notes |
|---------|---------|-------|
| `reactor_poll_interval` | `1s` | Per-tick scan of all subscriptions |
| `reactor_poll_batch_limit` | `100` | Max firings drained per subscription per tick |
| `reactor_firings_prune_interval` | `1h` | TTL sweep cadence |
| `reactor_firings_retention` | `7 days` | Anything older is pruned |

All four live on `SchedulerConfig`. The runner picks defaults from
`DefaultRunnerConfig` and forwards them at scheduler construction time.

## Metrics

| Metric | Type | Labels | Notes |
|--------|------|--------|-------|
| `cloacina_reactor_firings_total` | counter | `graph`, `reactor` | One per fire that successfully wrote a row |
| `cloacina_reactor_firings_pruned_total` | counter | — | Sum of rows deleted by TTL prune |

These are bounded by the reactor set, not by request data, so they sit
comfortably under the I-0099 cardinality guard.

## See also

- [Computation Graph Health]({{< ref "computation-graph-health" >}}) —
  watching reactor liveness
- [Cleaning Up Events]({{< ref "/workflows/how-to-guides/cleaning-up-events" >}}) —
  the equivalent retention story for the workflow execution side
