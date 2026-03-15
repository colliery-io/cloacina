---
id: fix-cursor-fullstate-watermark
level: task
title: "Fix Cursor/FullState watermark comparison — is_backward() and covers() are no-ops"
short_code: "CLOACI-T-0142"
created_at: 2026-03-15T14:39:29.291853+00:00
updated_at: 2026-03-15T14:45:01.846977+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fix Cursor/FullState watermark comparison — is_backward() and covers() are no-ops

## Objective

`is_backward()` in `watermark.rs` always returns `Ok(false)` for Cursor and FullState variants — the `_existing_val`/`_proposed_val` variables are unused. `covers()` only checks exact value match for these kinds. This means:
- Backward watermark movements are silently accepted for Cursor/FullState
- Late arrival detection doesn't work for Cursor/FullState (only exact matches detected)

**Location**: `crates/cloacina/src/continuous/watermark.rs` lines 118-140

### The Problem

Cursors ARE opaque to the framework, but S-0006 says they're user assertions. The question is: should the framework enforce monotonicity for Cursor/FullState, or should it trust the user? If trusting, the code should be explicit about that — not silently discard the values.

Options:
1. Accept that Cursor/FullState watermarks can't be compared — document clearly, remove the variables entirely (not just underscore them)
2. Add an optional user-provided comparator for Cursor ordering
3. Track Cursor/FullState watermarks by `emitted_at` timestamp for monotonicity (earlier timestamp = backward)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `is_backward()` for Cursor/FullState either implements real comparison or explicitly documents why it can't
- [ ] `covers()` for Cursor/FullState has semantics that make late arrival detection meaningful
- [ ] No underscore-prefixed "unused" variables — code is either used or removed
- [ ] Tests prove backward rejection works (or document that it intentionally doesn't)
- [ ] Tests prove late arrival detection works for Cursor/FullState boundaries

## Status Updates

**Decision**: Option 3 — use `emitted_at` timestamps for Cursor/FullState monotonicity and coverage.

**Changes**:
- Refactored `is_backward()` and `boundary_covered()` to take `&ComputationBoundary` (full struct) instead of `&BoundaryKind`
- **Cursor/FullState `is_backward()`**: rejects if `proposed.emitted_at < existing.emitted_at`. Same emitted_at = idempotent re-assertion = accepted.
- **Cursor/FullState `boundary_covered()`**: covered if `boundary.emitted_at <= watermark.emitted_at`. A boundary emitted before the watermark represents data from an earlier point in time — covered.
- No underscore-prefixed variables — all fields are now used or explicitly not destructured
- TimeRange/OffsetRange behavior unchanged (structural comparison on end positions)

**Tests (17 watermark tests)**:
- `test_advance_cursor_forward_accepted` — later emitted_at accepted
- `test_advance_cursor_backward_rejected` — earlier emitted_at rejected
- `test_advance_fullstate_forward_accepted` — later emitted_at accepted
- `test_advance_fullstate_backward_rejected` — earlier emitted_at rejected
- `test_advance_fullstate_same_emitted_at_accepted` — idempotent
- `test_covers_cursor_by_timestamp` — before/same/after watermark
- `test_covers_fullstate_by_timestamp` — before/after watermark
- All existing offset/time tests unchanged and passing
- 116 total tests (108 unit + 8 integration)
