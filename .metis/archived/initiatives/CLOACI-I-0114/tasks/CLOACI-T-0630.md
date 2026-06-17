---
id: result-handling-extraction-share
level: task
title: "Result-handling extraction — share post-execution status/retry/context-write logic out of ThreadTaskExecutor"
short_code: "CLOACI-T-0630"
created_at: 2026-05-27T17:36:28.055785+00:00
updated_at: 2026-05-28T18:26:42.128518+00:00
parent: CLOACI-I-0114
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0114
---

# Result-handling extraction — share post-execution status/retry/context-write logic out of ThreadTaskExecutor

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0114]].

## Objective **[REQUIRED]**

Eliminate the single biggest correctness risk of the fleet *before* building it: the agent path drifting from the thread path on status transitions, retry decisions, and context persistence. Extract everything `ThreadTaskExecutor` does *after* the task closure returns — status write, context persist, retry/timeout classification, metrics — into a shared component that both the thread executor and (later) the fleet reconciliation call. After this, the fleet differs from threads only in *where the closure ran*, not in *how the outcome is recorded*.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] Shared result-handling component extracted — `TaskResultHandler` in `crates/cloacina/src/executor/result_handler.rs`. Takes outcome + identity + retry policy + duration; performs all DAL state writes, counter bumps, and logging; returns the dispatcher's `ExecutionResult`.
- [x] `ThreadTaskExecutor` refactored to call it — post-closure branch tree collapsed to one `self.result_handler.handle_outcome(...)` call. Counters widened to `Arc<AtomicU64>` so the handler and the executor's `metrics()` share the live values; `runner_id` computed once and threaded through. No observable behavior change (same DAL calls, same save-then-mark order, same retry semantics).
- [x] Existing tests pass unchanged — `angreal test unit` green (698 tests, 0 failures, 0 warnings). The 7 `is_transient_*` tests moved verbatim to `result_handler.rs` and pass under their new module path.
- [x] Callable from a non-thread context — `TaskResultHandler::new(dal, total_executed, total_failed, runner_id)` takes nothing thread-specific; the fleet's `FleetExecutor` (T-0633) will construct its own with the agent's owning runner id.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Pure refactor in the `cloacina` crate. Carve the post-closure block into a function/struct taking the result + claim/identity and the DAL. Keep retry classification (`RetryCondition`/`RetryPolicy`) inside it. This is the one fleet task with **no dependency on the substrate** — it can be pulled forward and landed while [[CLOACI-I-0115]] is in flight.

### Dependencies
None (independent of the substrate). Feeds [[CLOACI-T-0633]] (fleet reconciliation reuses this component).

### Risk Considerations
- Silent behavior drift during extraction — lean on the existing test suite; consider a characterization test pinning status/retry outcomes before refactoring.
- `with_task_handle`/slot-token and metrics side effects must stay correctly scoped after the move.

## Status Updates **[REQUIRED]**

### 2026-05-28 — Extracted, no behavior change, `angreal test unit` ✅

- **`crates/cloacina/src/executor/result_handler.rs`** (new) — `TaskResultHandler` struct: `dal: DAL`, `total_executed/total_failed: Arc<AtomicU64>`, `runner_id: Option<UniversalUuid>`. Single public entry `handle_outcome(event, claimed_task, outcome, retry_policy, duration) -> ExecutionResult` that reproduces the thread executor's post-closure branch tree (success → `complete_task_transaction` → counters/log; error → `should_retry_task` → `schedule_task_retry` or `mark_failed`). Private helpers — `complete_task_transaction`, `save_task_context`, `should_retry_task`, `is_transient_error` (pub for testing), `schedule_task_retry` — moved verbatim from `ThreadTaskExecutor` (same DAL calls, same COR-10 save-then-mark order, same retry-policy semantics).
- **`ThreadTaskExecutor`** refactored:
  - Struct gains `result_handler: TaskResultHandler` field; `total_executed`/`total_failed` widened to `Arc<AtomicU64>` so the handler shares the live counters with the executor's `metrics()` view.
  - Constructor computes `runner_id` once (`if config.enable_claiming { Some(instance_id) } else { None }`) and hands it to the handler — same logic the inline `claim_runner_id` had at every call site.
  - Post-closure block (the entire `let result = match execution_result { ... }` tree) collapsed to a single `self.result_handler.handle_outcome(...)` call.
  - `Clone` impl threads the handler clone and `Arc::clone`s the counters (clones now share counters and the handler).
  - Deleted moved methods + their orphaned doc-comments; trimmed `use chrono::Utc;`, `info`, `warn`, `RetryCondition`, `RetryPolicy` imports that became unused.
- **Tests:** the 7 `is_transient_*` tests moved verbatim to `result_handler.rs` under `is_transient_tests` (they now construct a `TaskResultHandler` instead of a `ThreadTaskExecutor`). 698 tests pass total, 0 failures, 0 warnings.
- **Postgres path** compiled clean via `angreal check crate crates/cloacina` ✅.

**T-0630 complete.** Both the thread executor's existing post-execution path AND the upcoming fleet reconciliation (T-0633) now route through one shared handler, eliminating the single largest correctness risk of the fleet (thread/fleet behavioral drift) before any fleet code is written.