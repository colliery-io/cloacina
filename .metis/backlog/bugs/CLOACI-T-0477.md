---
id: fix-silent-dependency-context
level: task
title: "Fix silent dependency context loading failures in executor"
short_code: "CLOACI-T-0477"
created_at: 2026-04-11T13:50:36.342354+00:00
updated_at: 2026-04-13T00:10:49.190364+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Fix silent dependency context loading failures in executor

## Objective

Stop silently swallowing dependency context loading failures in `ThreadTaskExecutor`. Non-root tasks should fail explicitly when upstream context can't be loaded, not run with empty/partial data.

## Review Finding References

COR-002 (from architecture review `review/10-recommendations.md` REC-005)

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: Any workflow with task dependencies where a transient DB error occurs during context loading
- **Expected vs Actual**: Task should fail with a clear error when dependency context can't be loaded. Actual: task runs with empty context, produces wrong results, failure only logged at `debug` level.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `build_task_context` returns error for non-root tasks when dependency context loading fails
- [ ] Root tasks (no dependencies) retain current lenient behavior
- [ ] Log level elevated from `debug` to `error` for the failure case
- [ ] Failed task marked as failed with clear error message referencing context load failure
- [ ] No regression in existing tests

## Implementation Notes

### Technical Approach
- Replace `if let Ok(dep_metadata_with_contexts) = ...` with `match` and explicit error propagation
- For non-root tasks: return `Err(ExecutorError::ContextLoadFailed { task_name, cause })`
- For root tasks: keep current behavior (no dependencies to load)
- Elevate `debug!` to `error!` for the failure path

### Key Files
- `crates/cloacina/src/executor/thread_task_executor.rs` — `build_task_context`

### Dependencies
None. Small, focused fix.

## Status Updates

*To be added during implementation*