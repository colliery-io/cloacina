# cloacina::execution_planner <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>




## Structs

### `cloacina::execution_planner::TaskScheduler`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


The main Task Scheduler that manages workflow execution and task readiness.

The TaskScheduler converts Workflow definitions into persistent database execution plans,
tracks task state transitions, and manages dependencies through trigger rules.

**Examples:**

```rust,ignore
use cloacina::{Database, TaskScheduler};
use cloacina::workflow::Workflow;

// Create a new scheduler with recovery
let database = Database::new("postgresql://localhost/cloacina")?;
let scheduler = TaskScheduler::with_global_workflows_and_recovery(database).await?;

// Run the scheduling loop
scheduler.run_scheduling_loop().await?;
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `dal` | `DAL` |  |
| `runtime` | `Arc < Runtime >` |  |
| `instance_id` | `Uuid` |  |
| `poll_interval` | `Duration` |  |
| `dispatcher` | `Option < Arc < dyn Dispatcher > >` | Optional dispatcher for push-based task execution |
| `shutdown_rx` | `Option < tokio :: sync :: watch :: Receiver < bool > >` | Shutdown signal for graceful termination of the scheduling loop. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn new (database : Database) -> Result < Self , ValidationError >
```

Creates a new TaskScheduler instance with default configuration using global workflow registry.

This is the recommended constructor for most use cases. The TaskScheduler will:
- Use all workflows registered in the global registry
- Use default poll interval (100ms)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | Database instance for persistence |


**Returns:**

A new TaskScheduler instance ready to schedule and manage workflow executions.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | May return ValidationError if construction fails. |


**Examples:**

```rust,ignore
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



##### `with_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn with_poll_interval (database : Database , poll_interval : Duration ,) -> Result < Self , ValidationError >
```

Creates a new TaskScheduler with custom poll interval using global workflow registry.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | Database instance for persistence |
| `poll_interval` | `-` | How often to check for ready tasks |


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



##### `with_poll_interval_sync` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">pub(crate)</span>


```rust
fn with_poll_interval_sync (database : Database , poll_interval : Duration) -> Self
```

Creates a new TaskScheduler with custom poll interval (synchronous version).

<details>
<summary>Source</summary>

```rust
    pub(crate) fn with_poll_interval_sync(database: Database, poll_interval: Duration) -> Self {
        let dal = DAL::new(database.clone());

        Self {
            dal,
            runtime: Arc::new(Runtime::new()),
            instance_id: Uuid::new_v4(),
            poll_interval,
            dispatcher: None,
            shutdown_rx: None,
        }
    }
```

</details>



##### `with_runtime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_runtime (mut self , runtime : Arc < Runtime >) -> Self
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



##### `runtime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runtime (& self) -> & Arc < Runtime >
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



##### `with_shutdown` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_shutdown (mut self , shutdown_rx : tokio :: sync :: watch :: Receiver < bool >) -> Self
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



##### `with_dispatcher` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_dispatcher (mut self , dispatcher : Arc < dyn Dispatcher >) -> Self
```

Sets the dispatcher for push-based task execution.

When a dispatcher is configured, the scheduler will dispatch task events
when tasks become ready, in addition to marking them Ready in the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `dispatcher` | `-` | The dispatcher to use for task events |


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



##### `dispatcher` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn dispatcher (& self) -> Option < & Arc < dyn Dispatcher > >
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



##### `schedule_workflow_execution` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn schedule_workflow_execution (& self , workflow_name : & str , input_context : Context < serde_json :: Value > ,) -> Result < Uuid , ValidationError >
```

Schedules a new workflow execution with the provided input context.

This method:
1. Validates the workflow exists in the registry
2. Stores the input context in the database
3. Creates a new workflow execution record
4. Initializes task execution records for all workflow tasks

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `-` | Name of the workflow to execute |
| `input_context` | `-` | Context containing input data for the workflow |


**Returns:**

The UUID of the created workflow execution on success.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns `ValidationError::WorkflowNotFound` if the workflow doesn't exist in the registry, or other validation errors if database operations fail. |


**Examples:**

```rust,ignore
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



##### `create_workflow_execution_postgres` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn create_workflow_execution_postgres (& self , workflow_execution_id : UniversalUuid , now : UniversalTimestamp , workflow_name : String , workflow_version : String , stored_context : Option < UniversalUuid > , task_data : Vec < (String , String , String , i32) > ,) -> Result < () , ValidationError >
```

Creates workflow execution and tasks in PostgreSQL.

<details>
<summary>Source</summary>

```rust
    async fn create_workflow_execution_postgres(
        &self,
        workflow_execution_id: UniversalUuid,
        now: UniversalTimestamp,
        workflow_name: String,
        workflow_version: String,
        stored_context: Option<UniversalUuid>,
        task_data: Vec<(String, String, String, i32)>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database()
            .get_postgres_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction(|conn| {
                // Insert workflow execution
                diesel::insert_into(workflow_executions::table)
                    .values(&NewUnifiedWorkflowExecution {
                        id: workflow_execution_id,
                        workflow_name,
                        workflow_version,
                        status: "Pending".to_string(),
                        context_id: stored_context,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    })
                    .execute(conn)?;

                // Insert all tasks
                for (task_name, trigger_rules, task_config, max_attempts) in task_data {
                    diesel::insert_into(task_executions::table)
                        .values(&NewUnifiedTaskExecution {
                            id: UniversalUuid::new_v4(),
                            workflow_execution_id,
                            task_name,
                            status: "NotStarted".to_string(),
                            attempt: 1,
                            max_attempts,
                            trigger_rules,
                            task_configuration: task_config,
                            created_at: now,
                            updated_at: now,
                        })
                        .execute(conn)?;
                }

                Ok::<_, diesel::result::Error>(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
```

</details>



##### `create_workflow_execution_sqlite` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn create_workflow_execution_sqlite (& self , workflow_execution_id : UniversalUuid , now : UniversalTimestamp , workflow_name : String , workflow_version : String , stored_context : Option < UniversalUuid > , task_data : Vec < (String , String , String , i32) > ,) -> Result < () , ValidationError >
```

Creates workflow execution and tasks in SQLite.

<details>
<summary>Source</summary>

```rust
    async fn create_workflow_execution_sqlite(
        &self,
        workflow_execution_id: UniversalUuid,
        now: UniversalTimestamp,
        workflow_name: String,
        workflow_version: String,
        stored_context: Option<UniversalUuid>,
        task_data: Vec<(String, String, String, i32)>,
    ) -> Result<(), ValidationError> {
        let conn = self
            .dal
            .database()
            .get_sqlite_connection()
            .await
            .map_err(|e| ValidationError::ConnectionPool(e.to_string()))?;

        conn.interact(move |conn| {
            conn.transaction(|conn| {
                // Insert workflow execution
                diesel::insert_into(workflow_executions::table)
                    .values(&NewUnifiedWorkflowExecution {
                        id: workflow_execution_id,
                        workflow_name,
                        workflow_version,
                        status: "Pending".to_string(),
                        context_id: stored_context,
                        started_at: now,
                        created_at: now,
                        updated_at: now,
                    })
                    .execute(conn)?;

                // Insert all tasks
                for (task_name, trigger_rules, task_config, max_attempts) in task_data {
                    diesel::insert_into(task_executions::table)
                        .values(&NewUnifiedTaskExecution {
                            id: UniversalUuid::new_v4(),
                            workflow_execution_id,
                            task_name,
                            status: "NotStarted".to_string(),
                            attempt: 1,
                            max_attempts,
                            trigger_rules,
                            task_configuration: task_config,
                            created_at: now,
                            updated_at: now,
                        })
                        .execute(conn)?;
                }

                Ok::<_, diesel::result::Error>(())
            })
        })
        .await
        .map_err(|e| ValidationError::ConnectionPool(e.to_string()))??;

        Ok(())
    }
```

</details>



##### `run_scheduling_loop` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn run_scheduling_loop (& self) -> Result < () , ValidationError >
```

Runs the main scheduling loop that continuously processes active workflow executions.

This loop:
1. Checks for active workflow executions
2. Updates task readiness based on dependencies and trigger rules
3. Marks completed workflow executions
4. Repeats every second

**Returns:**

This method runs indefinitely until an error occurs.

**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns validation errors if database operations fail during the scheduling loop. The loop will continue running on non-fatal errors, with errors logged. |


**Examples:**

```rust,ignore
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



##### `process_active_executions` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn process_active_executions (& self) -> Result < () , ValidationError >
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



##### `get_task_trigger_rules` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn get_task_trigger_rules (& self , workflow : & Workflow , task_namespace : & TaskNamespace ,) -> serde_json :: Value
```

Gets trigger rules for a specific task from the task implementation.

<details>
<summary>Source</summary>

```rust
    fn get_task_trigger_rules(
        &self,
        workflow: &Workflow,
        task_namespace: &TaskNamespace,
    ) -> serde_json::Value {
        workflow
            .get_task(task_namespace)
            .map(|task| task.trigger_rules())
            .unwrap_or_else(|_| serde_json::json!({"type": "Always"}))
    }
```

</details>



##### `get_task_configuration` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn get_task_configuration (& self , _workflow : & Workflow , _task_namespace : & TaskNamespace ,) -> serde_json :: Value
```

Gets task configuration (currently returns empty object).

<details>
<summary>Source</summary>

```rust
    fn get_task_configuration(
        &self,
        _workflow: &Workflow,
        _task_namespace: &TaskNamespace,
    ) -> serde_json::Value {
        // In the future, this could include task-specific configuration
        serde_json::json!({})
    }
```

</details>
