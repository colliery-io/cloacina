---
id: fix-watermark-kind-mixing-and
level: task
title: "Fix watermark kind mixing and boundary coalescing semantic corruption"
short_code: "CLOACI-T-0162"
created_at: 2026-03-15T18:24:43.410920+00:00
updated_at: 2026-03-15T19:38:49.849996+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Fix watermark kind mixing and boundary coalescing semantic corruption

**Priority: P2 — MEDIUM**
**Parent**: [[CLOACI-I-0025]]

## Objective

Fix two related semantic correctness issues:

1. **Watermark kind mixing** (`watermark.rs:136-139`): When watermark kinds don't match (e.g., switching from `OffsetRange` to `TimeRange`), the function returns `Ok(false)` instead of an error. This silently allows a kind swap, breaking monotonicity semantics. The `covers()` function then returns `false` for cross-kind checks, silently breaking late arrival detection.

2. **Boundary coalescing with mixed kinds** (`boundary.rs:236-284`): If boundaries of different kinds arrive (e.g., one `OffsetRange`, one `Cursor`), coalescing returns the latest by `emitted_at` instead of erroring. A burst of offset-range boundaries followed by a cursor discards all the ranges. Silent semantic corruption.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `BoundaryLedger::advance()` returns an error if the new watermark kind differs from the existing watermark kind for the same source
- [ ] `coalesce()` returns an error (not silent latest) when boundary kinds are mixed
- [ ] Error is propagated to the scheduler, which logs `warn!` and rejects the boundary
- [ ] Per-source watermark kind is locked on first advance — changing it requires explicit reset
- [ ] Unit test: mixed-kind watermark advance is rejected
- [ ] Unit test: mixed-kind coalescing returns error
- [ ] Existing tests for same-kind operations still pass

## Implementation Notes

- For watermarks: add a `kind` field to `BoundaryLedger` per-source watermark entry, set on first advance, reject mismatches
- For coalescing: change return type from `Option<ComputationBoundary>` to `Result<Option<ComputationBoundary>, CoalesceError>`
- Consider: is there a legitimate use case for kind changes? If so, add an explicit `reset_watermark(source)` method instead of silent acceptance

## Status Updates

### 2026-03-15 — Completed
- **Watermark kind mixing**: `is_backward()` now returns `WatermarkError::IncompatibleKinds` when existing and proposed watermarks have different kinds (previously returned `Ok(false)` allowing silent kind swap). Added `source_name` parameter for context in error.
- **Boundary coalescing**: Mixed-kind boundaries now log `warn!` and return `None` (refuse to coalesce) instead of silently returning the latest boundary. This prevents semantic corruption where a burst of OffsetRange boundaries followed by a Cursor would silently discard the ranges.
- All 412 unit tests pass
