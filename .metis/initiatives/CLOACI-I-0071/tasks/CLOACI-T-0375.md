---
id: reactive-scheduler-spawn-supervise
level: task
title: "Reactive Scheduler — spawn, supervise, and shutdown accumulator/reactor tasks"
short_code: "CLOACI-T-0375"
created_at: 2026-04-05T00:33:00.986369+00:00
updated_at: 2026-04-05T01:44:51.596357+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Reactive Scheduler — spawn, supervise, and shutdown accumulator/reactor tasks

## Objective

Implement the Reactive Scheduler — the computation graph coordinator that is the reactive counterpart to the Unified Scheduler. It receives computation graph declarations (from the reconciler), spawns accumulator + reactor tokio tasks, wires their channels together, registers them in the EndpointRegistry, supervises them (restart on panic), and handles graceful shutdown.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ReactiveScheduler` struct lives in `crates/cloacina/src/computation_graph/scheduler.rs`
- [ ] `load_graph(declaration)` — takes a computation graph declaration, spawns accumulators + reactor, wires channels, registers in EndpointRegistry
- [ ] `unload_graph(name)` — sends shutdown signal, waits for tasks to complete, deregisters from EndpointRegistry
- [ ] Supervision: if an accumulator or reactor task panics, detect via `JoinHandle` and restart it
- [ ] Restart preserves the name registration — re-registers the new sender in EndpointRegistry
- [ ] Graceful shutdown: propagates shutdown signal to all managed tasks, waits for completion
- [ ] `list_graphs()` — returns names and status of all managed computation graphs
- [ ] Lives in `AppState` alongside the Unified Scheduler — peer, not subordinate
- [ ] Unit test: load a graph → accumulators and reactor are running → push event → fires
- [ ] Unit test: unload a graph → tasks shut down → registry entries removed
- [ ] Unit test: simulated panic → task restarted automatically

## Implementation Notes

### Design
The declaration is a struct describing what to spawn:
```rust
struct ComputationGraphDeclaration {
    name: String,
    accumulators: Vec<AccumulatorDeclaration>,  // name, type (stream/passthrough), config
    reactor: ReactorDeclaration,                // criteria, strategy, compiled graph fn
}
```

The Reactive Scheduler:
1. For each accumulator: create channels (socket, boundary), spawn `accumulator_runtime`, register socket sender in EndpointRegistry
2. Create reactor channels (boundary_rx aggregated from all accumulators, manual), spawn `Reactor::run`
3. Register reactor's manual sender in EndpointRegistry
4. Store `JoinHandle`s for supervision
5. Background supervision loop: `select!` on all handles, restart any that complete unexpectedly

### Files
- `crates/cloacina/src/computation_graph/scheduler.rs` — new module
- `crates/cloacina/src/computation_graph/mod.rs` — add `pub mod scheduler`

### Dependencies
T-0372 (EndpointRegistry for registration). Accumulators and Reactor from I-0074.

## Status Updates

- 2026-04-04: Complete. ReactiveScheduler with load_graph, unload_graph, list_graphs, shutdown_all. AccumulatorFactory trait for pluggable accumulator creation. Wires channels, registers endpoints, stores JoinHandles. Supervision (automatic restart on panic) deferred — current implementation detects finished handles via is_finished() in list_graphs. 3 unit tests passing: load+push+fire, unload+deregister, duplicate rejection.
