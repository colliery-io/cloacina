---
id: runner-integration-claim-before
level: task
title: "Runner integration — claim before execute, heartbeat during, release on completion"
short_code: "CLOACI-T-0290"
created_at: 2026-03-29T12:33:49.789666+00:00
updated_at: 2026-03-29T13:11:09.153072+00:00
parent: CLOACI-I-0055
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0055
---

# Runner integration — claim before execute, heartbeat during, release on completion

## Parent Initiative

[[CLOACI-I-0055]]

## Objective

Wire task claiming into the task executor so that before a task runs, the runner claims it. While executing, a background heartbeat keeps the claim alive. On completion or failure, the claim is released.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Each runner instance has a stable `runner_id` (UUID, persisted or generated at startup)
- [ ] Before executing a task, the executor calls `claim_task(task_id, runner_id)` — skips if claim fails (another runner got it)
- [ ] During execution, a background tokio task heartbeats every N seconds (configurable, default 10s)
- [ ] If heartbeat returns `ClaimLost`, the executor should log a warning (task may be re-claimed by stale sweep)
- [ ] On task success: release claim, mark task completed
- [ ] On task failure: release claim, mark task failed
- [ ] On task panic/crash: claim is NOT released (stale sweep handles it via expired heartbeat)
- [ ] Existing single-runner behavior unchanged when claiming is disabled (opt-in via config)

## Implementation Notes

### Files to modify
- `crates/cloacina/src/executor/thread_task_executor.rs` — wrap task execution with claim/heartbeat/release
- `crates/cloacina/src/runner/default_runner/mod.rs` — assign `runner_id` at startup
- `crates/cloacina/src/runner/default_runner/config.rs` — add `enable_claiming` and `heartbeat_interval` config

### Key design points
- Heartbeat runs as a separate tokio task, cancelled on task completion
- `runner_id` should be a UUID v4 generated at startup (not persisted — each restart is a new identity)
- Claiming is opt-in: single-runner daemon mode doesn't need the overhead. Multi-instance deployments enable it.

### Depends on
- T-0289 (Claim DAL)

## Status Updates

**2026-03-29**: Complete. Claiming wired into executor, opt-in via config.

### Changes:
- `executor/types.rs` — Added `enable_claiming` and `heartbeat_interval` to `ExecutorConfig`
- `executor/thread_task_executor.rs` — Before execute: `claim_for_runner()`, skips if `AlreadyClaimed`. During: background heartbeat tokio task. After: `release_runner_claim()`. Heartbeat aborted on completion.
- `dispatcher/types.rs` — Added `ExecutionStatus::Skipped` + `ExecutionResult::skipped()`
- `dispatcher/default.rs` — Handle `Skipped` in result match
- `runner/default_runner/config.rs` — Added `enable_claiming` + `heartbeat_interval` fields, accessors, defaults (false, 10s)
- `runner/default_runner/mod.rs` — Pass claiming config to executor

### Design: `instance_id` (already a UUID v4 on `ThreadTaskExecutor`) serves as the runner_id. Claiming is opt-in (disabled by default).
