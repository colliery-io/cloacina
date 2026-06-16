---
title: "Subscription fan-out"
description: "Why reactor → workflow subscriptions are DB-backed: the durable event log, per-subscription watermarks, at-least-once delivery, and how this composes with the in-process CG fast path (CLOACI-I-0100)."
weight: 50
aliases:
  - "/computation-graphs/explanation/subscription-fan-out/"

---

# Subscription fan-out

This page explains the model behind the **DB-backed reactor → workflow subscription** shipped in CLOACI-I-0100 — what it is, why it's a separate path from the in-process CG firing path, and what guarantees it gives. For the *recipe* to wire a workflow to a reactor, see [Subscribe a workflow to a reactor]({{< ref "/embed/how-to/subscribe-workflow-to-reactor" >}}). For the CEL filter side, see [Filter reactor firings with CEL]({{< ref "filter-reactor-firings-with-cel" >}}).

## The topology, restated

Per CLOACI-S-0011 (post-2026-04-24 topology amendment), a **reactor** is a standalone publisher of firing events. The reactor itself does not know who its subscribers are; subscribers declare the reactor as their upstream by name (the **upstream-declaration pattern**). Two subscriber kinds coexist on every firing:

| Subscriber kind | Mechanism | Path |
|---|---|---|
| Computation graph | `#[computation_graph(trigger = reactor("name"))]` | **In-process** — graph function called directly in the reactor's loop |
| Workflow trigger | `#[trigger(upstream = reactor("name"))]` | **DB-backed** — a row is written to `reactor_firings`, picked up by a separate poll loop |

Both paths see the same firing payload. The in-process path is dispatched synchronously and has minimum latency but zero durability across restart; the DB-backed path is durable, at-least-once, and survives both server restart and subscriber-side outage.

## Why DB-backed for workflows

Workflows are durable execution graphs. Their lifecycle is owned by the scheduler — they enter `pending`, get claimed, run, complete, get retried. Tying a workflow's *birth* to an in-process reactor firing would couple two lifecycles that should not be coupled:

- The reactor's loop is a hot path. Blocking it on `INSERT INTO workflows (...) ...` for every firing — possibly across many subscribers per firing — is a throughput problem.
- A subscriber-side outage (a workflow's runner unhealthy, a tenant temporarily quiesced) would silently drop firings if the path were in-process.
- Server restart between "reactor fired" and "workflow inserted" would lose the firing.

The DB-backed path decouples the two. The reactor writes a small row per firing (cheap, fixed shape) and moves on; a separate poll loop reads pending rows, applies any CEL predicate, and inserts workflow executions. Subscribers get **at-least-once** delivery, not "best-effort while everything is healthy."

## What gets written, what gets read

On every reactor firing (after the in-process subscribers have been dispatched), the scheduler writes one row to `reactor_firings`:

```text
reactor_firings:
  id              uuid PK
  reactor_name    text
  tenant_id       text
  payload         bytea     -- bincode-serialized HashMap<String, Vec<u8>>
                            -- where each (key, value) is one source name and
                            -- its JSON-encoded boundary value
  fired_at        timestamp
```

The payload is a snapshot of the reactor's input cache at firing time. Each subscriber sees the same bytes.

Per-subscription state is in a separate `reactor_subscriptions` table:

```text
reactor_subscriptions:
  id                  uuid PK
  reactor_name        text
  workflow_name       text
  tenant_id           text
  predicate           text NULL          -- optional CEL expression
  last_seen_fired_at  timestamp NULL     -- watermark
```

The `(reactor_name, workflow_name, tenant_id)` triple is unique — one subscription per (reactor, workflow, tenant). The watermark advances as the poll loop processes firings; it is the only piece of state that distinguishes "delivered" from "pending" for a given subscriber.

## The dispatch loop

Every `reactor_poll_interval` (default `1s`), the runner runs `poll_reactor_subscriptions_once` per tenant:

1. For each `reactor_subscriptions` row in the tenant, fetch up to `reactor_poll_batch_limit` (default 100) firings in `reactor_firings` with `fired_at > last_seen_fired_at`, ordered by `fired_at`.
2. For each firing:
    - If the subscription has no predicate: dispatch a workflow execution with the firing payload merged into the trigger's input context.
    - If the subscription has a predicate: evaluate the CEL expression against the payload; dispatch only if it returns `true`.
    - In either case, advance `last_seen_fired_at` to the firing's timestamp.
3. Commit the watermark update.

The watermark advances even on filtered-out firings — a CEL `false` does not stall the subscription. The watermark advances even on dispatch failure of an *individual* workflow (the workflow row was inserted in a way the scheduler reads later; insertion errors retry on the next tick).

## What "at-least-once" means in practice

A single firing may be delivered to a subscriber **more than once** under three failure modes:

1. **Server crash between watermark advance and commit.** The next poll re-reads the firing, re-evaluates the predicate, and re-dispatches.
2. **Dispatch insert succeeds but watermark commit fails.** Same.
3. **Operator-triggered replay.** Not currently shipped, but if the watermark is manually rewound, every firing past the rewind point is re-delivered.

Workflows that receive reactor firings should be **idempotent at the firing-id granularity** — either by writing to the same logical record (UPSERT) or by including an idempotency key derived from the firing. The dispatched task context includes a `reactor_firing_id` field for exactly this purpose; consult [Filter reactor firings with CEL]({{< ref "filter-reactor-firings-with-cel" >}}) for the idempotency-key recipe.

The dual is "at-most-once" — and the in-process CG path is "at-most-once" by design. If your subscriber must observe every firing exactly once, the DB path is the only viable choice and you must design for idempotency.

## Watermark, TTL, and pruning

The `reactor_firings` table grows monotonically — every firing writes a row. To keep it bounded, a separate prune loop runs every `reactor_firings_prune_interval` (default `1h`) and deletes firings where:

- The firing is older than `reactor_firings_retention` (default `7days`), **and**
- Every active subscription has `last_seen_fired_at >= fired_at` (so no subscription would re-deliver if the row were kept).

A subscription that has never been polled (no `last_seen_fired_at`) holds the entire log open — pruning is conservative. If you remove a subscription, prune cleans up its tail on the next tick.

This means the audit window for "what did this reactor fire?" is bounded by `reactor_firings_retention`. Increase the retention if you need longer audit windows; the storage cost is one row per firing.

## Composition with the in-process path

Both paths fire on the same event. The in-process path is **always** synchronous to the reactor loop — every `#[computation_graph(trigger = reactor("foo"))]` declaration runs first, in declaration order, before the reactor returns from its firing handler. Only after every in-process subscriber has been called does the reactor write the `reactor_firings` row that the DB path will pick up.

In other words: in-process CG subscribers see firings *before* DB-backed workflow subscribers, by one event-loop tick plus one poll-interval. Latency budget:

- In-process CG: tens of microseconds (one async call).
- DB-backed workflow: `(reactor_poll_interval / 2)` average + one workflow-execution-insert + one workflow-claim-and-run cycle. Under default `reactor_poll_interval = 1s`, expect ~500ms fire-to-execution-row latency.

If a downstream needs *both* an in-process CG (for fast routing decisions) and a workflow (for durable downstream effects), declare both — they coexist, see the same firing payload, and run on the latency budgets above.

## When to use this vs alternatives

| You want | Reach for |
|---|---|
| A workflow that fires on a reactor firing, durable across restart | DB-backed subscription (this page; [recipe]({{< ref "/embed/how-to/subscribe-workflow-to-reactor" >}})) |
| A computation graph that fires on a reactor firing, minimum latency | `#[computation_graph(trigger = reactor("..."))]` ([Tutorial 07]({{< ref "/embed/tutorials/10-computation-graph" >}})) |
| A workflow that fires on a non-reactor source (cron, file watch, HTTP poll) | Implement the [`Trigger` trait]({{< ref "/embed/tutorials/07-event-triggers" >}}) — no reactor involved |
| Both — a fast in-process CG *and* a durable downstream workflow | Both. They coexist on the same firing. |

## References

- **CLOACI-I-0100** — DB-backed reactor → workflow subscription fan-out (initiative).
- **CLOACI-T-0602** — CEL predicate filtering on subscriptions.
- **CLOACI-S-0011** — Primitive nomenclature spec; 2026-04-24 topology amendment makes the reactor a standalone publisher.
- **Code**: `crates/cloacina/src/dal/unified/reactor_subscriptions.rs`, `crates/cloacina/src/runner/default_runner/reactor_subscriptions_api.rs`, `crates/cloacina/src/cron_trigger_scheduler.rs`.
- **Example**: `examples/features/computation-graphs/filtered-reactor/` — runnable end-to-end demo.
