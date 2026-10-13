---
title: "cloacina::execution_planner"
description: "API reference of the Rust module cloacina::execution_planner."
editUrl: false
sidebar:
  order: 111
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

**Task Scheduler**

The Task Scheduler converts Workflow definitions into persistent database execution plans
and manages task readiness based on dependencies and trigger rules.

**Overview**

The scheduler builds on existing Cloacina components:
- **Workflow**: Task definitions and dependency graphs
- **Context**: Type-safe serializable execution context
- **Database**: Persistent execution state tracking
- **DAL**: Data access layer for database operations

**Key Features**

- Convert Workflow instances into database execution plans
- Manage task state transitions based on dependencies
- Support advanced trigger rules for conditional execution
- Coordinate with executor through database state
- Context management and merging for task dependencies

Recovery of orphaned tasks is handled out-of-band by
[`stale_claim_sweeper::StaleClaimSweeper`](../../../rust/cloacina/execution_planner/stale_claim_sweeper/#struct.StaleClaimSweeper), which releases stale claims
based on heartbeat expiry rather than wall-clock task status.

**Task State Management**

Tasks transition through the following states:
- **NotStarted**: Initial state when task is created
- **Pending**: Waiting for dependencies to complete
- **Ready**: Dependencies satisfied, ready for execution
- **Running**: Currently being executed
- **Completed**: Successfully finished
- **Failed**: Execution failed
- **Skipped**: Skipped due to trigger rules
- **Abandoned**: Permanently failed after recovery attempts

**Error Handling & Recovery**

The scheduler relies on per-task retry policies for failure handling.
Orphaned tasks (claimed by a runner whose heartbeat has expired) are
reclaimed by the stale claim sweeper rather than by scheduler startup.

**Context Management**

Context handling follows these rules:
- Initial context provided at workflow execution
- Single dependency: inherits context directly
- Multiple dependencies: merges contexts with later overrides
- Type-safe serialization/deserialization
- Validation of context values in trigger rules

**Performance Considerations**

- Scheduling loop runs every second by default
- Efficient database queries for task state updates
- Batch processing of task readiness checks
- Optimized context merging for multiple dependencies
- Minimal database locking for concurrent operations

**Thread Safety**

The scheduler is designed for concurrent operation:
- Thread-safe database operations
- Atomic task state transitions
- Safe context merging for parallel tasks
- Lock-free trigger rule evaluation

**Usage**

```rust
use cloacina::{workflow, task, Context, Database, TaskError};
use cloacina::execution_planner::TaskScheduler;

// Define tasks
#[task(id = "fetch-data", dependencies = [])]
async fn fetch_data(context: &mut Context<serde_json::Value>) -> Result<(), TaskError> {
    context.insert("data", serde_json::json!({"status": "fetched"}))?;
    Ok(())
}

// Create workflow
let workflow = workflow! {
    name: "data-pipeline",
    description: "Simple data processing workflow",
    tasks: [fetch_data]
};

// Schedule execution
let database = Database::new("postgresql://localhost/cloacina")?;
let scheduler = TaskScheduler::new(database, vec![workflow]);
let input_context = Context::new();
let execution_id = scheduler.schedule_workflow_execution("data-pipeline", input_context).await?;

// Run scheduling loop
scheduler.run_scheduling_loop().await?;
```

## Re-exports

- [`TriggerCondition`](../../../rust/cloacina/execution_planner/trigger_rules/#enum.TriggerCondition)
- [`TriggerRule`](../../../rust/cloacina/execution_planner/trigger_rules/#enum.TriggerRule)
- [`ValueOperator`](../../../rust/cloacina/execution_planner/trigger_rules/#enum.ValueOperator)

## Structs

### <span id="struct.TaskScheduler"></span><code style="font-size: 0.85em">struct TaskScheduler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

The main Task Scheduler that manages workflow execution and task readiness.

The TaskScheduler converts Workflow definitions into persistent database execution plans,
tracks task state transitions, and manages dependencies through trigger rules.

**Thread Safety**

The TaskScheduler is designed to be thread-safe and can be shared across multiple threads.
All database operations are performed through a connection pool, and state transitions
are handled atomically.

**Error Handling**

The scheduler implements comprehensive error handling:
- Database errors are wrapped in ValidationError
- Workflow validation errors are caught early
- Context evaluation errors are handled gracefully

**Performance**

The scheduler is optimized for:
- Efficient database operations
- Minimal locking
- Batch processing where possible
- Memory-efficient context management

**Examples:**

```rust
use cloacina::{Database, TaskScheduler};
use cloacina::workflow::Workflow;

// Create a new scheduler with recovery
let database = Database::new("postgresql://localhost/cloacina")?;
let scheduler = TaskScheduler::with_global_workflows_and_recovery(database).await?;

// Run the scheduling loop
scheduler.run_scheduling_loop().await?;
```

#### Methods

##### <span id="TaskScheduler.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn new(database: Database) -> Result<Self, ValidationError>
```

Creates a new TaskScheduler instance with default configuration using global workflow registry.

This is the recommended constructor for most use cases. The TaskScheduler will:
- Use all workflows registered in the global registry
- Use default poll interval (100ms)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database instance for persistence |

**Returns:**

A new TaskScheduler instance ready to schedule and manage workflow executions.

**Errors:**

May return ValidationError if construction fails.

**Examples:**

```rust
use cloacina::{Database, TaskScheduler};

let database = Database::new("postgresql://localhost/cloacina")?;
let scheduler = TaskScheduler::new(database).await?;
```

<details>
<summary>Source</summary>

```rust
    pub async fn new(database: Database) -> Result<Self, ValidationError> {
        let scheduler = Self::with_poll_interval(database, Duration::from_millis(100)).await?;
        Ok(scheduler)
    }
```

</details>

##### <span id="TaskScheduler.with_poll_interval"></span><code>with_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn with_poll_interval(
    database: Database,
    poll_interval: Duration,
) -> Result<Self, ValidationError>
```

Creates a new TaskScheduler with custom poll interval using global workflow registry.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database instance for persistence |
| `poll_interval` | `Duration` | How often to check for ready tasks |

**Returns:**

A new TaskScheduler instance ready to schedule and manage workflow executions.

<details>
<summary>Source</summary>

```rust
    pub async fn with_poll_interval(
        database: Database,
        poll_interval: Duration,
    ) -> Result<Self, ValidationError> {
        Ok(Self::with_poll_interval_sync(database, poll_interval))
    }
```

</details>

##### <span id="TaskScheduler.with_runtime"></span><code>with_​runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self
```

Sets the runtime for this scheduler, replacing the default.

<details>
<summary>Source</summary>

```rust
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = runtime;
        self
    }
```

</details>

##### <span id="TaskScheduler.runtime"></span><code>runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runtime(&self) -> &Arc<Runtime>
```

Returns a reference to the runtime used by this scheduler.

<details>
<summary>Source</summary>

```rust
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }
```

</details>

##### <span id="TaskScheduler.with_shutdown"></span><code>with_​shutdown</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_shutdown(
    mut self,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> Self
```

Sets the shutdown receiver for graceful termination of the scheduling loop.

<details>
<summary>Source</summary>

```rust
    pub fn with_shutdown(mut self, shutdown_rx: tokio::sync::watch::Receiver<bool>) -> Self {
        self.shutdown_rx = Some(shutdown_rx);
        self
    }
```

</details>

##### <span id="TaskScheduler.with_dispatcher"></span><code>with_​dispatcher</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_dispatcher(mut self, dispatcher: Arc<dyn Dispatcher>) -> Self
```

Sets the dispatcher for push-based task execution.

When a dispatcher is configured, the scheduler will dispatch task events
when tasks become ready, in addition to marking them Ready in the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dispatcher` | `Arc<dyn Dispatcher>` | The dispatcher to use for task events |

**Returns:**

Self for method chaining

<details>
<summary>Source</summary>

```rust
    pub fn with_dispatcher(mut self, dispatcher: Arc<dyn Dispatcher>) -> Self {
        self.dispatcher = Some(dispatcher);
        self
    }
```

</details>

##### <span id="TaskScheduler.dispatcher"></span><code>dispatcher</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn dispatcher(&self) -> Option<&Arc<dyn Dispatcher>>
```

Returns a reference to the dispatcher if configured.

<details>
<summary>Source</summary>

```rust
    pub fn dispatcher(&self) -> Option<&Arc<dyn Dispatcher>> {
        self.dispatcher.as_ref()
    }
```

</details>

##### <span id="TaskScheduler.schedule_workflow_execution"></span><code>schedule_​workflow_​execution</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn schedule_workflow_execution(
    &self,
    workflow_name: &str,
    input_context: Context<serde_json::Value>,
) -> Result<Uuid, ValidationError>
```

Schedules a new workflow execution with the provided input context.

This method:
1. Validates the workflow exists in the registry
2. Stores the input context in the database
3. Creates a new workflow execution record
4. Initializes task execution records for all workflow tasks

**Performance**

This operation performs multiple database transactions:
- Context storage
- Workflow execution creation
- Task execution initialization

All operations are performed in a single transaction for consistency.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `&str` | Name of the workflow to execute |
| `input_context` | `Context<serde_json::Value>` | Context containing input data for the workflow |

**Returns:**

The UUID of the created workflow execution on success.

**Errors:**

Returns `ValidationError::WorkflowNotFound` if the workflow doesn't exist in the registry,
or other validation errors if database operations fail.

**Examples:**

```rust
use cloacina::{Context, TaskScheduler};
use serde_json::json;

let scheduler = TaskScheduler::new(database).await?;
let mut context = Context::new();
context.insert("input", json!({"key": "value"}))?;

let execution_id = scheduler.schedule_workflow_execution("my-workflow", context).await?;
```

<details>
<summary>Source</summary>

```rust
    pub async fn schedule_workflow_execution(
        &self,
        workflow_name: &str,
        input_context: Context<serde_json::Value>,
    ) -> Result<Uuid, ValidationError> {
        info!("Scheduling workflow execution: {}", workflow_name);

        // Look up workflow in scoped runtime registry
        let workflow = match self.runtime.get_workflow(workflow_name) {
            Some(wf) => wf,
            None => return Err(ValidationError::WorkflowNotFound(workflow_name.to_string())),
        };

        let current_version = workflow.metadata().version.clone();
        let last_version = self
            .dal
            .workflow_execution()
            .get_last_version(workflow_name)
            .await?;

        if last_version.as_ref() != Some(&current_version) {
            info!(
                "Workflow '{}' version changed: {} -> {}",
                workflow_name,
                last_version.unwrap_or_else(|| "none".to_string()),
                current_version
            );
        }

        // Store context first (separate operation - needed before main transaction)
        let stored_context = self.dal.context().create(&input_context).await?;

        // Build all task data BEFORE the transaction
        let task_ids = workflow.topological_sort()?;
        let mut task_data: Vec<(String, String, String, i32)> = Vec::with_capacity(task_ids.len());

        for task_id in &task_ids {
            let trigger_rules = self.get_task_trigger_rules(&workflow, task_id);
            let task_config = self.get_task_configuration(&workflow, task_id);
            let max_attempts = workflow
                .get_task(task_id)
                .map(|t| t.retry_policy().max_attempts)
                .unwrap_or(3);

            task_data.push((
                task_id.to_string(),
                trigger_rules.to_string(),
                task_config.to_string(),
                max_attempts,
            ));
        }

        // Prepare workflow execution data
        let workflow_execution_id = UniversalUuid::new_v4();
        let now = UniversalTimestamp::now();
        let wf_name = workflow_name.to_string();
        let wf_version = current_version.clone();

        // Create workflow execution AND tasks in a single atomic transaction
        // This prevents the race condition where the scheduler sees a workflow execution before tasks exist
        crate::dispatch_backend!(
            self.dal.backend(),
            self.create_workflow_execution_postgres(
                workflow_execution_id,
                now,
                wf_name,
                wf_version,
                stored_context,
                task_data,
            )
            .await?,
            self.create_workflow_execution_sqlite(
                workflow_execution_id,
                now,
                wf_name,
                wf_version,
                stored_context,
                task_data,
            )
            .await?
        );

        // NOTE: cloacina_active_workflows gauge is SQL-derived — re-seeded
        // each tick in SchedulerLoop::process_active_executions from the
        // workflow_executions row count. We intentionally do NOT increment
        // here; doing so would cause gauge drift on any code path that skips
        // finalize_workflow_execution (crash, claim loss, etc.).
        info!("Workflow execution scheduled: {}", workflow_execution_id);
        Ok(workflow_execution_id.into())
    }
```

</details>

##### <span id="TaskScheduler.run_scheduling_loop"></span><code>run_​scheduling_​loop</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn run_scheduling_loop(&self) -> Result<(), ValidationError>
```

Runs the main scheduling loop that continuously processes active workflow executions.

This loop:
1. Checks for active workflow executions
2. Updates task readiness based on dependencies and trigger rules
3. Marks completed workflow executions
4. Repeats every second

**Performance**

The scheduling loop:
- Runs every second by default
- Processes all active workflow executions in each iteration
- Uses efficient batch queries where possible
- Implements backoff for database errors

**Thread Safety**

The scheduling loop is designed to be run in a separate thread or task.
Multiple instances should not be run simultaneously.

**Returns:**

This method runs indefinitely until an error occurs.

**Errors:**

Returns validation errors if database operations fail during the scheduling loop.
The loop will continue running on non-fatal errors, with errors logged.

**Examples:**

```rust
use cloacina::TaskScheduler;

let scheduler = TaskScheduler::with_global_workflows(database);
scheduler.run_scheduling_loop().await?;
```

<details>
<summary>Source</summary>

```rust
    pub async fn run_scheduling_loop(&self) -> Result<(), ValidationError> {
        let mut scheduler_loop = SchedulerLoop::with_dispatcher(
            &self.dal,
            self.runtime.clone(),
            self.instance_id,
            self.poll_interval,
            self.dispatcher.clone(),
        );
        if let Some(ref shutdown_rx) = self.shutdown_rx {
            scheduler_loop = scheduler_loop.with_shutdown(shutdown_rx.clone());
        }
        scheduler_loop.run().await
    }
```

</details>

##### <span id="TaskScheduler.process_active_executions"></span><code>process_​active_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn process_active_executions(&self) -> Result<(), ValidationError>
```

Processes all active workflow executions to update task readiness.

<details>
<summary>Source</summary>

```rust
    pub async fn process_active_executions(&self) -> Result<(), ValidationError> {
        let scheduler_loop = SchedulerLoop::with_dispatcher(
            &self.dal,
            self.runtime.clone(),
            self.instance_id,
            self.poll_interval,
            self.dispatcher.clone(),
        );
        scheduler_loop.process_active_executions().await
    }
```

</details>
