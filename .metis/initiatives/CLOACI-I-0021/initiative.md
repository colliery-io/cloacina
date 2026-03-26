---
id: taskhandle-execution-control-and
level: initiative
title: "TaskHandle - Execution Control and Deferred Task Support"
short_code: "CLOACI-I-0021"
created_at: 2026-01-29T01:48:51.565981+00:00
updated_at: 2026-01-29T03:21:18.008590+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: taskhandle-execution-control-and
---

# TaskHandle - Execution Control and Deferred Task Support Initiative

## Context

The ThreadTaskExecutor enforces a `max_concurrent_tasks` limit (default: 4) using an AtomicUsize counter. Tasks that poll external conditions (file arrival, API readiness, queue draining) hold a concurrency slot for the entire duration of their polling loop, even though they're doing no real work. With 4 polling tasks, zero slots remain for actual compute.

Promoted from backlog item CLOACI-T-0047.

## Goals & Non-Goals

**Goals:**
- Tasks can release their concurrency slot while waiting on an external condition
- The programming model stays simple — tasks run start-to-finish, no re-entrancy required
- Running state gains sub-states (`Running::Active`, `Running::Deferred`) for observability without changing the top-level state machine
- Existing tasks are unaffected — TaskHandle is opt-in via function signature

**Non-Goals:**
- Persistent deferred state that survives restarts (existing orphan recovery handles this — the task re-enters and hits the defer_until loop again)
- Scheduler-driven resume (no new scheduler logic; the future stays parked in the tokio runtime)
- Broad execution control surface (checkpoint, heartbeat, yield_slot) — can be added later to the same type

## Architecture

### Overview

Inline slot release using a semaphore. The executor's concurrency management switches from AtomicUsize counting to a tokio::sync::Semaphore. TaskHandle holds a reference to the semaphore and the task's execution metadata. When `defer_until` is called, it drops the permit, polls the condition, and reacquires the permit before returning.

### Key Components

1. **TaskHandle** — passed as optional second parameter to task functions. Holds semaphore reference and task execution ID.
2. **Semaphore-based concurrency** — replaces AtomicUsize in ThreadTaskExecutor. Permits are acquired before execution and held for the duration, except during defer.
3. **Running sub-state** — `Running::Active` / `Running::Deferred` expressed as a column or status variant on the task execution record. Set when entering defer_until, cleared on resume. No impact on state machine transitions.
4. **Macro enhancement** — `#[task]` macro detects optional `&TaskHandle` second parameter and generates appropriate wrapper code.

### Sequence: defer_until

```
Task function called (permit acquired)
  → task does initial work
  → calls handle.defer_until(condition, interval)
    → writes Running::Deferred to DB
    → drops semaphore permit (slot freed)
    → loop: sleep(interval), check condition
    → condition true: acquire permit (may wait if at capacity)
    → writes Running::Active to DB
  → task continues with slot re-held
  → task returns, permit dropped naturally
```

### Recovery on restart

No special handling needed. Orphan recovery sees the task as Running (regardless of Active/Deferred sub-state), resets it to Ready, and re-executes. The task re-enters defer_until and resumes polling.

## Detailed Design

### TaskHandle type

```rust
pub struct TaskHandle {
    semaphore: Arc<Semaphore>,
    task_execution_id: UniversalUuid,
    dal: DAL,
}

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
        // Mark as deferred
        self.dal.set_running_sub_state(self.task_execution_id, RunningSubState::Deferred).await?;

        // Release the concurrency slot
        // (permit is held by caller scope — need design for how to transfer ownership)

        loop {
            tokio::time::sleep(poll_interval).await;
            if condition().await {
                break;
            }
        }

        // Re-acquire slot
        let _permit = self.semaphore.acquire().await.map_err(|_| ...)?;

        // Mark as active
        self.dal.set_running_sub_state(self.task_execution_id, RunningSubState::Active).await?;

        Ok(())
    }
}
```

### Macro signature detection

The `#[task]` macro needs to detect whether the function accepts one or two parameters:

```rust
// Single param — current behavior, no TaskHandle
#[task(id = "simple")]
async fn simple(context: &mut Context<Value>) -> Result<(), TaskError> { ... }

// Two params — executor provides TaskHandle
#[task(id = "waiter")]
async fn waiter(context: &mut Context<Value>, handle: &TaskHandle) -> Result<(), TaskError> {
    handle.defer_until(|| file_exists("/data.csv"), Duration::from_secs(5)).await?;
    process_file("/data.csv");
    Ok(())
}
```

### Slot token abstraction

The executor's concurrency permit is wrapped in a `SlotToken` that owns the release/reclaim lifecycle. This decouples TaskHandle from tokio's `OwnedSemaphorePermit` directly, giving a clean extension point for future needs (weighted slots, priorities, cross-executor management).

```rust
pub struct SlotToken {
    permit: Option<OwnedSemaphorePermit>,
    semaphore: Arc<Semaphore>,
}

impl SlotToken {
    /// Release the concurrency slot. Returns immediately.
    pub fn release(&mut self) {
        self.permit.take(); // drop the permit, freeing the slot
    }

    /// Reacquire a concurrency slot. May wait if at capacity.
    pub async fn reclaim(&mut self) -> Result<(), TaskError> {
        let permit = self.semaphore.clone().acquire_owned().await
            .map_err(|_| TaskError::ExecutionFailed { ... })?;
        self.permit = Some(permit);
        Ok(())
    }
}
```

TaskHandle holds a `SlotToken` and `defer_until` calls `release()` / `reclaim()` internally. The token is created by the executor when it acquires the initial permit, and passed into TaskHandle.

## Alternatives Considered

### Option 2: Deferred task state with scheduler resume
The task returns early with a defer result. The scheduler persists a `Deferred` state with a `resume_after` timestamp and re-schedules the task later. Rejected because:
- Forces re-entrant task design — users must structure tasks to handle being called multiple times
- More complex programming model for the common case
- Touches scheduler, database schema, and state machine transitions
- Existing orphan recovery already handles the restart case for Option 1

### Broad TaskHandle scope (checkpoint, heartbeat, yield_slot)
Deferred as future work. The TaskHandle type is extensible — these methods can be added later without breaking existing tasks.

## Implementation Plan

1. Semaphore-based concurrency — replace AtomicUsize with tokio::sync::Semaphore in ThreadTaskExecutor
2. TaskHandle type and defer_until — define the type, implement permit release/reacquire, wire up DB sub-state writes
3. Macro enhancement — detect optional second parameter, generate wrapper that provides TaskHandle
4. Running sub-state — add Active/Deferred distinction to execution records (migration + DAL)
5. Integration tests and example — end-to-end test of defer_until behavior, concurrency slot verification
