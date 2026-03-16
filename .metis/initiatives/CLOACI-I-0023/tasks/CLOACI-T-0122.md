---
id: executionledger-with-ledgerevent
level: task
title: "ExecutionLedger with LedgerEvent and cursor-based scanning"
short_code: "CLOACI-T-0122"
created_at: 2026-03-15T11:46:31.118193+00:00
updated_at: 2026-03-15T12:06:56.697828+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# ExecutionLedger with LedgerEvent and cursor-based scanning

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement `ExecutionLedger` with `LedgerEvent` enum and cursor-based scanning as specified in CLOACI-S-0007. The ledger is the in-memory append-only log that records all graph activity. The `ContinuousScheduler` writes to it; observers (and later `LedgerTrigger`) read from it.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ExecutionLedger` struct with `events: Vec<LedgerEvent>` (append-only)
- [ ] `LedgerEvent` enum: `TaskCompleted { task, at, context }`, `TaskFailed { task, at, error }`, `BoundaryEmitted { source, boundary }`, `AccumulatorDrained { task, boundary }`
- [ ] `append(event)` method for writing
- [ ] `events_since(cursor: usize) -> &[LedgerEvent]` for cursor-based scanning
- [ ] `len()` method returning current ledger size (cursor can use this)
- [ ] Thread-safe: `ExecutionLedger` behind `Arc<RwLock<>>` for concurrent reads/writes
- [ ] Unit tests: append + scan, cursor advancement, concurrent access

## Implementation Notes

### Technical Approach
- In `continuous/ledger.rs`
- Simple `Vec<LedgerEvent>` — append is O(1), scan from cursor is O(n) where n is new events
- No GC/compaction in this initiative — ledger grows unbounded (production hardening in I-0025)
- `LedgerTrigger` implementation deferred to I-0024, but the ledger API should support its read pattern

### Dependencies
- T-0117 (ComputationBoundary — used in LedgerEvent variants)

## Status Updates

- Created `continuous/ledger.rs`
- `ExecutionLedger` with append-only `Vec<LedgerEvent>`
- `LedgerEvent` enum: TaskCompleted, TaskFailed, BoundaryEmitted, AccumulatorDrained
- Removed `Clone` derive from `LedgerEvent` — `Context<Value>` doesn't impl Clone
- Cursor-based `events_since(cursor)`, `len()`, `get()`, `is_empty()`
- Helper methods: `task_name()`, `is_task_completed()`, `is_task_failed()`
- 5 passing tests: append+len, events_since, cursor advancement, event helpers, get
- Total continuous module: 47 passing tests
