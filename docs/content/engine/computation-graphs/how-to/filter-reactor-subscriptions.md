---
title: "Filter Reactor Subscriptions (Python)"
description: "Attach a CEL predicate to a workflow's reactor subscription so only matching firings dispatch."
weight: 20
---

# Filter Reactor Subscriptions (Python)

When you subscribe a workflow to a reactor, every firing dispatches the workflow
by default. Pass a **CEL predicate** to fire only on matching firings — the
Python analog of [Filter reactor firings with CEL]({{< ref "/engine/computation-graphs/how-to/filter-reactor-firings-with-cel" >}}).

## Subscribe with a filter

`DefaultRunner.subscribe_workflow_to_reactor` takes an optional keyword-only
`when` argument — a CEL expression evaluated against each firing:

```python
import cloaca

runner = cloaca.DefaultRunner("sqlite:///app.db")

# Unfiltered — fires on every reactor firing:
runner.subscribe_workflow_to_reactor("pricing", "alert_workflow")

# Filtered — fires only when the predicate is true:
runner.subscribe_workflow_to_reactor(
    "pricing",
    "alert_workflow",
    when="payload.pricing.price > 100 && payload.pricing.region == 'us-east'",
)
```

Signature:

```python
subscribe_workflow_to_reactor(
    reactor: str,
    workflow: str,
    tenant: str | None = None,
    *,
    when: str | None = None,
) -> str
```

## What the predicate sees

The CEL expression evaluates against these variables:

| Variable | Type | Meaning |
|----------|------|---------|
| `payload` | dict | The firing's boundary data. Top-level keys are accumulator/source names; nested keys are that source's fields — e.g. for a `pricing` accumulator, `payload.pricing.price`. |
| `reactor` | str | The reactor name |
| `tenant` | str | The tenant |

## Semantics

- When the predicate evaluates **false**, the workflow is **not** dispatched, but
  the subscription's watermark still advances (the firing is consumed, not
  re-delivered).
- An invalid CEL expression is rejected immediately with a `ValueError` at
  subscribe time.

The predicate language, fail-closed evaluation, and idempotency behavior are
identical to the Rust path — see the
[Rust how-to]({{< ref "/engine/computation-graphs/how-to/filter-reactor-firings-with-cel" >}})
for the conceptual detail.

## See also

- [Filter Reactor Firings with CEL (Rust)]({{< ref "/engine/computation-graphs/how-to/filter-reactor-firings-with-cel" >}})
- [Subscription Fan-out]({{< ref "/engine/explanation/subscription-fan-out" >}})
