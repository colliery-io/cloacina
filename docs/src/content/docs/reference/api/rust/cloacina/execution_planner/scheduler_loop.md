---
title: "cloacina::execution_planner::scheduler_loop"
description: "Scheduler loop and workflow execution processing."
sidebar:
  order: 130
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Scheduler loop and workflow execution processing.

This module contains the main scheduling loop that continuously processes
active workflow executions and manages task readiness.

## Structs

### <span id="struct.SchedulerLoop"></span>`struct SchedulerLoop<'a>`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Scheduler loop operations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `& 'a DAL` |  |
| `runtime` | `Arc < Runtime >` |  |
| `instance_id` | `Uuid` |  |
| `poll_interval` | `Duration` |  |
| `dispatcher` | `Option < Arc < dyn Dispatcher > >` | Optional dispatcher for push-based task execution |
| `shutdown_rx` | `Option < tokio :: sync :: watch :: Receiver < bool > >` | Shutdown signal — when the sender drops or sends, the loop exits cleanly. |
| `consecutive_errors` | `u32` | Consecutive error count for circuit breaker / backoff. |



## Functions

### <span id="fn.dispatch_one"></span>`fn dispatch_one`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
async fn dispatch_one (dispatcher : & Arc < dyn Dispatcher > , task : TaskExecution)
```

Dispatch a single Ready task and log the outcome (CLOACI-T-0745). Shared by the postgres (spawned, concurrent) and sqlite (serial) dispatch paths. NoCapacity is expected backpressure (the task stays Ready, retried later); other errors are surfaced as warnings.

<details>
<summary>Source</summary>

```rust
async fn dispatch_one(dispatcher: &Arc<dyn Dispatcher>, task: TaskExecution) {
    let event = TaskReadyEvent::new(
        task.id,
        task.workflow_execution_id,
        task.task_name.clone(),
        task.attempt,
    );
    match dispatcher.dispatch(event).await {
        Ok(()) => {}
        Err(DispatchError::NoCapacity(_)) => {
            debug!(
                task_id = %task.id,
                task_name = %task.task_name,
                "Dispatch deferred: executor at capacity"
            );
        }
        Err(e) => {
            warn!(
                task_id = %task.id,
                task_name = %task.task_name,
                error = %e,
                "Failed to dispatch ready task"
            );
        }
    }
}
```

</details>



