---
id: taskhandle-execution-control
level: task
title: "TaskHandle - Execution control handle with defer_until support"
short_code: "CLOACI-T-0047"
created_at: 2025-12-13T20:18:24.754380+00:00
updated_at: 2025-12-13T20:18:24.754380+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#feature"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# TaskHandle - Execution control handle with defer_until support

## Objective

Provide an optional `TaskHandle` parameter for tasks that need execution control capabilities, starting with `defer_until` for releasing concurrency slots while polling for external conditions.

## Backlog Item Details

### Type
- [x] Feature - New functionality or enhancement

### Priority
- [x] P2 - Medium (nice to have)

### Business Justification
- **User Value**: Tasks waiting on external conditions (file arrival, API state, etc.) don't consume concurrency slots, allowing more actual work to proceed
- **Business Value**: Better resource utilization, higher throughput for workflows with polling tasks
- **Effort Estimate**: M

## Problem Statement

Currently, the executor has a `max_concurrent_tasks` limit (default: 4). A task doing:

```rust
loop {
    if file_exists(path).await { break; }
    tokio::time::sleep(Duration::from_secs(5)).await;
}
```

Consumes a slot the entire time, even though it's just waiting. With 4 polling tasks, no slots remain for actual work.

## Proposed Solution

### Optional TaskHandle Parameter

The `#[task]` macro detects function signature and provides `TaskHandle` when requested:

```rust
// Existing tasks unchanged - no handle needed
#[task(id = "simple_task")]
async fn simple_task(context: &mut Context<Value>) -> Result<(), TaskError> {
    // ...
}

// Tasks needing execution control opt-in to second parameter
#[task(id = "waiting_task")]
async fn waiting_task(
    context: &mut Context<Value>,
    handle: &TaskHandle,
) -> Result<(), TaskError> {
    // Releases slot while polling, re-acquires when condition is true
    handle.defer_until(
        || async { file_exists("/inbox/data.csv").await },
        Duration::from_secs(5)
    ).await?;

    // Continues with slot re-acquired
    process_file().await
}
```

### TaskHandle::defer_until Implementation

```rust
impl TaskHandle {
    pub async fn defer_until<F, Fut>(
        &self,
        condition: F,
        poll_interval: Duration,
    ) -> Result<(), TaskError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = bool>,
    {
        // Release the concurrency slot
        self.executor.release_slot();

        // Poll until condition is true
        loop {
            if condition().await {
                break;
            }
            tokio::time::sleep(poll_interval).await;
        }

        // Re-acquire slot (may wait if at capacity)
        self.executor.acquire_slot().await;

        Ok(())
    }
}
```

### Key Design Decisions

1. **No separate service** - polling happens inline, just with slot released
2. **Opt-in via signature** - backwards compatible, only tasks that need it add the parameter
3. **Clean separation** - Context is for data, TaskHandle is for execution control
4. **Re-acquire may wait** - if slots are full when condition fires, task waits for availability

## Acceptance Criteria

- [ ] `TaskHandle` struct with `defer_until` method
- [ ] `#[task]` macro detects optional second `&TaskHandle` parameter
- [ ] `defer_until` releases slot during polling
- [ ] `defer_until` re-acquires slot when condition is true
- [ ] Existing single-parameter tasks unchanged (backwards compatible)
- [ ] Integration tests for defer_until behavior
- [ ] Documentation with examples

## Implementation Notes

### Technical Approach

1. Create `TaskHandle` struct with executor reference
2. Update `#[task]` macro to parse function signature for optional second param
3. Update task executor to create and pass `TaskHandle` when needed
4. Implement slot release/acquire in executor (likely via semaphore or atomic counter)

### Future TaskHandle Methods

- `checkpoint()` - save intermediate state
- `heartbeat()` - keep task alive during long operations
- `yield_slot()` - temporarily release slot without condition

### Dependencies
- Task macro (cloacina-macros)
- Thread task executor (slot management)

### Risk Considerations
- Re-acquire contention if many tasks defer simultaneously
- Need to handle task cancellation during defer (cleanup slot state)

## Status Updates

- **2025-12-13**: Created from design discussion
