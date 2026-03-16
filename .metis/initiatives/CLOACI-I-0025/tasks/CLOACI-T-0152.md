---
id: replace-std-mutex-rwlock-with
level: task
title: "Replace std Mutex/RwLock with parking_lot to eliminate lock poisoning"
short_code: "CLOACI-T-0152"
created_at: 2026-03-15T18:24:18.736848+00:00
updated_at: 2026-03-15T18:37:03.979089+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Replace std Mutex/RwLock with parking_lot to eliminate lock poisoning

**Priority: P0 — CRITICAL**
**Parent**: [[CLOACI-I-0025]]

## Objective

Replace all `std::sync::Mutex` and `std::sync::RwLock` usage in the continuous scheduling module with `parking_lot::Mutex`/`RwLock` to eliminate lock poisoning. Currently 92 `.unwrap()` calls on locks across scheduler.rs (51), ledger_trigger.rs (15), accumulator.rs (11), and watermark.rs (15). If any user-supplied task panics while holding a lock, the lock poisons and all subsequent acquisitions panic — cascading crash of the entire scheduler.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All `std::sync::Mutex` and `std::sync::RwLock` in `crates/cloacina/src/continuous/` replaced with `parking_lot` equivalents
- [ ] All `.unwrap()` calls on lock acquisitions are removed (parking_lot doesn't return `Result`, so `.lock()` returns the guard directly)
- [ ] `parking_lot` added to `Cargo.toml` dependencies
- [ ] No behavioral change — all existing tests pass
- [ ] Verify: a task that panics while holding a lock does NOT poison the lock for other threads

## Implementation Notes

- `parking_lot::Mutex::lock()` returns `MutexGuard` directly (no `Result`), so all `.unwrap()` calls simply become `.lock()`
- `parking_lot::RwLock` similarly — `.read()` and `.write()` return guards directly
- Key files: `scheduler.rs` (51), `ledger_trigger.rs` (15), `accumulator.rs` (11), `watermark.rs` (15)
- Also check `state_management.rs`, `connections/` for any lock usage
- This is a mechanical refactor — search-and-replace with verification

## Status Updates

### 2026-03-15 — Completed
- Replaced `std::sync::Mutex` → `parking_lot::Mutex` in: `graph.rs`, `ledger_trigger.rs`
- Replaced `std::sync::RwLock` → `parking_lot::RwLock` in: `scheduler.rs`, `accumulator.rs`, `ledger_trigger.rs`, `boundary.rs`
- Removed all 92 `.unwrap()` calls on lock acquisitions (parking_lot returns guards directly)
- Fixed `ledger_trigger.rs:97` where `.read().map_err(...)` was used for poison handling — removed entirely since parking_lot never poisons
- Fixed integration test file (`tests/integration/continuous/mod.rs`) — replaced fully-qualified `std::sync::RwLock::new(...)` with `parking_lot::RwLock::new(...)`
- `parking_lot` was already in `Cargo.toml` — no dependency change needed
- `std::sync::LazyLock` in `boundary.rs` stays as std (parking_lot has no equivalent)
- All 405 unit tests pass, zero compilation errors
