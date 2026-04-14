---
id: add-heartbeat-driven-cancellation
level: task
title: "Add heartbeat-driven cancellation for claim-lost tasks"
short_code: "CLOACI-T-0481"
created_at: 2026-04-11T14:49:50.083060+00:00
updated_at: 2026-04-13T18:32:48.881531+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Guard mark_completed/mark_failed with claim ownership

## Objective

Add `WHERE claimed_by = runner_id` to `mark_completed` and `mark_failed` DAL operations so that a runner that lost its claim cannot overwrite results from the runner that now owns the task. This is a small, defensive fix — the last-writer-wins race condition is currently unguarded.

## Review Finding References

COR-005 (REC-010) — defensive subset. Full cancellation in CLOACI-T-0487.

## Background (from investigation)

Today when heartbeat detects `ClaimLost`, the heartbeat loop breaks but the task keeps running. When it finishes, `complete_task_transaction` saves context (upsert overwrites) and `mark_completed` succeeds (no ownership check). Two runners can execute the same task concurrently and last writer wins.

The heartbeat detection is correct. The gap is that `mark_completed`/`mark_failed` trust the caller unconditionally.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `mark_completed` adds `WHERE claimed_by = runner_id` (Postgres + SQLite)
- [ ] `mark_failed` adds `WHERE claimed_by = runner_id` (Postgres + SQLite)
- [ ] Both return a flag indicating whether the write was applied (rows_updated > 0)
- [ ] `complete_task_transaction` checks the flag and logs WARN + skips context save if claim was lost
- [ ] Context upsert is gated on claim ownership (save context AFTER confirming claim, or check before)
- [ ] No regression in existing tests

## Implementation Notes

### Approach
- `mark_completed` and `mark_failed` already take `task_execution_id`. Add `runner_id` parameter.
- Add `.filter(task_executions::claimed_by.eq(Some(runner_id)))` — same pattern as heartbeat.
- Return `bool` or enum indicating whether the transition happened.
- In `complete_task_transaction`, check ownership before saving context to avoid overwriting.

### Key Files
- `crates/cloacina/src/dal/unified/task_execution/state.rs` — mark_completed, mark_failed
- `crates/cloacina/src/executor/thread_task_executor.rs` — complete_task_transaction, mark_task_failed

### Edge Cases
- Tasks running without claiming enabled (`enable_claiming = false`): `claimed_by` is NULL. The filter needs to handle this — either skip the check or use `claimed_by IS NULL OR claimed_by = runner_id`.
- Scheduler-driven completions (not executor): scheduler calls mark_completed for pipeline-level completion — those don't have a runner_id. May need a separate code path or pass `None`.

## Status Updates

*To be added during implementation*
