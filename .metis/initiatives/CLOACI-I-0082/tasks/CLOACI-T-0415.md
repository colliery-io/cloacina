---
id: bug-fixes-sequence-race
level: task
title: "Bug fixes — sequence race, transactional deletes, shutdown ordering, field naming"
short_code: "CLOACI-T-0415"
created_at: 2026-04-06T01:05:47.535666+00:00
updated_at: 2026-04-06T01:39:30.421793+00:00
parent: CLOACI-I-0082
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0082
---

# Bug fixes — sequence race, transactional deletes, shutdown ordering, field naming

## Objective

Fix four correctness bugs and one naming issue that were found during the I-0082 review. These are independent fixes that can land as a single commit with no new features.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] BUG-1: `BoundarySender::send()` increments sequence number AFTER successful channel send, not before. Existing sequence number tests updated to verify.
- [ ] BUG-2: `CheckpointDAL::delete_graph_state` wraps all four DELETEs in a `conn.transaction()` block for both Postgres and SQLite variants.
- [ ] BUG-4: `serve.rs` shuts down the reactive scheduler BEFORE the axum server stops accepting connections (reverse current ordering).
- [ ] MINOR-1: `Reactor._input_strategy` field renamed to `input_strategy` (no underscore prefix — the field is used).
- [ ] All existing tests pass.

## Implementation Notes

### BUG-1: Sequence increment race (`accumulator.rs:253`)
```rust
// BEFORE (buggy):
self.sequence.fetch_add(1, Ordering::SeqCst);
self.inner.send((self.source_name.clone(), bytes)).await?;

// AFTER (correct):
self.inner.send((self.source_name.clone(), bytes)).await
    .map_err(|e| AccumulatorError::Send(...))?;
self.sequence.fetch_add(1, Ordering::SeqCst);
```

### BUG-2: Transactional deletes (`checkpoint.rs:849-866`)
Wrap the four `diesel::delete` calls in both `delete_graph_state_postgres` and `delete_graph_state_sqlite` in `conn.transaction(|conn| { ... })`.

### BUG-4: Shutdown ordering (`serve.rs:127-134`)
Use a `tokio::sync::watch` shutdown signal shared between the scheduler and the axum server. On SIGINT/SIGTERM: signal scheduler shutdown first, await completion, then stop accepting HTTP connections.

### MINOR-1: Field naming (`reactor.rs:228`)
Rename `_input_strategy` to `input_strategy` throughout `Reactor` struct and its constructor.

## Status Updates

*To be added during implementation*
