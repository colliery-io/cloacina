# cloacina::models::execution_event <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Execution Event Model

This module defines domain structures and types for tracking execution events.
Execution events provide a complete audit trail of task and workflow state
transitions for debugging, compliance, and replay capability.
These are API-level types; backend-specific models handle database storage.

## Structs

### `cloacina::models::execution_event::ExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Represents an execution event record (domain type).

Execution events are append-only records tracking all state transitions
for tasks and workflows. Each event captures the transition type, associated
context, and ordering information for replay.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `id` | `UniversalUuid` | Unique identifier for this event |
| `workflow_execution_id` | `UniversalUuid` | The workflow execution this event belongs to |
| `task_execution_id` | `Option < UniversalUuid >` | The task execution this event relates to (None for workflow-level events) |
| `event_type` | `String` | The type of event (e.g., "task_created", "task_completed") |
| `event_data` | `Option < String >` | JSON-encoded additional data for the event |
| `worker_id` | `Option < String >` | Worker ID that generated this event (for distributed tracing) |
| `created_at` | `UniversalTimestamp` | When this event was created |
| `sequence_num` | `i64` | Monotonically increasing sequence number for ordering |
| `request_id` | `Option < UniversalUuid >` | CLOACI-T-0583: id of the originating request. Populated by server
route handlers from the current tracing span; `None` for background
scheduler emissions and the daemon path. |
| `runner_id` | `Option < UniversalUuid >` | CLOACI-T-0583: id of the runner instance that emitted the event.
Populated for per-tenant runner emissions; `None` for the daemon's
single direct runner. |
| `tenant_id` | `Option < String >` | CLOACI-T-0583: tenant scope. `None` on the daemon and on
emissions without a tenant context. |



### `cloacina::models::execution_event::NewExecutionEvent`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`, `Serialize`, `Deserialize`

Structure for creating new execution event records (domain type).

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `workflow_execution_id` | `UniversalUuid` | The workflow execution this event belongs to |
| `task_execution_id` | `Option < UniversalUuid >` | The task execution this event relates to (None for workflow-level events) |
| `event_type` | `String` | The type of event |
| `event_data` | `Option < String >` | JSON-encoded additional data for the event |
| `worker_id` | `Option < String >` | Worker ID that generated this event |
| `request_id` | `Option < UniversalUuid >` | CLOACI-T-0583: originating request id (from the tracing span). |
| `runner_id` | `Option < UniversalUuid >` | CLOACI-T-0583: runner instance id. |
| `tenant_id` | `Option < String >` | CLOACI-T-0583: tenant scope. |

#### Methods

##### `workflow_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn workflow_event (workflow_execution_id : UniversalUuid , event_type : ExecutionEventType , event_data : Option < String > , worker_id : Option < String > ,) -> Self
```

Creates a new execution event for a workflow-level transition.

CLOACI-T-0583 correlation fields default to `None` for backward
compatibility; use [`Self::with_context`] to populate them after
construction.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_event(
        workflow_execution_id: UniversalUuid,
        event_type: ExecutionEventType,
        event_data: Option<String>,
        worker_id: Option<String>,
    ) -> Self {
        Self {
            workflow_execution_id,
            task_execution_id: None,
            event_type: event_type.as_str().to_string(),
            event_data,
            worker_id,
            request_id: None,
            runner_id: None,
            tenant_id: None,
        }
    }
```

</details>



##### `task_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_event (workflow_execution_id : UniversalUuid , task_execution_id : UniversalUuid , event_type : ExecutionEventType , event_data : Option < String > , worker_id : Option < String > ,) -> Self
```

Creates a new execution event for a task-level transition.

CLOACI-T-0583 correlation fields default to `None`; use
[`Self::with_context`] to populate them.

<details>
<summary>Source</summary>

```rust
    pub fn task_event(
        workflow_execution_id: UniversalUuid,
        task_execution_id: UniversalUuid,
        event_type: ExecutionEventType,
        event_data: Option<String>,
        worker_id: Option<String>,
    ) -> Self {
        Self {
            workflow_execution_id,
            task_execution_id: Some(task_execution_id),
            event_type: event_type.as_str().to_string(),
            event_data,
            worker_id,
            request_id: None,
            runner_id: None,
            tenant_id: None,
        }
    }
```

</details>



##### `with_context` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_context (mut self , request_id : Option < UniversalUuid > , runner_id : Option < UniversalUuid > , tenant_id : Option < String > ,) -> Self
```

Builder-style: attach correlation context to an event before insert. Any `Some` value overrides whatever the constructor set; `None`s pass through unchanged. CLOACI-T-0583.

<details>
<summary>Source</summary>

```rust
    pub fn with_context(
        mut self,
        request_id: Option<UniversalUuid>,
        runner_id: Option<UniversalUuid>,
        tenant_id: Option<String>,
    ) -> Self {
        if request_id.is_some() {
            self.request_id = request_id;
        }
        if runner_id.is_some() {
            self.runner_id = runner_id;
        }
        if tenant_id.is_some() {
            self.tenant_id = tenant_id;
        }
        self
    }
```

</details>





## Enums

### `cloacina::models::execution_event::ExecutionEventType` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Enumeration of execution event types in the system.

These cover the full lifecycle of tasks and workflows, providing
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
- **`WorkflowStarted`** - Workflow execution started
- **`WorkflowCompleted`** - Workflow execution completed successfully
- **`WorkflowFailed`** - Workflow execution failed
- **`WorkflowPaused`** - Workflow was paused
- **`WorkflowResumed`** - Paused workflow was resumed
