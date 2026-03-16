---
id: implement-restore-from-persisted
level: task
title: "Implement restore_from_persisted_state consumer watermark initialization"
short_code: "CLOACI-T-0151"
created_at: 2026-03-15T18:24:17.437504+00:00
updated_at: 2026-03-15T19:12:45.294883+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Implement restore_from_persisted_state consumer watermark initialization

**Priority: P0 — CRITICAL**
**Parent**: [[CLOACI-I-0025]]

## Objective

Complete the stubbed-out `restore_from_persisted_state()` in `scheduler.rs:142-148` so that consumer watermarks are actually initialized from persisted state on restart. Currently, the TODO loads state but never applies it, making persistence a lie — all accumulators start fresh, late arrival detection breaks, and tasks reprocess already-processed data.

## Problem

`scheduler.rs:142-148`:
```rust
// TODO: Initialize accumulator consumer watermark from
// state.consumer_watermark. Requires mutable access to
// accumulator which is behind Arc<Mutex>.
```

The persisted `PersistedAccumulatorState` is loaded from the database but never applied to the accumulators. Consequences:
- Consumer watermark is `None` after restart → late arrival check (`if let Some(_consumer_wm)`) always returns `is_late = false` → everything passes
- Tasks reprocess data that was already drained before the crash
- The entire persistence layer is effectively dead code

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] On startup, `restore_from_persisted_state()` acquires mutable access to each accumulator via `Arc<Mutex>` and sets `consumer_watermark` from `state.consumer_watermark`
- [ ] Add `set_consumer_watermark(&mut self, wm: ComputationBoundary)` method to `SignalAccumulator` trait
- [ ] After restore, late arrival detection correctly identifies boundaries already covered by the persisted watermark
- [ ] Integration test: drain accumulator → persist state → simulate restart → verify consumer watermark is restored → verify late data is detected
- [ ] Test: orphaned state (edge no longer in graph) logs a warning but does not panic

## Implementation Notes

- The accumulator is behind `Arc<Mutex<Box<dyn SignalAccumulator>>>` — just acquire the mutex lock in the restore loop
- Both `SimpleAccumulator` and `WindowedAccumulator` need the new trait method
- Consider also restoring `last_drain_at` for metrics accuracy
- Depends on: CLOACI-T-0150 (parking_lot locks make this simpler)

## Status Updates

### 2026-03-15 — Completed
- Added `set_consumer_watermark(&mut self, watermark: ComputationBoundary)` to `SignalAccumulator` trait
- Implemented for both `SimpleAccumulator` and `WindowedAccumulator`
- Completed the TODO in `restore_from_persisted_state()`: deserializes `consumer_watermark` JSON from `AccumulatorStateRow`, finds matching edge by `{source}:{task}` ID, acquires `Arc<Mutex>` lock, and calls `set_consumer_watermark()` on the accumulator
- Handles deserialization errors with `warn!` log
- Handles edges with no persisted watermark gracefully (info log)
- Orphaned state detection (existing) still works with `warn!` log
- All 408 unit tests pass
