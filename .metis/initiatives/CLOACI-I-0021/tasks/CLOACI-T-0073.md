---
id: implement-slottoken-abstraction
level: task
title: "Implement SlotToken abstraction and TaskHandle with defer_until"
short_code: "CLOACI-T-0073"
created_at: 2026-01-29T02:02:25.397228+00:00
updated_at: 2026-01-29T02:12:45.996729+00:00
parent: CLOACI-I-0021
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0021
---

# Implement SlotToken abstraction and TaskHandle with defer_until

## Parent Initiative

[[CLOACI-I-0021]]

## Objective

Implement the SlotToken abstraction wrapping semaphore permit lifecycle, and TaskHandle which exposes `defer_until` to task functions. These are the core types that enable tasks to release concurrency slots while waiting on external conditions.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] SlotToken type with `release()` and `reclaim()` methods
- [ ] TaskHandle type holding SlotToken + task execution metadata
- [ ] `defer_until` method that releases slot, polls condition, reclaims slot
- [ ] Unit tests for SlotToken release/reclaim lifecycle
- [ ] Unit tests for defer_until behavior
- [ ] Types exported from executor module

## Implementation Notes

### Key files
- New: `crates/cloacina/src/executor/slot_token.rs` — SlotToken type
- New: `crates/cloacina/src/executor/task_handle.rs` — TaskHandle type
- Modified: `crates/cloacina/src/executor/mod.rs` — exports

### Dependencies
- T-0072 (semaphore in ThreadTaskExecutor) — completed

## Progress

- Created `slot_token.rs` with `SlotToken` type: `release()`, `reclaim()`, `is_held()`
  - 5 unit tests: release frees permit, reclaim reacquires, noop when held, drop releases, waits for availability
- Created `task_handle.rs` with `TaskHandle` type: `defer_until()`, `task_execution_id()`, `is_slot_held()`, `into_slot_token()`
  - 3 unit tests: releases and reclaims slot, immediate condition, frees slot for other tasks during defer
- Both types exported from `executor/mod.rs`
- All 8 new tests passing

## Status Updates

- **2026-01-29**: Completed. SlotToken and TaskHandle implemented with full test coverage.
