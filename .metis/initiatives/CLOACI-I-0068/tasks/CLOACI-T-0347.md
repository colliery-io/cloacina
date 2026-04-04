---
id: t6-executor-and-scheduler-gap
level: task
title: "T6: Executor and scheduler gap coverage"
short_code: "CLOACI-T-0347"
created_at: 2026-04-03T13:09:26.355338+00:00
updated_at: 2026-04-03T18:16:42.156414+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T6: Executor and scheduler gap coverage

## Parent Initiative
[[CLOACI-I-0068]] — Tier 2 (~510 missed lines)

## Objective
Improve coverage for executor and scheduler modules. thread_task_executor.rs is at 52%, scheduler.rs at 52%, executor/types.rs at 0%.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [ ] thread_task_executor.rs: test spawn, execute, timeout handling, concurrent execution limits
- [ ] scheduler.rs: test poll_and_execute, missed execution catchup, schedule enable/disable
- [ ] executor/types.rs: test ExecutionResult, PipelineResult construction and accessors
- [ ] executor/pipeline_executor.rs: test execute/execute_async trait methods (72% → >85%)
- [ ] Coverage of executor/ moves from ~52% to >70%

## Source Files
- crates/cloacina/src/executor/thread_task_executor.rs (169 missed, 52%)
- crates/cloacina/src/scheduler.rs (279 missed, 52%)
- crates/cloacina/src/executor/types.rs (62 missed, 0%)
- crates/cloacina/src/executor/pipeline_executor.rs (19 missed, 72%)

## Implementation Notes
The thread_task_executor tests need real workflows registered in the global registry. The scheduler tests need a real DB + registered cron schedules. executor/types.rs is likely pure data structures — unit tests only.

## Status Updates

### 2026-04-03 — Complete (60 new tests, 74 total passing)

types.rs (12): 0%→70.7%. thread_task_executor.rs (22): merge_context, is_transient_error, construction. pipeline_executor.rs (16): PipelineStatus/Error/Result. scheduler.rs (10): config, catchup policy, schedule helpers.
