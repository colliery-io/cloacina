---
id: cooperative-task-cancellation-on
level: task
title: "Cooperative task cancellation on claim loss — tokio::select + TaskHandle extension"
short_code: "CLOACI-T-0487"
created_at: 2026-04-13T17:31:23.923615+00:00
updated_at: 2026-04-22T03:08:53.171211+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

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

- 2026-04-21: Implemented both layers.
  - Layer 1 (`tokio::select!`): added `watch::channel(bool)` shared between the heartbeat task and the execute wrapper. Heartbeat fires `true` on `ClaimLost`. New helper `ThreadTaskExecutor::execute_with_cancellation` races `execute_with_timeout` against the watch and returns `ExecutorError::ClaimLost` if the signal wins. Applied to both the handle-holding and handle-less branches of `execute_dispatched`.
  - Layer 2 (`TaskHandle`): added `is_cancelled()` (sync read of the watch value) and `cancelled()` (future that resolves on signal). Wired via new `TaskHandle::with_dal_and_cancel` constructor.
  - Retry path: `should_retry_task` early-returns `false` on `ExecutorError::ClaimLost` — the owning runner drives the outcome, we don't fight for the claim.
  - New error variant `ExecutorError::ClaimLost` with message "Task claim lost to another runner".
  - Tests:
    - `task_handle` unit tests for `is_cancelled` / `cancelled` — default-false, reflects watch, resolves after send, behavior when sender dropped.
    - `tests/integration/executor/claim_loss_cancellation.rs` — forcibly rewrites `claimed_by` in the DB to simulate a competing runner, then asserts (1) a sleeping task that ignores cancellation is dropped mid-sleep (Layer 1, static never flips); (2) a handle-holding task observes cancellation via `handle.cancelled().await` and exits gracefully (Layer 2, static flips).
  - Verified: `angreal cloacina unit --backend sqlite` (700 passed), full `cargo test -p cloacina --test integration --features postgres,sqlite,macros` postgres run (269 passed).
  - Scope note: this is workflow-task-only. Computation graphs / reactors have their own cooperative-shutdown mechanism (`shutdown_signal` + supervisor restart, T-0411/T-0412) and no claim-ownership concept; a unified surface across workflows and CGs would be future work tied to S-0008.
