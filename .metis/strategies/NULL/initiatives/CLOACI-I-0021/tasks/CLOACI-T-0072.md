---
id: replace-atomicusize-concurrency
level: task
title: "Replace AtomicUsize concurrency tracking with Semaphore"
short_code: "CLOACI-T-0072"
created_at: 2026-01-29T02:02:25.212160+00:00
updated_at: 2026-01-29T02:08:31.098332+00:00
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

# Replace AtomicUsize concurrency tracking with Semaphore

## Parent Initiative

[[CLOACI-I-0021]]

## Objective

Replace the AtomicUsize-based concurrency counter in ThreadTaskExecutor with a tokio::sync::Semaphore. This is the foundation for TaskHandle/defer_until — semaphore permits can be released and reacquired, unlike a simple counter.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] ThreadTaskExecutor uses Arc<Semaphore> instead of AtomicUsize for active_tasks
- [ ] Permit is acquired before task execution and held for the duration
- [ ] has_capacity() uses semaphore available_permits()
- [ ] ExecutorMetrics still reports active task count correctly
- [ ] All existing tests pass with no behavior change

## Implementation Notes

### Key files
- `crates/cloacina/src/executor/thread_task_executor.rs` — main change
- `crates/cloacina/src/executor/types.rs` — ExecutorConfig, ExecutorMetrics
- `crates/cloacina/src/dispatcher/traits.rs` — TaskExecutor trait (has_capacity)

### Approach
- Replace `active_tasks: AtomicUsize` with `semaphore: Arc<Semaphore>` initialized to max_concurrent_tasks
- Acquire OwnedSemaphorePermit before executing a task, drop on completion
- Derive active count from `max - available_permits()` for metrics
- Keep total_executed and total_failed as AtomicU64

## Progress

- Replaced `active_tasks: AtomicUsize` with `semaphore: Arc<Semaphore>` in ThreadTaskExecutor
- Constructor captures `max_concurrent_tasks` before moving config, initializes semaphore
- `execute()` acquires `OwnedSemaphorePermit` at entry; permit drops naturally at scope exit
- Removed all manual `fetch_add`/`fetch_sub` calls on the old counter
- `has_capacity()` now checks `available_permits() > 0`
- `metrics()` derives active count from `max - available_permits()`
- Clone impl shares the semaphore via `Arc::clone` (clones coordinate on same limit)
- Added `pub fn semaphore()` accessor for future TaskHandle use
- 276 unit tests + 11 integration tests passing

## Status Updates

- **2026-01-29**: Completed. Semaphore swap is in place, all tests green.
