---
id: cooperative-task-cancellation-on
level: task
title: "Cooperative task cancellation on claim loss — tokio::select + TaskHandle extension"
short_code: "CLOACI-T-0487"
created_at: 2026-04-13T17:31:23.923615+00:00
updated_at: 2026-04-13T17:31:23.923615+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Cooperative task cancellation on claim loss — tokio::select + TaskHandle extension

*Prerequisite: T-0481 (claim ownership guard) provides the defensive DAL layer. This task adds active cancellation on top.*

## Objective

When heartbeat detects `ClaimLost`, actively cancel the running task so it stops consuming resources and doesn't attempt to write results. Today the heartbeat loop breaks but the task keeps running to completion.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

### Technical Debt Impact
- **Current Problems**: A task whose claim was lost continues executing to completion, wasting CPU/IO. With T-0481's guard it can't overwrite results, but it still runs unnecessarily.
- **Benefits of Fixing**: Claim-lost tasks are cancelled promptly. Resources freed. Tasks that opt in via TaskHandle can do graceful cleanup.
- **Risk Assessment**: `tokio::select!` cancellation drops the future mid-execution — safe for pure-compute tasks, but tasks with external side effects (HTTP calls, file writes) may leave partial work. TaskHandle opt-in mitigates this for tasks that need graceful shutdown.

## Design (two layers)

### Layer 1: tokio::select cancellation (executor-level, no trait changes)

- Heartbeat sends on a `oneshot::Sender<()>` when `ClaimLost` detected
- Executor wraps `task.execute()` in `tokio::select!` racing against the cancellation receiver
- If cancelled, the task future is dropped and executor returns `ExecutionResult::failure("claim lost")`
- Works for all tasks without any code changes on the task side

### Layer 2: TaskHandle.is_cancelled() (opt-in graceful shutdown)

- Extend `TaskHandle` with `is_cancelled() -> bool` and `cancelled() -> impl Future` (async wait)
- Tasks that call `requires_handle() -> true` get a handle wired to the cancellation signal
- Long-running tasks can check `handle.is_cancelled()` at safe points and clean up before returning
- Short tasks that don't check it still get Layer 1 cancellation

## Acceptance Criteria

- [ ] Heartbeat sends cancellation signal on `ClaimLost` (oneshot channel)
- [ ] Executor races `task.execute()` against cancellation via `tokio::select!`
- [ ] Cancelled tasks return failure result with "claim lost" reason
- [ ] `TaskHandle` extended with `is_cancelled()` and `cancelled()` methods
- [ ] Tasks using TaskHandle can observe cancellation cooperatively
- [ ] Integration test: simulate claim loss mid-execution, verify task is cancelled

## Key Files

- `crates/cloacina/src/executor/thread_task_executor.rs` — select cancellation wiring
- `crates/cloacina/src/executor/task_handle.rs` — TaskHandle extension
- `crates/cloacina-workflow/src/task.rs` — TaskHandle trait (if needed)

## Dependencies

- T-0481 (claim ownership guard) should land first — provides the safety net if cancellation doesn't fire

## Status Updates

*To be added during implementation*
