---
id: integration-test-two-concurrent
level: task
title: "Integration test: two concurrent claim calls return non-overlapping batches"
short_code: "CLOACI-T-0207"
created_at: 2026-03-17T01:38:53.017652+00:00
updated_at: 2026-03-17T01:49:39.760880+00:00
parent: CLOACI-I-0034
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0034
---

# Integration test: two concurrent claim calls return non-overlapping batches

## Parent Initiative

[[CLOACI-I-0034]]

## Objective

Write an integration test that verifies two concurrent `claim_pipeline_batch` calls on the same outbox return non-overlapping results, proving the FOR UPDATE SKIP LOCKED mechanism works correctly for pipelines.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test creates 10 pipeline executions with matching outbox entries
- [ ] Test calls `claim_pipeline_batch(5)` twice (sequentially or concurrently)
- [ ] Assert: the union of both claim results contains all 10 pipelines
- [ ] Assert: the intersection of both claim results is empty (no duplicates)
- [ ] Assert: the `pipeline_outbox` table is empty after both claims complete
- [ ] Test runs against all enabled backends using `get_all_fixtures()`
- [ ] Test file is located in `crates/cloacina/tests/integration/dal/` and registered in the dal mod.rs

## Test Cases

### Test Case 1: Non-overlapping pipeline claims
- **Test ID**: TC-001
- **Preconditions**: Clean database with migrations applied
- **Steps**:
  1. Create 10 pipeline executions with status `Pending`
  2. Insert 10 corresponding `pipeline_outbox` entries
  3. Call `claim_pipeline_batch(5)` -- expect 5 results
  4. Call `claim_pipeline_batch(5)` -- expect 5 different results
  5. Call `claim_pipeline_batch(5)` -- expect 0 results (outbox drained)
- **Expected Results**: No pipeline appears in more than one claim batch; all 10 are claimed exactly once

### Test Case 2: Requeue and reclaim
- **Test ID**: TC-002
- **Preconditions**: Clean database with migrations applied
- **Steps**:
  1. Create 3 pipeline executions with status `Running`
  2. Insert 3 outbox entries
  3. Claim all 3
  4. Requeue 1 pipeline via `requeue_pipeline`
  5. Claim again
- **Expected Results**: Second claim returns exactly the 1 requeued pipeline

## Implementation Notes

### Technical Approach
- Follow the pattern in `task_claiming.rs` integration test
- Use `get_all_fixtures()` to test on all backends
- Use `dal.pipeline_execution().create(...)` to create test pipelines
- Use `dal.pipeline_execution().insert_outbox(...)` to populate the outbox
- Use `dal.pipeline_execution().claim_pipeline_batch(...)` to claim
- Use `HashSet` to verify non-overlap

### Files to Create/Modify
- `crates/cloacina/tests/integration/dal/pipeline_claiming.rs` (new)
- `crates/cloacina/tests/integration/dal/mod.rs` (add module)

### Dependencies
- CLOACI-T-0204 (schema)
- CLOACI-T-0205 (DAL methods)

## Status Updates

*To be added during implementation*
