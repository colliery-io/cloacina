---
id: tutorial-08-accumulators
level: task
title: "Tutorial 08: Accumulators — passthrough accumulator, runtime, boundary sender, channel wiring"
short_code: "CLOACI-T-0387"
created_at: 2026-04-05T13:36:42.542343+00:00
updated_at: 2026-04-05T14:01:30.407424+00:00
parent: CLOACI-I-0072
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0072
---

# Tutorial 08: Accumulators — passthrough accumulator, runtime, boundary sender, channel wiring

## Objective

Second computation graph tutorial. Introduces accumulators — the data sources that feed computation graphs. The user implements a passthrough accumulator, creates the runtime with explicit channel plumbing (socket channel, boundary channel, shutdown signal), spawns it as a tokio task, and pushes events through. Builds on Tutorial 07 by connecting the accumulator output to the compiled graph via a simple reactor.

## What the user learns
- `Accumulator` trait: `process()` method, `Event` and `Output` associated types
- `AccumulatorContext`, `BoundarySender`, `AccumulatorRuntimeConfig`
- `accumulator_runtime()` — the 3-task merge channel model
- `shutdown_signal()` for graceful shutdown
- Channel plumbing: `mpsc::channel` for socket, boundary, and shutdown
- Pushing serialized events via the socket channel
- How boundaries flow from accumulator → reactor → compiled graph

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example crate at `examples/tutorials/computation-graphs/library/08-accumulators/`
- [ ] Implements a `PricingAccumulator` (passthrough: receives `PricingUpdate`, emits `PricingData`)
- [ ] Creates all channels manually in `main()`: socket, boundary, shutdown
- [ ] Spawns `accumulator_runtime` as a tokio task
- [ ] Creates a `Reactor` with `WhenAny` + `Latest`, wires boundary_rx
- [ ] Pushes 3 pricing events via socket_tx, reactor fires graph 3 times
- [ ] Prints terminal output values showing the computation results
- [ ] Compiles and runs with `angreal demos tutorial-08`
- [ ] Docs page at `docs/content/tutorials/computation-graphs/library/08-accumulators.md`

## Implementation Notes

### Files
- `examples/tutorials/computation-graphs/library/08-accumulators/Cargo.toml`
- `examples/tutorials/computation-graphs/library/08-accumulators/src/main.rs`
- `docs/content/tutorials/computation-graphs/library/08-accumulators.md`

### Dependencies
T-0386 (Tutorial 07 — reuses the graph pattern, adds accumulator + reactor layer)

## Status Updates

- 2026-04-05: Example code complete and running. PricingAccumulator passthrough → reactor → graph. 3 events pushed, 3 fires observed. Explicit channel plumbing in main(). Docs page deferred.
