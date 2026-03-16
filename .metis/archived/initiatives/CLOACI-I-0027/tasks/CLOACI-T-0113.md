---
id: implement-testresult-taskoutcome
level: task
title: "Implement TestResult, TaskOutcome, and assertion helpers"
short_code: "CLOACI-T-0113"
created_at: 2026-03-14T02:59:45.649464+00:00
updated_at: 2026-03-14T03:19:34.328338+00:00
parent: CLOACI-I-0027
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0027
---

# Implement TestResult, TaskOutcome, and assertion helpers

## Parent Initiative

[[CLOACI-I-0027]]

## Objective

Implement `TestResult`, `TaskOutcome` enum, and ergonomic assertion helpers that make test failures readable and actionable. These types are the consumer-facing output of `TestRunner::run()`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TestResult` struct with `context: Context<serde_json::Value>` and `task_outcomes: IndexMap<String, TaskOutcome>`
- [ ] `TaskOutcome` enum: `Completed`, `Failed(TaskError)`, `Skipped`
- [ ] `TaskOutcome::is_completed()`, `is_failed()`, `is_skipped()` convenience methods
- [ ] `TaskOutcome::unwrap_error()` returns `&TaskError` or panics with clear message
- [ ] `TestResult::assert_all_completed()` panics with list of non-completed tasks if any fail
- [ ] `TestResult::assert_task_completed(id)`, `assert_task_failed(id)`, `assert_task_skipped(id)`
- [ ] Assertion panic messages include task ID, actual outcome, and expected outcome
- [ ] `TestResult` indexable by task ID (implement `Index<&str>` for ergonomic `result["task_id"]` access)

## Implementation Notes

### Technical Approach
- `TestResult` and `TaskOutcome` in `result.rs`
- Assertion helpers in `assertions.rs`, implemented as methods on `TestResult` and `TaskOutcome`
- Panic messages should be developer-friendly, e.g.: `assertion failed: expected task 'validate' to be Completed, but was Failed(TaskError: "missing field")`
- `IndexMap` preserves insertion (execution) order, useful for debugging

### Dependencies
- Depends on CLOACI-T-0111 (crate scaffold)
- Can be built in parallel with CLOACI-T-0112 (TestRunner)

## Status Updates

- All types implemented during T-0111 scaffold (needed for runner compilation)
- `TestResult` with `context` + `task_outcomes: IndexMap<String, TaskOutcome>`
- `TaskOutcome` enum with `Completed`, `Failed(TaskError)`, `Skipped`
- All convenience methods: `is_completed()`, `is_failed()`, `is_skipped()`, `unwrap_error()`
- `Display` impl for TaskOutcome (used in assertion messages)
- `Index<&str>` impl for ergonomic `result["task_id"]` access
- All assertion helpers: `assert_all_completed()`, `assert_task_completed()`, `assert_task_failed()`, `assert_task_skipped()`
- Panic messages include task ID, actual outcome, and available tasks
