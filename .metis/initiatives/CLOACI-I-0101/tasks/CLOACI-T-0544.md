---
id: t-02b-multi-graph-per-reactor-fan
level: task
title: "T-02b: Multi-graph-per-reactor fan-out"
short_code: "CLOACI-T-0544"
created_at: 2026-04-25T15:08:05.348610+00:00
updated_at: 2026-04-25T15:08:05.348610+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-02b: Multi-graph-per-reactor fan-out

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Lift the current "one reactor instance per graph" constraint in `ComputationGraphScheduler` so a single `#[reactor]` declaration can fan out to multiple `#[computation_graph(trigger = reactor(R))]` subscribers. Today's `load_graph_split` builds a fresh reactor instance per graph load (carried over from the bundled-form era — see T-0543 M5 status note); this task adds a shared-reactor binding path so one firing of R invokes every graph subscribed to R, rather than just one.

This was originally folded into T-0540's acceptance criteria (the fan-out integration test), but the runtime change is independent of the workflow-task `invokes = computation_graph(...)` macro work, so it lives on its own.

## Acceptance Criteria

- [ ] Scheduler exposes a way to register a reactor once and bind multiple graphs to it (rather than the current 1:1 `load_graph_split`). Concrete shape: e.g. `load_reactor(ReactorRegistration, accumulator_factories) -> ReactorHandle` + `bind_graph_to_reactor(graph_name, reactor_name, graph_fn)` — exact API to be settled during implementation.
- [ ] On firing, every graph bound to the reactor receives the same `InputCache` and runs (sequentially or concurrently — to be decided). Failure of one graph does not block siblings.
- [ ] Existing single-graph split-form path stays green (either kept as a thin wrapper over the new API, or migrated to it).
- [ ] Integration test (sqlite + postgres): two graphs declare `trigger = reactor(R)`; pushing one event to R's accumulator fires both graphs, both terminal outputs observed.
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` + `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. Audit current `ComputationGraphScheduler` — `load_graph_split` constructs the reactor + accumulators + graph binding in one shot. Identify what needs to split: probably the reactor lifecycle (accumulator factories + reactor loop) becomes a standalone object, and graph subscription becomes a separate operation.
2. Decide concurrency model on firing — sequential (simpler, deterministic) or concurrent via tokio::join (more throughput, harder error handling). Default to sequential unless a benchmark says otherwise.
3. Inventory side: `ReactorEntry` is already separate from `ComputationGraphEntry`; the runtime seeding step in `Runtime::seed_from_inventory` may need to register reactors first, then bind subscribed graphs in a second pass.
4. Update integration tests in `crates/cloacina/tests/integration/computation_graph.rs` to cover the fan-out shape.

### Key Files

- `crates/cloacina/src/computation_graph/scheduler.rs` — primary surface change.
- `crates/cloacina/src/runtime.rs` — seed-from-inventory may need a two-pass change.
- `crates/cloacina/tests/integration/computation_graph.rs` — fan-out test.

### Dependencies

- T-0539 (split form is the only form on main; bundled form is removed). Already done.

### Risk Considerations

- **Failure isolation**: one graph panic must not poison the reactor or its other subscribers. Wrap each graph invocation in catch_unwind or run in an isolated task per subscriber.
- **Ordering guarantees**: nothing in the docs promises subscriber ordering today. Pick one (declaration order? alphabetical?) and document it.
- **Cancellation**: T-0487 cooperative cancellation needs to apply per-subscriber, not at the reactor level.

## Status Updates

*To be added during implementation.*
