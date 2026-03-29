---
id: claim-dal-claim-heartbeat-release
level: task
title: "Claim DAL — claim, heartbeat, release, find_stale for SQLite + Postgres"
short_code: "CLOACI-T-0289"
created_at: 2026-03-29T12:33:48.983677+00:00
updated_at: 2026-03-29T12:59:22.946196+00:00
parent: CLOACI-I-0055
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0055
---

# Claim DAL — claim, heartbeat, release, find_stale for SQLite + Postgres

## Parent Initiative

[[CLOACI-I-0055]]

## Objective

Implement the DAL operations for task claiming: atomic claim acquisition, heartbeat updates, claim release, and stale claim discovery. Both SQLite and Postgres backends.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

**2026-03-29**: Complete. All DAL operations implemented for both backends.

### Changes:
- `mod.rs` — Added `RunnerClaimResult` (Claimed/AlreadyClaimed), `HeartbeatResult` (Ok/ClaimLost), `StaleClaim` types
- `claiming.rs` — Added 4 operations with SQLite + Postgres impls each:
  - `claim_for_runner(task_id, runner_id)` — atomic UPDATE WHERE claimed_by IS NULL
  - `heartbeat(task_id, runner_id)` — UPDATE WHERE claimed_by = runner_id
  - `release_runner_claim(task_id)` — clear claimed_by + heartbeat_at
  - `find_stale_claims(threshold)` — SELECT WHERE heartbeat_at < cutoff AND status = Running
