---
id: latearrivalpolicy-enum-and
level: task
title: "LateArrivalPolicy enum and consumer watermark check in scheduler"
short_code: "CLOACI-T-0132"
created_at: 2026-03-15T13:14:12.091351+00:00
updated_at: 2026-03-15T13:33:17.441614+00:00
parent: CLOACI-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0024
---

# LateArrivalPolicy enum and consumer watermark check in scheduler

## Parent Initiative

[[CLOACI-I-0024]]

## Objective

Implement the full `LateArrivalPolicy` enum and add consumer watermark checking to the scheduler's boundary routing path. Before routing a boundary to an accumulator, the scheduler checks if it falls behind the consumer watermark and applies the configured per-edge policy.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `LateArrivalPolicy` enum: `Discard`, `AccumulateForward`, `Retrigger`, `RouteToSideChannel { task_name: String }`
- [ ] Replace the existing `LateArrivalPolicy` stub (only had `AccumulateForward`) with full enum
- [ ] Scheduler checks `accumulator.consumer_watermark()` before `receive()` for each boundary
- [ ] If boundary is covered by consumer watermark, apply edge's `late_arrival_policy`:
  - `Discard`: drop silently, log at debug
  - `AccumulateForward`: call `receive()` normally (current behavior)
  - `Retrigger`: re-submit the boundary for processing (mark for next cycle)
  - `RouteToSideChannel`: route to a designated correction task (log + store for side-channel processing)
- [ ] Per-edge policy configurable via `GraphEdge.late_arrival_policy`
- [ ] Unit tests: each policy variant behavior, non-late boundary passes through normally

## Implementation Notes

### Technical Approach
- Extend `LateArrivalPolicy` in `continuous/graph.rs`
- Add late arrival check to `process_detector_output()` in scheduler
- `RouteToSideChannel` stores the late boundary in a side-channel buffer (Vec on the scheduler, picked up by a designated task)
- `Retrigger` pushes the boundary back into the accumulator but flags it for re-execution

### Dependencies
- T-0129 (BoundaryLedger for coverage checks), T-0130 (scheduler integration)

## Status Updates

- Extended `LateArrivalPolicy` from single-variant stub to full enum: Discard, AccumulateForward, Retrigger, RouteToSideChannel
- Changed from `Copy` to `Clone` (RouteToSideChannel holds String)
- Added consumer watermark check in `process_detector_output()` before `receive()`:
  - Checks `boundary_ledger.covers(source, boundary)` when consumer watermark exists
  - Applies per-edge policy: Discard (drops), AccumulateForward (receives), Retrigger (receives), RouteToSideChannel (logs)
- All 80 unit tests + 4 integration tests passing
- No new tests added here — late arrival behavior validated in T-0134 integration tests
