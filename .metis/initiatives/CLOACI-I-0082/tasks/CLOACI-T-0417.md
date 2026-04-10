---
id: scheduler-wiring-dal-health-graph
level: task
title: "Scheduler wiring — DAL, health, graph name, and expected sources into reactor and accumulators"
short_code: "CLOACI-T-0417"
created_at: 2026-04-06T01:05:49.913615+00:00
updated_at: 2026-04-06T09:05:59.312535+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Scheduler wiring — DAL, health, graph name, and expected sources into reactor and accumulators

## Objective

Wire the DAL, health channels, graph name, and expected sources through the ReactiveScheduler into the Reactor and AccumulatorFactory so that all persistence and health features actually function in the production code path. This is the single highest-impact task in I-0082 — it unblocks every other resilience feature.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ReactiveScheduler` accepts a DAL handle (passed at construction or via `load_graph`)
- [ ] `load_graph()` calls `.with_dal(dal)`, `.with_graph_name(name)`, `.with_health(health_tx)`, and `.with_expected_sources(sources)` on the Reactor
- [ ] `AccumulatorFactory::spawn()` trait signature extended to accept `Option<DAL>` and `Option<watch::Sender<AccumulatorHealth>>`
- [ ] All factory implementations updated: `TestAccumulatorFactory` (scheduler tests), `PassthroughAccumulatorFactory` (packaging_bridge), and any others
- [ ] Factories create `AccumulatorContext` with `checkpoint: Some(CheckpointHandle::new(...))` and `health: Some(health_tx)` when DAL is available
- [ ] `load_graph()` creates health channels per accumulator and calls `registry.register_accumulator_health()` for each
- [ ] `load_graph()` creates a `ReactorHealth` channel and stores/exposes it
- [ ] Full-graph restart in `check_and_restart_failed()` also re-wires DAL/health/sources (not just channels)
- [ ] All existing scheduler and integration tests pass

## Implementation Notes

### Scheduler changes (`scheduler.rs`)
1. Add `dal: Option<DAL>` field to `ReactiveScheduler`
2. `new()` accepts `Option<DAL>` parameter
3. In `load_graph()`:
   - Extract accumulator names from declaration for `with_expected_sources`
   - Create `reactor_health_channel()` and pass sender via `.with_health()`
   - Pass DAL via `.with_dal()` and graph name via `.with_graph_name()`
   - For each accumulator: create `health_channel()`, pass to factory, register with endpoint registry

### AccumulatorFactory trait change (`scheduler.rs:68-73`)
```rust
// BEFORE:
fn spawn(&self, name: String, boundary_tx: ..., shutdown_rx: ...) -> (...);

// AFTER:
fn spawn(
    &self,
    name: String,
    boundary_tx: mpsc::Sender<(SourceName, Vec<u8>)>,
    shutdown_rx: watch::Receiver<bool>,
    dal: Option<DAL>,
    health_tx: Option<watch::Sender<AccumulatorHealth>>,
    graph_name: String,
) -> (mpsc::Sender<Vec<u8>>, JoinHandle<()>);
```

### Files to update
- `crates/cloacina/src/computation_graph/scheduler.rs` — scheduler struct, load_graph, check_and_restart_failed
- `crates/cloacina/src/computation_graph/packaging_bridge.rs` — PassthroughAccumulatorFactory
- `crates/cloacinactl/src/commands/serve.rs` — pass DAL to scheduler constructor
- Integration tests that create schedulers/factories

## Status Updates

*To be added during implementation*
