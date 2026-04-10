---
id: fix-accumulator-run-ownership
level: task
title: "Fix Accumulator::run() ownership — enable user-defined event loops"
short_code: "CLOACI-T-0416"
created_at: 2026-04-06T01:05:48.580770+00:00
updated_at: 2026-04-06T01:39:30.880428+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Fix Accumulator::run() ownership — enable user-defined event loops

## Objective

Fix the ownership problem that makes `Accumulator::run()` dead code. Currently the accumulator runtime moves `acc` into the processor task, so the event loop task can never call `acc.run()`. Any user-defined accumulator with an active event loop has its `run()` silently ignored.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Accumulator::run()` is actually called by the event loop task in `accumulator_runtime`
- [ ] A test accumulator with a custom `run()` that actively pushes events works end-to-end
- [ ] Existing passthrough/socket-only accumulators (default `run()` = pending) continue to work
- [ ] Polling and batch accumulator runtimes are not affected (they have their own runtime functions)
- [ ] All existing accumulator tests pass

## Implementation Notes

### The Problem (`accumulator.rs:298-383`)
The runtime spawns 3 tasks: event loop, socket receiver, processor. The processor owns `&mut acc` for `process()` calls. The event loop task needs `&mut acc` for `run()`. Rust's ownership model prevents both.

### Approach Options

**Option A — Split accumulator into two halves**: Separate `run()` into a standalone `EventSource` trait that doesn't need `&mut self` on the accumulator. The event source pushes raw events into the merge channel, the processor calls `process()`. This is clean but changes the trait API.

**Option B — Arc<Mutex<A>>**: Wrap the accumulator in `Arc<Mutex<A>>`. The event loop task locks to call `run()`, the processor locks to call `process()`. Since the merge channel serializes events, contention is minimal. Downside: Mutex overhead on every `process()` call.

**Option C — Move run() to own before processor starts**: Call `acc.run(ctx, event_tx)` in the event loop task, and move `acc` to the processor task only after `run()` completes. But `run()` is long-lived (it loops forever), so this doesn't work for concurrent operation.

**Option D — Separate run state from process state**: The `run()` method receives `&mut self` plus the event sender. Split the accumulator into `RunState` (for `run()`) and `ProcessState` (for `process()`), each owning their own data. Accumulator trait gets `fn split(self) -> (RunState, ProcessState)`.

Recommend **Option A** as the cleanest — it aligns with how stream accumulators already work (StreamBackend is a separate trait from Accumulator). The `run()` default on `Accumulator` becomes a separate `EventSource` trait with a blanket "no-op" implementation.

## Status Updates

*To be added during implementation*
