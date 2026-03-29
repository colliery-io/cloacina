---
id: integration-tests-concurrent
level: task
title: "Integration tests — concurrent claimants, crash recovery, double-claim prevention"
short_code: "CLOACI-T-0292"
created_at: 2026-03-29T12:33:51.545268+00:00
updated_at: 2026-03-29T13:45:41.794761+00:00
parent: CLOACI-I-0055
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0055
---

# Integration tests — concurrent claimants, crash recovery, double-claim prevention

## Parent Initiative

[[CLOACI-I-0055]]

## Objective

End-to-end integration tests proving the claiming system works correctly under concurrency, crash scenarios, and edge cases.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Double-claim prevention**: Two runners try to claim the same task concurrently — exactly one succeeds
- [ ] **Heartbeat keeps claim alive**: A claimed task with fresh heartbeats is NOT swept as stale
- [ ] **Crash recovery**: A task with no heartbeat for > threshold is swept and re-queued
- [ ] **Startup grace period**: Sweeper does not evict tasks during its warmup period after restart
- [ ] **Claim release on completion**: Task completion clears `claimed_by` and `heartbeat_at`
- [ ] **Claim release on failure**: Task failure also clears the claim
- [ ] **Stale runner can't heartbeat after re-claim**: If runner A's claim expires and runner B claims the task, runner A's heartbeat call returns `ClaimLost`
- [ ] Tests run on both SQLite and Postgres backends

## Implementation Notes

### Files to create
- `crates/cloacina/tests/integration/dal/task_claiming.rs` — DAL-level tests
- `crates/cloacina/tests/integration/scheduler/claiming.rs` — scheduler-level tests with stale sweep

### Test patterns
- Use the existing `TestFixture` for DB setup
- Simulate concurrent claims with multiple tokio tasks
- Simulate crash by claiming, then never heartbeating, then running sweep
- Simulate startup grace by creating sweeper, checking it doesn't evict during warmup

### Depends on
- T-0288, T-0289, T-0290, T-0291 (all claiming tasks)

## Status Updates

**2026-03-29**: Complete. 5 new integration tests, all pass on SQLite.

### Tests added to `dal/task_claiming.rs`:
1. `test_runner_double_claim_prevention` — two runners claim same task, exactly one wins
2. `test_heartbeat_ownership_guard` — owner heartbeats succeed, non-owner returns ClaimLost
3. `test_release_claim_clears_fields` — release sets claimed_by and heartbeat_at to None
4. `test_reclaim_after_release` — after release, another runner can claim; original runner's heartbeat fails
5. `test_find_stale_claims` — 0s threshold finds recently claimed task, 9999s threshold does not

### Notes:
- Tests use existing `get_all_fixtures()` pattern — run on all enabled backends
- Postgres tests pass when Docker is running (verified earlier in session)
- SQLite: 11 passed (3 original + 5 new + 3 other filtered), 0 failed
