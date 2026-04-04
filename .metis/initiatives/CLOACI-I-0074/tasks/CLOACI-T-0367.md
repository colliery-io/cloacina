---
id: accumulator-trait-runtime-and
level: task
title: "Accumulator trait, runtime, and BoundarySender"
short_code: "CLOACI-T-0367"
created_at: 2026-04-04T22:54:46.438564+00:00
updated_at: 2026-04-04T23:03:02.162592+00:00
parent: CLOACI-I-0074
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0074
---

# Accumulator trait, runtime, and BoundarySender

## Objective

Implement the core `Accumulator` trait, the 3-task merge channel runtime, `AccumulatorContext`, and `BoundarySender`. This is the foundation that all accumulator classes (stream, passthrough, etc.) build on.

Spec: CLOACI-S-0004.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Accumulator` trait with `process()`, `run()`, `init()` as defined in S-0004
- [ ] `AccumulatorContext` struct: `BoundarySender`, name, `CheckpointHandle`, shutdown watch channel
- [ ] `BoundarySender`: serializes via bincode (release) / JSON (debug), sends `(SourceName, Vec<u8>)` over mpsc
- [ ] `accumulator_runtime()` function: spawns 3 tokio tasks (event loop, socket receiver, processor) connected by merge channel
- [ ] `process()` called sequentially by processor task — no concurrent `&mut self` access
- [ ] Event loop task and socket receiver task run independently, never block each other
- [ ] Merge channel bounded (backpressure) with configurable capacity
- [ ] Shutdown signal (watch channel) cleanly stops all 3 tasks
- [ ] Unit tests: BoundarySender serialization round-trip, merge channel ordering, shutdown behavior

Place in `crates/cloacina/src/computation_graph/accumulator.rs` (or `accumulator/` module).

### Dependencies
T-0362 (InputCache, SourceName — already done).

## Status Updates

**2026-04-04**: Completed.
- Created `computation_graph/accumulator.rs` (~350 lines)
- `Accumulator` trait: `process()`, `run()`, `init()` with async_trait
- `AccumulatorContext`: BoundarySender, name, shutdown watch channel (CheckpointHandle deferred to I-0072)
- `BoundarySender`: dual-format serialize (bincode/JSON), sends (SourceName, Vec<u8>) over mpsc
- `accumulator_runtime()`: 3-task model — event loop (waits for shutdown), socket receiver (deserializes + pushes to merge), processor (calls process() sequentially)
- `AccumulatorRuntimeConfig` with configurable merge_channel_capacity (default 1024)
- `shutdown_signal()` helper creates watch channel pair
- 5 unit tests passing: sender round-trip, socket event processing, multiple events in order, shutdown, process() returning None (filtering)
