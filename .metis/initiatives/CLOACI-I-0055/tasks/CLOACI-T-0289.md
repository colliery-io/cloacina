---
id: claim-dal-claim-heartbeat-release
level: task
title: "Claim DAL — claim, heartbeat, release, find_stale for SQLite + Postgres"
short_code: "CLOACI-T-0289"
created_at: 2026-03-29T12:33:48.983677+00:00
updated_at: 2026-03-29T12:33:48.983677+00:00
parent: CLOACI-I-0055
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0055
---

# Claim DAL — claim, heartbeat, release, find_stale for SQLite + Postgres

## Parent Initiative

[[CLOACI-I-0055]]

## Objective

Implement the DAL operations for task claiming: atomic claim acquisition, heartbeat updates, claim release, and stale claim discovery. Both SQLite and Postgres backends.

## Acceptance Criteria

- [ ] `claim_task(task_id, runner_id)` — atomic compare-and-swap: sets `claimed_by` and `heartbeat_at` only if `claimed_by` is NULL. Returns success/failure (not an error if already claimed).
- [ ] `heartbeat_task(task_id, runner_id)` — updates `heartbeat_at` only if `claimed_by` matches `runner_id`. Returns false if claim was lost.
- [ ] `release_claim(task_id)` — clears `claimed_by` and `heartbeat_at` (on task completion or failure).
- [ ] `find_stale_claims(threshold)` — returns tasks where `claimed_by` is not NULL and `heartbeat_at < now - threshold`.
- [ ] SQLite implementation (UPDATE WHERE for atomicity)
- [ ] Postgres implementation (SELECT FOR UPDATE or UPDATE WHERE RETURNING)
- [ ] Unit tests for each operation on both backends

## Implementation Notes

### Files to modify
- `crates/cloacina/src/dal/unified/task_execution/claiming.rs` — add claim operations (file already exists with retry scheduling)
- `crates/cloacina/src/dal/unified/task_execution/mod.rs` — expose new methods

### Key design points
- `claim_task` must be atomic: two runners calling it concurrently on the same task — exactly one succeeds
- SQLite: single-writer, so UPDATE WHERE is sufficient
- Postgres: UPDATE ... WHERE claimed_by IS NULL RETURNING id (returns 0 rows if already claimed)
- `heartbeat_task` should verify the runner still owns the claim (guard against stale runner updating after re-claim)
- Return a `ClaimResult` enum: `Claimed`, `AlreadyClaimed`, `ClaimLost`

### Depends on
- T-0288 (schema migration)

## Status Updates

*To be added during implementation*
