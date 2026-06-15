---
title: "Filter reactor firings with CEL"
description: "Attach a CEL predicate to a workflow's reactor subscription so only firings matching the predicate cause a workflow execution (CLOACI-T-0602)."
weight: 60
aliases:
  - "/computation-graphs/how-to-guides/filter-reactor-firings-with-cel/"

---

# Filter reactor firings with CEL

Reactors fire at the rate of their accumulators — every boundary, every source tick. For workflows subscribed to a reactor, that can mean far more workflow executions than the subscriber actually wants. CLOACI-T-0602 ships a **CEL predicate** on each `reactor_subscriptions` row: only firings where the predicate evaluates to `true` are dispatched to the workflow. Filtered-out firings advance the watermark and are not retried.

This is the surgical alternative to "subscribe to the reactor, filter in the workflow's first task" — the filter runs in the dispatcher before any workflow row is inserted.

## Prerequisites

- A reactor that fires (declared as `#[reactor(...)]` or via the runtime API).
- A workflow with a `#[trigger]` that subscribes to that reactor — see [Subscribe a workflow to a reactor]({{< ref "/embed/how-to/subscribe-workflow-to-reactor" >}}).
- Familiarity with CEL ([cel-spec](https://github.com/google/cel-spec)). The variant Cloacina uses is [`cel-rust`](https://crates.io/crates/cel) — a subset close to the spec.

## Steps

### 1. Subscribe with a predicate

Predicates live on the subscription row, not the workflow declaration. Subscribe via the runner API and pass the CEL expression in the fourth argument:

```rust
runner
    .subscribe_workflow_to_reactor(
        "pricing_reactor",       // reactor name
        "alert_workflow",        // workflow name
        Some("public"),          // tenant ID (or None for default)
        Some("payload.value > 100"),  // CEL predicate
    )
    .await?;
```

Pass `None` as the fourth argument for an unfiltered subscription (every firing dispatches). The `subscribe_workflow_to_reactor` call is idempotent on `(reactor_name, workflow_name, tenant_id)` — re-subscribing replaces the predicate.

### 2. (Alternative) Register via configuration / package metadata

If the subscription lives in a packaged workflow's manifest, declare it there — the `subscribe_workflow_to_reactor` call is invoked by the package loader on registration. Consult the package's `package.toml` reactor-subscriptions section for the per-package surface.

### 3. Verify

```sh
cloacinactl --profile prod trigger list --tenant public --workflow alert_workflow
```

The trigger row's metadata should show the predicate string. Once the reactor fires, only matching firings produce workflow executions:

```sh
cloacinactl --profile prod execution list --tenant public --workflow alert_workflow
```

For the runnable end-to-end version of this exact recipe (insert four firings with values `[50, 150, 80, 200]` and see two `alert_workflow` rows), run:

```sh
angreal demos features filtered-reactor
```

The example source is at `examples/features/computation-graphs/filtered-reactor/`.

## CEL variables

The predicate is evaluated against a context with three top-level keys:

| Variable | Type | Notes |
|---|---|---|
| `payload` | object | The reactor firing's payload — top-level keys are the reactor's accumulator source names; values are JSON-decoded boundary values. |
| `reactor` | object | Metadata about the firing reactor. `reactor.name` is always populated. |
| `tenant` | string | The tenant the firing is scoped to. |

Example predicates:

```cel
payload.value > 100
```

Fire only when the `value` source's latest boundary exceeds 100.

```cel
payload.symbol == "BTC" && payload.price > 50000
```

Fire only for BTC pricing events above a threshold (assumes the reactor has `symbol` and `price` accumulator sources).

```cel
tenant == "prod" && reactor.name == "pricing_reactor"
```

A trivially-evaluable example. Useful as a smoke test — should match every firing for that tenant.

```cel
has(payload.user_id) && size(payload.actions) > 5
```

Check field presence + collection size before dispatching.

## Compile time vs evaluation time

The predicate is **compiled once at subscribe time**. A malformed predicate (syntax error, reference to an undeclared identifier the compiler can see) errors at `subscribe_workflow_to_reactor` — you find out at registration, not on every firing. Predicate-parse errors surface as `Error::CelParse(...)`.

The compiled predicate is **evaluated on every firing** during the subscription poll cycle. CEL evaluation is fast (microseconds for typical predicates), but it is not free — predicates that walk large nested payloads will add up across high-firing-rate reactors.

## Fail-closed evaluation

If the predicate **panics or returns a non-bool result** at firing time (e.g., `payload.missing_field > 0` against a payload without `missing_field`), evaluation is treated as **`false`**, the firing is skipped, and the error is logged. The watermark advances. This is deliberate — a predicate bug should not block the subscription's progress.

If you need strict matching (any evaluation error should *halt* the subscription), the predicate itself must guard:

```cel
has(payload.value) && payload.value > 100
```

`has()` returns a bool and never errors; the right-hand comparison only runs if the field exists.

## Idempotency key recipe

Filtered subscriptions are still **at-least-once** — see [Subscription fan-out]({{< ref "subscription-fan-out" >}}) for the failure modes. To make the workflow side idempotent, derive a key from the firing and write it through to the workflow's first task:

```rust
#[task(id = "process_firing")]
pub async fn process_firing(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    let firing_id = ctx
        .get("reactor_firing_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TaskError::missing("reactor_firing_id"))?
        .to_string();

    // Upsert keyed on firing_id — a second delivery is a no-op.
    upsert_alert(&firing_id, &payload).await?;
    Ok(())
}
```

The `reactor_firing_id` is automatically populated in the task context for reactor-dispatched workflows. Combine it with a unique constraint or upsert on the downstream side to make every delivery a no-op.

## What this how-to does NOT cover

- **Authoring the workflow trigger.** See [Subscribe a workflow to a reactor]({{< ref "/embed/how-to/subscribe-workflow-to-reactor" >}}).
- **CEL language semantics in depth.** See the [cel-spec](https://github.com/google/cel-spec) and [`cel-rust`](https://docs.rs/cel/) docs.
- **Filtering on the in-process CG fast path.** Filtering is a subscription-table concept — in-process `#[computation_graph(trigger = reactor("..."))]` declarations always see every firing.

## See also

- [Subscribe a workflow to a reactor]({{< ref "/embed/how-to/subscribe-workflow-to-reactor" >}}) — the subscription side without filtering.
- [Subscription fan-out]({{< ref "subscription-fan-out" >}}) — durability and at-least-once semantics.
- `examples/features/computation-graphs/filtered-reactor/` — runnable end-to-end.
- **CLOACI-T-0602** — CEL predicate filtering on subscriptions.
- **CLOACI-I-0100** — DB-backed reactor → workflow subscription fan-out (parent).
