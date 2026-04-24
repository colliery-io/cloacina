---
id: reactor-triggered-workflows-db
level: initiative
title: "Reactor-triggered workflows — DB-backed subscription fan-out"
short_code: "CLOACI-I-0100"
created_at: 2026-04-23T17:05:45.520081+00:00
updated_at: 2026-04-23T17:05:45.520081+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: reactor-triggered-workflows-db
---

# Reactor-triggered workflows — DB-backed subscription fan-out

## Context

Today a reactor's firing has exactly one subscriber: the compiled computation-graph traversal it was declared alongside. Workflows can be triggered by cron schedules, custom `@trigger` poll functions, and manual API calls — but not by reactors. Users who want an event-driven workflow kicked off by an accumulator-backed signal have to bridge the two systems manually (e.g., a CG terminal node calling the workflow API).

CLOACI-S-0011 ("Cloacina primitive nomenclature") positioned reactor as the specialized-trigger *noun* for the CG world. This initiative generalizes that: the reactor remains a single primitive, but its downstream is no longer restricted to a CG. Any number of workflow triggers can subscribe to a reactor's firings via an upstream-oriented declaration (`@trigger(reactor="name")`), matching how every other primitive in the system is defined ("what upstream do I react to?").

Supersedes CLOACI-T-0499 ("Accumulators as workflow event triggers"), which had the wrong primitive in its title — accumulators don't exist independently of a reactor, so the correct level of abstraction is *reactor* subscription, not accumulator subscription.

## Goals & Non-Goals

**Goals:**
- Let a reactor's firing drive any number of workflow executions.
- Use the existing upstream-declaration pattern: workflow trigger says *"my upstream is reactor X"*; the reactor does not know about its subscribers.
- DB-backed event log so that firings survive restart between subscriber polls, and so fan-out is a simple consequence of multiple rows pointing at the same reactor.
- Preserve the specialized low-latency CG traversal path — that subscription is the in-process fast path, this initiative adds the generic durable path alongside it.
- Fan-out at day one: N workflows can subscribe to one reactor.

**Non-Goals:**
- Workflow → reactor triggering. Already supported (a workflow task can push to an accumulator via the existing WS surface); no new work needed.
- Distributed / cross-process reactor firings. Today reactor and workflow live in the same server; extending to cross-server pub/sub is tied to S-0008 (horizontal scaling) and out of scope here.
- Accumulator-direct subscriptions. Per S-0011, accumulators exist only as reactor inputs — they do not become a standalone primitive.
- Replay / replay-point / exactly-once delivery beyond the subscription watermark. At-least-once delivery with idempotency the workflow's concern.

## Architecture

### Shape

```
Accumulators ──▶ Reactor ──fires──┬──▶ CG traversal   (in-process, unchanged fast path)
                                  │
                                  └──▶ reactor_firings row
                                          │
                                          ▼
                                   subscription poller
                                          │
                                          ▼
                                   workflow execution (1..N fan-out)
```

- Reactor runtime gains one additional side effect: write a row to `reactor_firings` when it fires. The in-process CG dispatch is untouched.
- `reactor_trigger_subscriptions` holds one row per (reactor, workflow) pair, with a `last_seen_fired_at` watermark.
- A new service loop (either the unified scheduler extended or a peer) polls subscriptions, reads unconsumed firings, dispatches workflows, and advances watermarks.
- TTL sweep prunes old `reactor_firings` rows, consistent with other event-log retention patterns already in the codebase.

### Declaration surface (user-facing)

Reactor side: **no changes.** Reactors declare only their accumulator upstream.

Workflow side: the existing `@trigger` decorator gains a new input-source kwarg:

```python
@cloaca.trigger(reactor="pricing_reactor")
def on_pricing_firing(ctx):
    # optional filter / transform; return Fire or Skip
    return cloaca.TriggerResult.fire(ctx)
```

A registration API is also exposed for callers that don't want a Python filter function (the trivial "fire on every reactor firing" case):

```python
cloaca.subscribe_workflow_to_reactor(reactor="pricing_reactor", workflow="incident_response")
```

### Schema

```sql
CREATE TABLE reactor_firings (
    id              UUID PRIMARY KEY,
    reactor_name    TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    payload         BYTEA,                -- the same data the CG traversal receives
    fired_at        TIMESTAMP NOT NULL,
    created_at      TIMESTAMP NOT NULL
);
CREATE INDEX reactor_firings_by_reactor_and_time
    ON reactor_firings (tenant_id, reactor_name, fired_at);

CREATE TABLE reactor_trigger_subscriptions (
    id                    UUID PRIMARY KEY,
    reactor_name          TEXT NOT NULL,
    workflow_name         TEXT NOT NULL,
    tenant_id             TEXT NOT NULL,
    enabled               BOOLEAN NOT NULL DEFAULT TRUE,
    last_seen_fired_at    TIMESTAMP,
    created_at            TIMESTAMP NOT NULL,
    updated_at            TIMESTAMP NOT NULL,
    UNIQUE (reactor_name, workflow_name, tenant_id)
);
```

Both tables follow the existing tenancy rules for package-scoped data.

### Polling + dispatch

For each enabled subscription:
1. `SELECT ... FROM reactor_firings WHERE tenant_id = ? AND reactor_name = ? AND fired_at > last_seen_fired_at ORDER BY fired_at`.
2. For each row, dispatch the workflow with `payload` as the input context (subject to the optional `@trigger(reactor=...)` filter function returning Fire).
3. Advance `last_seen_fired_at` atomically with the dispatch.

At-least-once semantics: if the process crashes between dispatch and watermark advance, the next poll will re-dispatch. This matches how cron triggers behave today.

### Retention

TTL-based prune (7 days default, configurable), consistent with other event-log retention in the codebase. The service loop sweeps `reactor_firings` older than the TTL on a fixed cadence. Pruning is not coupled to subscription watermarks — if a subscription is older than TTL, it will miss firings. Documented gotcha.

## Alternatives Considered

- **(A) In-process fanout channel (tokio broadcast).** Rejected: the CG traversal is already the in-process fast path; a second in-process subscription path is redundant and asymmetric with how cron triggers work. Lost durability across restarts.
- **(C) Reuse `/v1/ws/reactor/{name}` for firing broadcast.** Rejected for MVP: adds protocol surface and doesn't buy us anything today since all subscribers live in-process with the reactor. Revisit when S-0008 needs cross-process subscriptions.
- **Accumulator-level subscriptions.** Rejected: per S-0011, accumulators exist only as reactor inputs. The right abstraction level is the reactor's firing.
- **Subscription-driven prune (delete after all subscribers consumed).** Rejected for MVP: simpler TTL matches existing patterns; subscription-aware pruning can be added later if TTL turns out to be wrong.

## Testing Strategy

### Unit Testing
- DAL: insert-firing, poll-subscription-by-watermark, advance-watermark, TTL-prune.
- Dispatch logic: a fake firing + subscription produces exactly one workflow execution.
- Watermark advance is idempotent under re-poll.

### Integration Testing
- End-to-end: reactor fires → firing row written → subscription polls → workflow executes with the payload as context.
- Fan-out: two subscriptions on the same reactor both fire, independent watermarks.
- Tenancy: a subscription in tenant A cannot see firings from tenant B.
- Crash-recovery: dispatcher crash between dispatch and watermark advance → re-poll re-dispatches (at-least-once).
- TTL prune: old firings are removed; a subscription whose watermark predates the TTL is documented as losing those firings.

## Implementation Plan

Decompose into tasks during the decompose phase. Rough shape (to be refined):

1. **Schema + DAL** — migrations for `reactor_firings` + `reactor_trigger_subscriptions`, DAL methods (insert, poll, advance, prune). Covers both postgres and sqlite backends.
2. **Reactor wiring** — add the firing-row write to the reactor runtime, alongside the existing CG dispatch. Zero behavior change for graphs that don't have subscribers.
3. **Subscription polling service** — new service that reads subscriptions, polls firings, dispatches workflows, advances watermarks. Integrate with existing scheduler runtime.
4. **Trigger decorator + registration API** — extend `@trigger` with `reactor=` kwarg (Python + Rust registration), plus `subscribe_workflow_to_reactor(...)` entry point.
5. **TTL prune** — background sweep, configurable TTL, metrics.
6. **Tests** — unit + integration per the strategy above.
7. **Docs + tutorial** — how-to guide for event-driven workflows, API-reference additions for the new decorator kwarg and registration API.
