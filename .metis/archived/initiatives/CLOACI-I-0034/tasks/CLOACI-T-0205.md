---
id: implement-claim-pipeline-batch-dal
level: task
title: "Implement claim_pipeline_batch DAL with FOR UPDATE SKIP LOCKED"
short_code: "CLOACI-T-0205"
created_at: 2026-03-17T01:38:50.950406+00:00
updated_at: 2026-03-17T01:49:37.024521+00:00
parent: CLOACI-I-0034
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0034
---

# Implement claim_pipeline_batch DAL with FOR UPDATE SKIP LOCKED

## Parent Initiative

[[CLOACI-I-0034]]

## Objective

Implement the DAL methods for pipeline outbox operations: `insert_outbox`, `claim_pipeline_batch`, and `requeue_pipeline`. These methods enable the scheduler to claim pipelines using FOR UPDATE SKIP LOCKED (Postgres) or IMMEDIATE transactions (SQLite), preventing duplicate processing across multiple scheduler instances.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `insert_outbox(pipeline_execution_id)` inserts a row into `pipeline_outbox`
- [ ] `claim_pipeline_batch(limit)` atomically deletes outbox rows and returns the joined `PipelineExecution` records, using FOR UPDATE SKIP LOCKED on Postgres and IMMEDIATE transactions on SQLite
- [ ] `claim_pipeline_batch` only returns pipelines with status `Pending` or `Running`
- [ ] `requeue_pipeline(pipeline_execution_id)` inserts a new outbox row for pipelines that still need processing
- [ ] The `dispatch_backend!` macro is used for Postgres/SQLite dispatch
- [ ] The Postgres implementation uses `diesel::sql_query()` with raw SQL for the CTE pattern, following `claiming.rs`
- [ ] The SQLite implementation uses simple DELETE + SELECT within an IMMEDIATE transaction
- [ ] Return type is `Vec<PipelineExecution>` (same domain model used by `get_active_executions`)

## Implementation Notes

### Technical Approach
- Add methods directly to `PipelineExecutionDAL` in `pipeline_execution.rs` (or a new `pipeline_claiming.rs` submodule)
- Postgres `claim_pipeline_batch` uses a CTE: DELETE FROM pipeline_outbox WHERE id IN (SELECT ... FOR UPDATE SKIP LOCKED) RETURNING pipeline_execution_id, then JOIN to pipeline_executions
- Use `#[derive(QueryableByName)]` with `#[diesel(check_for_backend(diesel::pg::Pg))]` for the raw SQL result struct, matching the pattern in `claiming.rs`
- SQLite version: SELECT outbox entries, DELETE them, then SELECT from pipeline_executions by collected IDs

### Dependencies
- CLOACI-T-0204 (migration + schema + models must exist first)

## Status Updates

*To be added during implementation*
