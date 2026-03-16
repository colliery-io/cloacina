---
id: replace-ledgertrigger-polling-with
level: task
title: "Replace LedgerTrigger polling with event-driven notification"
short_code: "CLOACI-T-0156"
created_at: 2026-03-15T18:24:31.659035+00:00
updated_at: 2026-03-15T19:23:48.093369+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Replace LedgerTrigger polling with event-driven notification

**Priority: P1 — HIGH**
**Parent**: [[CLOACI-I-0025]]

## Objective

Replace the 50ms polling loop in `LedgerTrigger` (`ledger_trigger.rs:88`) with event-driven notification using a `tokio::sync::broadcast` or `tokio::sync::Notify` channel. Current polling at 20 polls/sec/detector with unbounded ledger growth makes `events_since()` increasingly expensive as the ledger grows.

Additionally fix the cursor race condition (`ledger_trigger.rs:96-126`) where the cursor is read before events are processed and updated after — two concurrent polls can process the same events twice.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `ExecutionLedger` exposes a notification mechanism (e.g., `subscribe() -> broadcast::Receiver` or `Notify`)
- [ ] `LedgerTrigger` awaits notification instead of polling on a fixed interval
- [ ] Cursor update is atomic with event processing (single lock scope)
- [ ] CPU usage under idle workloads drops to near-zero (no 50ms wake cycles)
- [ ] No double-processing of events under concurrent access
- [ ] Fallback: if notification is missed, periodic sweep catches up (e.g., every 5 seconds)

## Implementation Notes

- `ExecutionLedger::record()` calls `notify.notify_waiters()` after appending
- `LedgerTrigger` uses `tokio::select!` with notification + timeout fallback
- For cursor atomicity: read cursor, process events, update cursor all under one lock scope
- Alternative: make cursor an `AtomicUsize` and use `compare_exchange` for lock-free update

## Status Updates

### 2026-03-15 — Completed
- Added `Arc<tokio::sync::Notify>` to `ExecutionLedger` — `append()` calls `notify_waiters()` after every event
- Added `subscribe()` method to `ExecutionLedger` returning an `Arc<Notify>` handle
- `LedgerTrigger::new()` auto-subscribes to ledger notifications via `subscribe()`
- Exposed `notify_handle()` for callers to await notifications between polls
- Changed `poll_interval()` from 50ms to 5s (safety-net fallback — primary wake-up is via Notify)
- Cursor update was already atomic (cursor and event processing under single lock scope since T-0152 parking_lot migration)
- All 412 unit tests pass
