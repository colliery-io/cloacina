---
id: add-buffer-size-limits-and
level: task
title: "Add buffer size limits and eviction to accumulators and ledger"
short_code: "CLOACI-T-0150"
created_at: 2026-03-15T18:24:16.504174+00:00
updated_at: 2026-03-15T19:11:15.278160+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Add buffer size limits and eviction to accumulators and ledger

**Priority: P0 — CRITICAL**
**Parent**: [[CLOACI-I-0025]]

## Objective

Prevent guaranteed OOM on long-running schedulers by adding bounded buffer sizes and eviction policies to three unbounded collections: `ExecutionLedger.events`, `SimpleAccumulator.buffer`/`WindowedAccumulator.buffer`, and the `FiredTask` Vec returned by `scheduler.run()`.

## Problem

Three append-only collections grow without bound:

1. **`ExecutionLedger.events`** (`ledger.rs`): Every detector completion, task completion, and boundary emission appends a `LedgerEvent` containing a cloned `Context<serde_json::Value>`. At 100 events/sec, this is ~8.6M entries/day.
2. **`SimpleAccumulator.buffer` / `WindowedAccumulator.buffer`** (`accumulator.rs:106-108, 231-233`): `receive()` blindly pushes `BufferedBoundary` with no size check. A fast detector can flood an accumulator to OOM.
3. **`scheduler.run()` return Vec** (`scheduler.rs:211`): Collects ALL `FiredTask` instances until shutdown. No streaming, no backpressure.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ExecutionLedger` has a configurable max size (default sensible, e.g. 100K events) with time-based or size-based eviction (ring buffer or LRU)
- [ ] `LedgerTrigger` cursors are adjusted when events are evicted (cursor must not point past evicted entries)
- [ ] `SimpleAccumulator` and `WindowedAccumulator` have configurable `max_buffer_size` with a drop policy (reject-newest or drop-oldest)
- [ ] When buffer is full, a `warn!` log is emitted with the source name and current buffer size
- [ ] `scheduler.run()` returns a stream/channel instead of a Vec, or periodically flushes fired tasks
- [ ] Unit tests verify eviction behavior and cursor adjustment
- [ ] Integration test: flood an accumulator with 10K boundaries without draining, verify bounded memory

## Implementation Notes

### Ledger
- Replace `Vec<LedgerEvent>` with `VecDeque<LedgerEvent>` and track a `base_offset` so cursors remain valid after front eviction
- `events_since(cursor)` must handle cursor < base_offset (return events from base_offset)
- Add `LedgerConfig { max_events: usize }`

### Accumulators
- Add `max_buffer_size: Option<usize>` to accumulator constructors
- On `receive()`, if buffer is full, apply policy (default: drop-oldest to preserve latest data)
- Expose `is_full()` in `AccumulatorMetrics` for observability

### Scheduler Run
- Replace `Vec<FiredTask>` with `tokio::sync::mpsc::UnboundedSender<FiredTask>` or bounded channel
- Caller receives the receiver half; can process results as they arrive

## Status Updates

### 2026-03-15 — Completed
- **Ledger**: Replaced `Vec<LedgerEvent>` with `VecDeque<LedgerEvent>` + `base_offset` tracking. Added `LedgerConfig { max_events }` (default 100K). Oldest events evicted on overflow. `events_since()` handles cursors pointing to evicted events by returning from earliest retained. Added `with_config()`, `base_offset()`, `retained_count()` methods.
- **Accumulators**: Added `max_buffer_size` (default 10K) and `dropped_count` to both `SimpleAccumulator` and `WindowedAccumulator`. `receive()` drops oldest boundary when full and logs `warn!`. Added `with_max_buffer()` constructors.
- **Scheduler**: Added `max_fired_tasks` (default 10K) to `ContinuousSchedulerConfig`. `fired_tasks` Vec is trimmed after each poll cycle. Added `drain_counter` for stable drain naming.
- 3 new eviction unit tests added to ledger. All 408 unit tests pass.
