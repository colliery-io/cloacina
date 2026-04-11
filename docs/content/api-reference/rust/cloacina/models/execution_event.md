# cloacina::models::execution_event <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Execution Event Model

This module defines domain structures and types for tracking execution events.
Execution events provide a complete audit trail of task and pipeline state
transitions for debugging, compliance, and replay capability.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::execution_event::ExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents an execution event record (domain type).

Execution events are append-only records tracking all state transitions
for tasks and pipelines. Each event captures the transition type, associated
context, and ordering information for replay.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` | Unique identifier for this event |
| `pipeline_execution_id` | `UniversalUuid` | The pipeline execution this event belongs to |
| `task_execution_id` | `Option < UniversalUuid >` | The task execution this event relates to (None for pipeline-level events) |
| `event_type` | `String` | The type of event (e.g., "task_created", "task_completed") |
| `event_data` | `Option < String >` | JSON-encoded additional data for the event |
| `worker_id` | `Option < String >` | Worker ID that generated this event (for distributed tracing) |
| `created_at` | `UniversalTimestamp` | When this event was created |
| `sequence_num` | `i64` | Monotonically increasing sequence number for ordering |



### `cloacina::models::execution_event::NewExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new execution event records (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pipeline_execution_id` | `UniversalUuid` | The pipeline execution this event belongs to |
| `task_execution_id` | `Option < UniversalUuid >` | The task execution this event relates to (None for pipeline-level events) |
| `event_type` | `String` | The type of event |
| `event_data` | `Option < String >` | JSON-encoded additional data for the event |
| `worker_id` | `Option < String >` | Worker ID that generated this event |

#### Methods

##### `pipeline_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pipeline_event (pipeline_execution_id : UniversalUuid , event_type : ExecutionEventType , event_data : Option < String > , worker_id : Option < String > ,) -> Self
```

Creates a new execution event for a pipeline-level transition.

<details>
<summary>Source</summary>

```rust
    pub fn pipeline_event(
        pipeline_execution_id: UniversalUuid,
        event_type: ExecutionEventType,
        event_data: Option<String>,
        worker_id: Option<String>,
    ) -> Self {
        Self {
            pipeline_execution_id,
            task_execution_id: None,
            event_type: event_type.as_str().to_string(),
            event_data,
            worker_id,
        }
    }
```

</details>



##### `task_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_event (pipeline_execution_id : UniversalUuid , task_execution_id : UniversalUuid , event_type : ExecutionEventType , event_data : Option < String > , worker_id : Option < String > ,) -> Self
```

Creates a new execution event for a task-level transition.

<details>
<summary>Source</summary>

```rust
    pub fn task_event(
        pipeline_execution_id: UniversalUuid,
        task_execution_id: UniversalUuid,
        event_type: ExecutionEventType,
        event_data: Option<String>,
        worker_id: Option<String>,
    ) -> Self {
        Self {
            pipeline_execution_id,
            task_execution_id: Some(task_execution_id),
            event_type: event_type.as_str().to_string(),
            event_data,
            worker_id,
        }
    }
```

</details>





## Enums

### `cloacina::models::execution_event::ExecutionEventType` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Enumeration of execution event types in the system.

These cover the full lifecycle of tasks and pipelines, providing
complete observability into execution state transitions.

#### Variants

- **`TaskCreated`** - A new task execution record was created
- **`TaskMarkedReady`** - Task transitioned to Ready status (eligible for claiming)
- **`TaskClaimed`** - Task was claimed by a worker
- **`TaskStarted`** - Task execution started
- **`TaskDeferred`** - Task was deferred (waiting for external condition)
- **`TaskResumed`** - Deferred task resumed execution
- **`TaskCompleted`** - Task completed successfully
- **`TaskFailed`** - Task failed with an error
- **`TaskRetryScheduled`** - Task scheduled for retry after failure
- **`TaskSkipped`** - Task was skipped (trigger rules not met)
- **`TaskAbandoned`** - Task was abandoned (exceeded max retries or manually cancelled)
- **`TaskReset`** - Task was reset by recovery process
- **`PipelineStarted`** - Pipeline execution started
- **`PipelineCompleted`** - Pipeline execution completed successfully
- **`PipelineFailed`** - Pipeline execution failed
- **`PipelinePaused`** - Pipeline was paused
- **`PipelineResumed`** - Paused pipeline was resumed
