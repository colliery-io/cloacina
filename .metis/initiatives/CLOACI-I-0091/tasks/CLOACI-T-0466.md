---
id: wire-runtime-into-defaultrunner
level: task
title: "Wire Runtime into DefaultRunner and execution engine"
short_code: "CLOACI-T-0466"
created_at: 2026-04-09T16:59:30.906683+00:00
updated_at: 2026-04-09T16:59:30.906683+00:00
parent: CLOACI-I-0091
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0091
---

# Wire Runtime into DefaultRunner and execution engine

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0091]]

## Objective

Thread `Runtime` through `DefaultRunner`, `TaskScheduler`, executor, and all internal call sites that currently read from globals via `get_task()`, `global_task_registry()`, etc. The runner uses `Runtime::from_global()` by default (backward compat) but accepts an injected `Runtime` for isolation.

**Effort**: 2-3 days

## Acceptance Criteria

- [ ] `DefaultRunner` holds an `Arc<Runtime>` field
- [ ] `DefaultRunner::new()` and `DefaultRunner::with_config()` default to `Runtime::from_global()`
- [ ] `DefaultRunnerBuilder` has a `.runtime(runtime)` method to inject a custom Runtime
- [ ] `TaskScheduler` receives `Runtime` from the runner (not reading globals directly)
- [ ] `ThreadTaskExecutor` uses `runtime.get_task()` instead of the global `get_task()`
- [ ] `execution_planner` module uses `runtime.get_workflow()` instead of `global_workflow_registry()`
- [ ] Scheduler/executor don't call `global_task_registry()`, `global_workflow_registry()`, or `global_trigger_registry()` directly — all go through `Runtime`
- [ ] Existing code continues to work via `from_global()` default
- [ ] All tests pass (behavior unchanged, just routed through Runtime)

## Implementation Notes

### Technical Approach

1. Add `runtime: Arc<Runtime>` to `DefaultRunner` struct
2. In `DefaultRunner::new()` / `with_config()`, set `runtime = Arc::new(Runtime::from_global())`
3. Add `runtime(runtime: Runtime)` to `DefaultRunnerBuilder`
4. Pass `runtime` to `TaskScheduler::new()` (add field)
5. Pass `runtime` to `ThreadTaskExecutor` (add field)
6. In executor, replace `get_task(&namespace)` with `self.runtime.get_task(&namespace)`
7. In scheduler, replace `global_workflow_registry()` reads with `self.runtime.get_workflow()`
8. In trigger evaluation, replace global reads with runtime reads

The key call sites to update:
- `executor/thread_task_executor.rs` — `get_task()` calls (~3 sites)
- `execution_planner/mod.rs` — `global_workflow_registry()` reads (~5 sites)
- `execution_planner/scheduler_loop.rs` — workflow lookup for dependency checking
- `runner/default_runner/services.rs` — passes scheduler/executor to spawned tasks

### Dependencies
After T-0465 (Runtime struct must exist first).

## Status Updates

*To be added during implementation*
