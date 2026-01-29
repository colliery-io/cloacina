---
id: enhance-task-macro-to-detect
level: task
title: "Enhance #[task] macro to detect optional TaskHandle parameter"
short_code: "CLOACI-T-0074"
created_at: 2026-01-29T02:02:25.580831+00:00
updated_at: 2026-01-29T02:35:46.162410+00:00
parent: CLOACI-I-0021
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0021
---

# Enhance #[task] macro to detect optional TaskHandle parameter

## Parent Initiative

[[CLOACI-I-0021]]

## Objective

Enhance the `#[task]` proc macro to detect when a task function accepts a second `&TaskHandle` parameter and generate the appropriate wrapper code. Existing single-parameter tasks must continue to work unchanged.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Macro detects single-param `fn(context)` and two-param `fn(context, handle)` signatures
- [ ] Generated Task trait impl calls the user function with TaskHandle when requested
- [ ] The Task trait's `execute` method signature supports passing a TaskHandle
- [ ] Existing tasks compile and work without modification
- [ ] A `requires_handle()` method on Task trait indicates whether the task needs a TaskHandle
- [ ] Compile-time error if the second parameter type is not TaskHandle

## Implementation Notes

### Key files
- `crates/cloacina-macros/src/tasks.rs` — proc macro code generation
- `crates/cloacina-workflow/src/task.rs` — Task trait definition
- `crates/cloacina/src/executor/thread_task_executor.rs` — executor passes TaskHandle when needed

### Dependencies
- T-0073 (TaskHandle type) — completed

## Progress

### Implementation Complete
- Added `requires_handle() -> bool` default method to Task trait in `cloacina-workflow/src/task.rs`
- Added task-local storage (`TASK_HANDLE_SLOT`) and accessor functions (`take_task_handle`, `return_task_handle`, `with_task_handle`) to `cloacina/src/executor/task_handle.rs`
- Modified macro in `cloacina-macros/src/tasks.rs` to detect optional second parameter named `handle` or `task_handle`
- Macro generates `requires_handle() -> true` in trait impl when handle param detected
- Macro-generated execute body takes handle from task-local, passes to user fn, returns it after
- Updated `ThreadTaskExecutor::execute()` to wrap task execution with `with_task_handle` scope when `task.requires_handle()` is true
- Exported new functions from `executor/mod.rs` and `lib.rs`
- All 284 unit tests pass, all macro validation tests pass

## Status Updates
