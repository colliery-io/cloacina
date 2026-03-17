---
id: modify-scheduler-loop-to-use
level: task
title: "Modify scheduler loop to use pipeline claiming instead of all-scan"
short_code: "CLOACI-T-0206"
created_at: 2026-03-17T01:38:52.288784+00:00
updated_at: 2026-03-17T01:49:38.762508+00:00
parent: CLOACI-I-0034
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0034
---

# Modify scheduler loop to use pipeline claiming instead of all-scan

## Parent Initiative

[[CLOACI-I-0034]]

## Objective

Modify the scheduler loop (`process_active_pipelines`) to use outbox-based pipeline claiming instead of scanning all active pipelines, and modify `schedule_workflow_execution` to insert into `pipeline_outbox` when creating new pipelines. This enables multiple scheduler instances to process pipelines without duplicating work.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `process_active_pipelines()` calls `claim_pipeline_batch(100)` instead of `get_active_executions()`
- [ ] After processing each pipeline, if its status is still `Pending` or `Running` (not Completed/Failed/Cancelled), `requeue_pipeline` is called to re-insert it into the outbox
- [ ] Backward compatibility: if `claim_pipeline_batch` returns empty but `get_active_executions()` returns results (migration transition period), fall back to the old scan behavior
- [ ] `schedule_workflow_execution` in `task_scheduler/mod.rs` inserts a `pipeline_outbox` row inside the same transaction that creates the pipeline execution
- [ ] Both Postgres and SQLite code paths in `create_pipeline_postgres` and `create_pipeline_sqlite` include the outbox insert
- [ ] Existing tests continue to pass

## Implementation Notes

### Technical Approach
1. In `scheduler_loop.rs::process_active_pipelines()`:
   - Replace `get_active_executions()` with `claim_pipeline_batch(100)`
   - After `process_pipelines_batch`, iterate the processed pipelines and call `requeue_pipeline` for any that are still active
   - Add fallback: if claimed list is empty, call `get_active_executions()` and process those (for backward compat during rolling upgrades)

2. In `task_scheduler/mod.rs`:
   - In `create_pipeline_postgres` and `create_pipeline_sqlite`, after the pipeline INSERT, add `diesel::insert_into(pipeline_outbox::table).values(&NewUnifiedPipelineOutbox { pipeline_execution_id: pipeline_id }).execute(conn)?;`
   - Import `pipeline_outbox` from schema and `NewUnifiedPipelineOutbox` from models

### Files to Modify
- `crates/cloacina/src/task_scheduler/scheduler_loop.rs`
- `crates/cloacina/src/task_scheduler/mod.rs`

### Dependencies
- CLOACI-T-0204 (schema + models)
- CLOACI-T-0205 (DAL methods)

### Risk Considerations
- The fallback to `get_active_executions()` ensures zero downtime during migration -- pipelines created before the outbox migration will still be processed via the old scan path

## Status Updates

*To be added during implementation*
