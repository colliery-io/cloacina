---
id: boundaryledger-with-source
level: task
title: "BoundaryLedger with source watermark tracking and coverage checks"
short_code: "CLOACI-T-0129"
created_at: 2026-03-15T13:14:08.488033+00:00
updated_at: 2026-03-15T13:26:28.442778+00:00
parent: CLOACI-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0024
---

# BoundaryLedger with source watermark tracking and coverage checks

## Parent Initiative

[[CLOACI-I-0024]]

## Objective

Implement `BoundaryLedger` — an in-memory store for per-data-source watermarks as specified in CLOACI-S-0006. The ledger tracks data completeness assertions from detectors and provides coverage checks used by `WindowedAccumulator`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BoundaryLedger` struct with `watermarks: HashMap<String, ComputationBoundary>`
- [ ] `advance(source, watermark)` — monotonic, rejects backward movement (returns error on backward)
- [ ] `covers(source, boundary) -> bool` — does the source watermark fully cover this boundary?
- [ ] `watermark(source) -> Option<&ComputationBoundary>` — get current watermark
- [ ] Coverage logic per BoundaryKind: TimeRange (end <= watermark.end), OffsetRange (end <= watermark.end), Cursor/FullState (value comparison)
- [ ] Thread-safe: behind `Arc<RwLock<>>` for concurrent reads from accumulators
- [ ] Unit tests: advance + covers for each BoundaryKind, backward rejection, missing source returns None

## Implementation Notes

### Technical Approach
- In `continuous/watermark.rs` (new file)
- Read-heavy, write-infrequent — `RwLock` is the right choice
- Monotonic enforcement: compare new watermark against existing, reject if it would go backward
- `covers()` semantics depend on BoundaryKind — TimeRange/OffsetRange check end positions, Cursor/FullState compare values

### Dependencies
- T-0117 (ComputationBoundary, BoundaryKind) from I-0023

## Status Updates

- Created `continuous/watermark.rs` with `BoundaryLedger`
- `advance()`: monotonic enforcement for TimeRange/OffsetRange (rejects backward end positions), Cursor/FullState always accept (opaque ordering)
- `covers()`: TimeRange/OffsetRange check end positions, Cursor/FullState require exact value match, different kinds return false
- `WatermarkError` enum: `BackwardMovement`, `IncompatibleKinds`
- 14 passing tests: advance (first/forward/backward/same/cursor/fullstate), covers (within/beyond/missing/cursor/fullstate/different kinds), watermark query, sources iterator
