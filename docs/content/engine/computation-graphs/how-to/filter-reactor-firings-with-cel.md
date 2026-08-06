---
title: "Filter reactor firings with CEL"
description: "Attach a CEL predicate to a workflow's reactor subscription so only firings matching the predicate cause a workflow execution (CLOACI-T-0602)."
weight: 60
aliases:
  - "/computation-graphs/how-to-guides/filter-reactor-firings-with-cel/"

---

# Filter reactor firings with CEL

Reactors fire at the rate of their accumulators — every boundary, every source tick. For workflows subscribed to a reactor, that can mean far more workflow executions than the subscriber actually wants. CLOACI-T-0602 ships a **CEL predicate** on each `reactor_subscriptions` row: only firings where the predicate evaluates to `true` are dispatched to the workflow. Firings the predicate rejects (`false`) advance the watermark and are not retried; firings whose predicate *errors* are held and retried — see [Fail-closed evaluation](#fail-closed-evaluation-but-no-silent-data-loss).

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

Reactor subscriptions are a **runner-API surface only** — there is no
`package.toml` section and no HTTP route for managing them. Rust uses
`runner.subscribe_workflow_to_reactor(...)` as above; Python uses
`runner.subscribe_workflow_to_reactor(reactor=..., workflow=..., tenant=None, when=...)`.

### 2. Verify

Fire the reactor and confirm that only matching firings produce workflow
executions:

```sh
cloacinactl --profile prod execution list --tenant public
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

The predicate is **compiled once at subscribe time**. A malformed predicate errors at `subscribe_workflow_to_reactor` — you find out at registration, not on every firing. So does a predicate that references a variable outside `payload`, `reactor`, and `tenant` (CLOACI-T-0922): `tennant == 'acme'` is valid CEL but nothing binds `tennant`, so it would compile fine and then never match. Comprehension iteration variables are of course fine — `payload.items.exists(i, i.price > 100)` binds `i` itself.

The compiled predicate is **evaluated on every firing** during the subscription poll cycle. CEL evaluation is fast (microseconds for typical predicates), but it is not free — predicates that walk large nested payloads will add up across high-firing-rate reactors.

## Fail-closed evaluation, but no silent data loss

A predicate that **errors** at firing time (e.g., `payload.missing_field > 0` against a payload without `missing_field`, or an expression that returns a non-bool) never dispatches the workflow. Fail-closed is about dispatch only — the firing itself is **not** thrown away (CLOACI-T-0922 changed this; it used to be skipped *and* the watermark advanced, which destroyed the firing silently):

| Outcome | Dispatch? | Watermark |
|---|---|---|
| `true` | yes | advances after dispatch |
| `false` | no — filtered | advances (the firing was seen and rejected) |
| error | no — fail-closed | **held**; the firing is retried next poll tick |

Retries are bounded. After **5 consecutive errors on the same firing**, that firing is dead-lettered and the watermark advances, so a firing whose shape permanently breaks the predicate cannot wedge the subscription behind it.

What you can observe:

- `cloacina_reactor_predicate_errors_total{reactor}` — one increment per evaluation error.
- `cloacina_reactor_predicate_dead_letters_total{reactor}` — one increment per firing dropped at the bound.
- A `warn` log per error and an `error` log per dead-letter, each carrying the subscription id, firing id, and the (truncated) expression.
- On the `reactor_trigger_subscriptions` row, also returned by `list_reactor_subscriptions`: `predicate_degraded`, `predicate_error_count`, `predicate_error_firing_id`, `last_predicate_error`, `last_predicate_error_at`. `predicate_degraded` clears on the next successful evaluation; the `last_predicate_error*` fields are kept as history.

To avoid the error path altogether, guard the predicate so it always returns a bool:

```cel
has(payload.value) && payload.value > 100
```

`has()` returns a bool and never errors; the right-hand comparison only runs if the field exists. A guarded predicate degrades to `false` (skip + advance) instead of erroring — which is what you want when a missing field genuinely means "not interesting", and what you do **not** want when it means "something upstream broke".

## Idempotency key recipe

Filtered subscriptions are still **at-least-once** — see [Subscription fan-out]({{< ref "subscription-fan-out" >}}) for the failure modes. To make the workflow side idempotent, derive a key from the firing and write it through to the workflow's first task:

```rust
#[task]
pub async fn process_firing(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    let firing_id = ctx
        .get("reactor_firing_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TaskError::ValidationFailed {
            message: "context missing reactor_firing_id".to_string(),
        })?
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
