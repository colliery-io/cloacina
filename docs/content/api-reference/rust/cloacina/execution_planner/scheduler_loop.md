# cloacina::execution_planner::scheduler_loop <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Scheduler loop and pipeline processing.

This module contains the main scheduling loop that continuously processes
active pipeline executions and manages task readiness.

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
