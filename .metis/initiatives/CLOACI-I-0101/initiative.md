---
id: decouple-computation-graph-from
level: initiative
title: "Decouple computation graph from reactor; enable embedded CG workflow tasks"
short_code: "CLOACI-I-0101"
created_at: 2026-04-23T23:44:55.541551+00:00
updated_at: 2026-04-24T15:13:17.470059+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: decouple-computation-graph-from
---

# Decouple computation graph from reactor; enable embedded CG workflow tasks

## Context

Today `#[computation_graph]` bundles three things together: the graph's compute topology, its accumulator inputs, and its reactor (firing criteria + action). There is no way to declare a graph without a reactor, which makes CLOACI-S-0011's *embedded* computation graph model (a CG as a workflow task, "semantically subsumed by the workflow, without its own reactor or standalone trigger surface") awkward to express — you'd pay the declaration cost for reactor/accumulator machinery you never use.

This initiative does the macro surgery to separate the graph from its reactor, land the matching runtime model, and deliver the embedded-CG-as-workflow-task capability (superseding CLOACI-T-0500).

Under the new model, every primitive in the system declares its *upstream*; reactors stay standalone and unaware of subscribers.

## Goals & Non-Goals

**Goals:**
- Split the `#[computation_graph]` macro so graph topology is declared independently of the reactor.
- Let a computation graph declare its trigger upstream directly: `trigger = reactor("name")`. No declaration changes on the reactor side.
- Let a workflow task invoke a computation graph: `invokes = computation_graph("name")`. Runs the compiled graph with the task's input context; terminal node outputs become the task's output context.
- Preserve the existing low-latency in-process reactor→graph fast path. The split is a declaration refactor, not a delivery change.
- Migrate all in-tree examples, tutorials, and tests to the new declaration. External users re-author against the new form; pre-1.0 breaking change, no shim.

**Non-Goals:**
- Workflow triggers driven by reactors (that's CLOACI-I-0100; will land in parallel and share the reactor publish surface but has its own decomposition).
- Multi-reactor-per-graph topologies. The new model permits multiple graphs subscribing to one reactor (natural fan-out); the inverse (one graph with multiple reactor triggers) is out of scope.
- Accumulator lifecycle changes. Accumulators remain reactor inputs only.
- CGs triggered by something other than a reactor in standalone mode. If a user wants a cron-triggered or manually-triggered CG without a reactor, they wrap it in a workflow and use a task that invokes the graph.
- Changes to how the reactor publishes firings to its in-process CG subscriber; that plumbing stays.

## Primitive model recap (from S-0011)

- **Reactor**: standalone. Declares accumulators + firing criteria. Publishes firings. Does not know about subscribers.
- **Computation graph**: a single primitive with one optional declaration clause — `trigger = reactor("name")`.
  - **With the clause**: the graph subscribes to its upstream reactor; reactor firings invoke it in-process (existing fast path).
  - **Without the clause**: the graph is invoked on demand. The only caller today is a workflow task that wraps it via `invokes = computation_graph("name")`.
- **Workflow task invoking a CG**: an ordinary workflow task whose body is a CG invocation — the graph runs as a deterministic function, input context in, output context out, one shot per task execution. This is what "wrapping a CG as a workflow task node and executing on demand" means in practice.

There is no separate "library graph" class. All CGs are declared the same way; the optional `trigger` clause is what determines whether the runtime subscribes to a reactor on the graph's behalf or waits for an explicit `invokes` caller. Subscribers declare upstream; publishers (reactors and trigger-less graphs) have no downstream coupling.

## Architecture

### Declaration surface (proposal — open for revision)

Reactor (unchanged):
```rust
#[reactor(
    name = "pricing_reactor",
    accumulators = [...],
    criteria = when_any(...),
    // no graph reference — reactor is standalone
)]
fn fire_pricing(input: PricingInput) -> PricingFiring { ... }
```

Standalone graph (references its trigger):
```rust
#[computation_graph(
    name = "pricing_graph",
    trigger = reactor("pricing_reactor"),
    entry_type = PricingFiring,
    nodes = [...],
)]
```

Trigger-less graph (no `trigger` clause; invoked on demand by a workflow task):
```rust
#[computation_graph(
    name = "batch_pricing_graph",
    entry_type = BatchPricingInput,
    nodes = [...],
)]
```

Workflow task that invokes a CG:
```rust
#[task(
    id = "price_batch",
    depends_on = ["fetch_batch"],
    invokes = computation_graph("batch_pricing_graph"),
)]
async fn price_batch(ctx: &mut Context<Value>) -> Result<(), TaskError> {
    // Body is auto-generated by the macro: unpack ctx -> graph entry type,
    // run the compiled graph, pack terminal outputs -> ctx. User code here
    // runs before/after the invocation if they want pre/post-processing,
    // or the body may be omitted entirely for the pure-invocation case.
}
```

Python surfaces mirror the Rust declarations — `@computation_graph(trigger=reactor("name"))`, `@task(invokes=computation_graph("name"))`.

### Compile-time type checking

- `trigger = reactor("name")` binds the graph's `entry_type` to the reactor's firing output type. Macro generates an assertion that the types match; mismatch = compile error.
- `invokes = computation_graph("name")` on a workflow task binds the graph's `entry_type` to the task's input context shape. Context is a `Context<Value>` bag today, so the adapter is runtime (deserialize required fields, serialize outputs). Typed context bridges are a future concern.

### Runtime paths

- **Standalone graph**: today's in-process path, preserved. When the reactor's criteria fire, it directly invokes the bound graph's compiled function. No DB hop.
- **Embedded (workflow-task) invocation**: the workflow task executor resolves the graph name via `ComputationGraphScheduler` (or a lighter-weight graph registry), calls the compiled graph function, captures terminal outputs into context, marks the task complete. Task retry/timeout/heartbeat apply as normal — if the graph panics or times out, the task fails and the workflow's retry policy takes over.
- **Reactor publishing**: unchanged for graph subscribers. I-0100 adds a second publish path (DB firing-log) for workflow-trigger subscribers; this initiative does not touch that.

### Breaking change (no shim)

The old bundled `#[computation_graph]` form is removed as part of this initiative. Pre-1.0 breakage policy: every in-tree user (examples, tutorials, tests, stream-accumulator crate, server integration tests) is migrated in the same PR as the macro change. External users see a compile-time error with a migration message pointing at the new declaration form. No deprecation window, no compatibility shim to maintain.

Release notes call this out as a breaking change in whichever minor release I-0101 lands.

## Alternatives Considered

- **Keep the bundled macro, mark reactor as optional**: half-measure; still leaks accumulator/reactor concepts into the embedded case. Rejected.
- **Separate macros but workflow-task references the graph by embedding the declaration inline**: every workflow would carry its own copy of the graph. Rejected — named packages/registries are how CGs ship today; embedded use should reuse that.
- **Workflow-task invoking a CG through the reactor firing path (i.e., pretend the workflow task publishes a reactor firing and subscribes to it)**: too clever, conflates standalone and embedded models, contradicts S-0011's "subsumed by the workflow" language.

## Testing Strategy

### Unit Testing
- Macro expansion for each of the three forms (CG with trigger, CG without trigger, workflow task with `invokes`).
- Compile-time type checking between reactor firing output and graph entry type.
- Compatibility shim: old bundled form produces an identical runtime to the split form.

### Integration Testing
- CG with a reactor trigger: end-to-end behavior matches today's reactor + CG test suite. Zero regressions.
- Trigger-less CG invoked by a workflow task: workflow execution runs the graph, terminal outputs land in downstream task context.
- Context ↔ graph-types adapter: typed entry fields, error on missing, terminal outputs serialized back.
- Failure modes: graph panic ⇒ task fails ⇒ workflow retry policy engages.
- Fan-out: two CGs both declaring `trigger = reactor("R")` — both invoked on each firing of R. Shipping as part of this initiative per the locked open-question.
- Mixed-package loading: a single workflow package containing both workflow declarations and a trigger-less CG — reconciler routes declarations independently.

## Implementation Plan

Rough decomposition (to be refined in the decompose phase):

1. **Macro split + IR** — parse the new clauses (`trigger = reactor(...)`, standalone without trigger, `invokes = computation_graph(...)`), emit separated declarations, internal IR for graph-without-reactor.
2. **Compile-time type binding** — verify reactor firing output ↔ graph entry type at macro expansion.
3. **Runtime registry changes** — `ComputationGraphScheduler` (or a shared graph registry) exposes a compiled-graph-function lookup usable by both the reactor runtime and the workflow task executor. Separate graph-from-reactor in the internal data model.
4. **Workflow task executor — CG invocation kind** — new task kind; resolves graph by name, adapts context in/out, invokes compiled graph.
5. **Migrate in-tree callers** — examples, tutorials, tests, stream-accumulator crate, server integration tests. Done in the same PR as the macro change; the branch doesn't leave the old form working at any commit.
6. **Python parity** — mirror the Rust decorator changes in `@computation_graph` / `@task`.
7. **Docs** — update CG tutorials to the new declaration form; add a how-to guide for wrapping a CG as a workflow task node.
8. **Breaking-change notes** — release notes call out the bundled form removal and point at the new declaration; a docs pass catches any remaining bundled-form examples.

CLOACI-T-0500 is subsumed: the embedded-CG-as-workflow-task capability is the payoff task of this initiative (likely rolled into step 4 + its tests + its docs slice).

## Locked decisions (2026-04-24)

- **Declaration syntax.** `trigger = reactor("name")` on the graph, `invokes = computation_graph("name")` on the workflow task. Parallel verb/noun structure; leaves room to widen what can appear on the right side of `trigger =` if a future CG ever wants a cron or event upstream without a reactor in the loop.
- **Multi-graph fan-out on one reactor.** Shipping as part of this initiative. Under the decoupled declaration, multiple CGs can declare `trigger = reactor("R")` and all are invoked on each firing of R. The decoupling was the whole point; artificially restricting fan-out would re-impose the 1:1 coupling in a different place. S-0011 has been amended (2026-04-24 changelog) to reflect this.
- **Package shape.** A single package may contain workflows, CGs with a reactor trigger, CGs without a trigger, and reactors. The reconciler routes each declaration by kind at load time; no separate "CG package" vs. "workflow package" distinction is required. The primitive model is about declaration shape, not packaging.
- **No compatibility shim.** The old bundled `#[computation_graph]` form is removed in the same PR that lands the split. Pre-1.0 breaking change; in-tree callers are migrated atomically, external users re-author against the new form. Keeps the macro surgery honest and avoids dual-form maintenance.
