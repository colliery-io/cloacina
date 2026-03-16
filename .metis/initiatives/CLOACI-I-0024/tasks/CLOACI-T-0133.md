---
id: ledgertrigger-implementing-trigger
level: task
title: "LedgerTrigger implementing Trigger trait with LedgerMatchMode"
short_code: "CLOACI-T-0133"
created_at: 2026-03-15T13:14:13.309658+00:00
updated_at: 2026-03-15T13:36:25.581094+00:00
parent: CLOACI-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0024
---

# LedgerTrigger implementing Trigger trait with LedgerMatchMode

## Parent Initiative

[[CLOACI-I-0024]]

## Objective

Implement `LedgerTrigger` — a specialized `Trigger` implementation that watches the `ExecutionLedger` for task completions and fires detector workflows for derived data sources. This completes the reactive feedback loop: task completes → LedgerTrigger fires → detector runs → downstream boundaries flow.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `LedgerTrigger` struct with `watch_tasks: Vec<String>`, `match_mode: LedgerMatchMode`, `ledger: Arc<RwLock<ExecutionLedger>>`, `cursor: usize`
- [ ] `LedgerMatchMode` enum: `Any` (fire when any watched task completes), `All` (fire when all watched tasks have completed since last fire)
- [ ] Implements the existing `Trigger` trait: `name()`, `poll_interval()`, `allow_concurrent()`, `poll()`
- [ ] `poll()` scans `ledger.events_since(cursor)` for matching `TaskCompleted` events
- [ ] Cursor advances after each poll to avoid re-triggering on old events
- [ ] `All` mode tracks which watched tasks have completed since last fire, resets on trigger
- [ ] Sub-second `poll_interval()` (ledger is in-memory)
- [ ] Registerable with existing `TriggerScheduler`
- [ ] Unit tests: Any mode fires on single match, All mode waits for all, cursor idempotency

## Implementation Notes

### Technical Approach
- In `continuous/ledger_trigger.rs` (new file)
- Implements `cloacina::trigger::Trigger` trait
- For `All` mode: maintain `HashSet<String>` of seen completions since last fire, fire when seen == watch_tasks, then clear
- Returns `TriggerResult::Fire(Some(context))` with the completed task's context, or `TriggerResult::Skip`

### Key Source References
- Existing `Trigger` trait: `crates/cloacina/src/trigger/mod.rs`
- `TriggerResult` enum: same file
- Registration pattern: see `TriggerScheduler` in `trigger_scheduler.rs`

### Dependencies
- T-0122 (ExecutionLedger from I-0023)

## Status Updates

- Created `continuous/ledger_trigger.rs` with `LedgerTrigger` and `LedgerMatchMode`
- Implements existing `Trigger` trait (Send + Sync + Debug + async poll)
- `LedgerMatchMode::Any`: fires when any watched task completes
- `LedgerMatchMode::All`: tracks seen completions in HashSet, fires when all seen, resets on fire
- Cursor-based idempotent scanning: advances past all events each poll
- Sub-second poll interval (50ms) — ledger is in-memory
- Fixed `TriggerError::PollFailed` → `TriggerError::PollError` (actual variant name)
- 7 passing tests: Any fires/skips, All waits/resets, cursor idempotency, empty ledger, metadata
