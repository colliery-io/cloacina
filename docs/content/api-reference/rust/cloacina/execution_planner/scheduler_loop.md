# cloacina::execution_planner::scheduler_loop <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Scheduler loop and workflow execution processing.

This module contains the main scheduling loop that continuously processes
active workflow executions and manages task readiness.

## Structs

### `cloacina::execution_planner::scheduler_loop::SchedulerLoop`<'a>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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

### `cloacina::execution_planner::scheduler_loop::dispatch_one`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


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
