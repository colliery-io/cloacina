---
id: integration-tests-for-execution
level: task
title: "Integration tests for execution events and outbox claiming"
short_code: "CLOACI-T-0085"
created_at: 2026-02-03T20:16:50.109531+00:00
updated_at: 2026-02-06T13:51:29.343926+00:00
parent: CLOACI-I-0022
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0022
---

# Integration tests for execution events and outbox claiming

## Parent Initiative

[[CLOACI-I-0022]] - Execution Events and Outbox-Based Task Distribution

## Objective

Write integration tests verifying execution events are emitted correctly and outbox-based claiming works for both Postgres and SQLite backends.

## Acceptance Criteria

## Acceptance Criteria

- [x] Test: Events emitted for full task lifecycle match actual state (`test_dal_emits_events_on_state_transitions`)
- [x] Test: Events queryable by pipeline_id and task_id (`test_events_queryable_by_pipeline`, `test_events_queryable_by_task`, `test_events_queryable_by_type`)
- [x] Test: Outbox claiming works with multiple concurrent workers (`test_concurrent_claiming_no_duplicates`)
- [x] Test: No duplicate claims (exactly-once claiming) (`test_concurrent_claiming_no_duplicates`)
- [x] Test: Outbox empty after all tasks claimed (`test_outbox_empty_after_claiming`)
- [x] Tests run on both Postgres and SQLite backends (tests run with both backends, postgres_tests module added)

## Test Cases

### Test Case 1: Event Emission Correctness
- Run a pipeline with multiple tasks including retry and failure scenarios
- Query `execution_events` for each task
- Verify event sequence matches actual state transitions
- Verify event_data contains expected context

### Test Case 2: Concurrent Claiming
- Insert N tasks into outbox
- Spawn M workers claiming concurrently
- Verify each task claimed exactly once
- Verify outbox empty after all claimed
- Verify no errors from concurrent access

### Test Case 3: Event Queries
- Execute pipeline with known task structure
- Query events by pipeline_id - verify all events returned
- Query events by task_id - verify only that task's events
- Query by event_type - verify filtering works

### Test Case 4: Multi-tenant Isolation
- Create execution in tenant A schema
- Verify events only visible in tenant A schema
- Verify outbox only visible in tenant A schema

### Dependencies

- Requires all other tasks complete (T-0079 through T-0084)

## Status Updates

### Session 2 (2026-02-06)
- Fixed test assertions - pipeline creation also emits PipelineStarted event
- Test counts now correctly account for: 1 PipelineStarted + N TaskCreated events
- All 22 execution_events tests passing:
  - `test_dal_emits_events_on_state_transitions` - verifies TaskCreated, TaskMarkedReady, TaskClaimed, TaskCompleted
  - `test_events_queryable_by_pipeline` - verifies events can be queried by pipeline_id
  - `test_events_queryable_by_task` - verifies events can be queried by task_id
  - `test_events_queryable_by_type` - verifies events can be queried by event_type
  - `test_outbox_empty_after_claiming` - verifies outbox is empty after all tasks claimed
  - `test_concurrent_claiming_no_duplicates` - verifies no duplicate claims with concurrent workers
  - `test_event_count_and_deletion` - verifies count_by_pipeline, count_older_than, delete_older_than
  - `test_get_recent_events` - verifies get_recent returns events in correct order
  - `test_manual_event_with_data` - verifies manually created events with event_data
  - PostgreSQL-specific tests also passing (with backing services up)
- Started backing services with `angreal services up`
- Running integration tests with `angreal cloacina integration`
