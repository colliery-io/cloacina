# cloacina::executor::thread_task_executor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Task Executor Module

This module provides the core task execution functionality for the Cloacina pipeline system.
The ThreadTaskExecutor implements the `TaskExecutor` trait for dispatcher-based execution.
The executor is responsible for:
- Executing tasks with proper timeout handling
- Managing task retries and error handling
- Maintaining task execution state
- Handling task dependencies and context management

## Structs

### `cloacina::executor::thread_task_executor::ThreadTaskExecutor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


ThreadTaskExecutor is a thread-based implementation of task execution.

This executor runs tasks in the current thread/process and manages:
- Task execution with timeout handling
- Context management and dependency resolution
- Error handling and retry logic
- State persistence
The executor maintains its own instance ID for tracking and logging purposes
and uses a task registry to resolve task implementations.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database connection pool for task state persistence |
| `dal` | `DAL` | Data Access Layer for database operations |
| `task_registry` | `Arc < TaskRegistry >` | Registry of available task implementations |
| `runtime` | `Arc < Runtime >` | Scoped runtime for task lookup (used in dispatcher execute path) |
| `instance_id` | `UniversalUuid` | Unique identifier for this executor instance |
| `config` | `ExecutorConfig` | Configuration parameters for executor behavior |
| `semaphore` | `Arc < Semaphore >` | Semaphore controlling concurrent task execution slots |
| `total_executed` | `AtomicU64` | Metrics: total tasks executed |
| `total_failed` | `AtomicU64` | Metrics: total tasks failed |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (database : Database , task_registry : Arc < TaskRegistry > , config : ExecutorConfig ,) -> Self
```

Creates a new ThreadTaskExecutor instance.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | Database connection pool for task state persistence |
| `task_registry` | `-` | Registry containing available task implementations |
| `config` | `-` | Configuration parameters for executor behavior |


**Returns:**

A new TaskExecutor instance with a randomly generated instance ID

<details>
<summary>Source</summary>

```rust
    pub fn new(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        config: ExecutorConfig,
    ) -> Self {
        Self::with_runtime_and_registry(database, task_registry, Arc::new(Runtime::new()), config)
    }
```

</details>



##### `with_runtime_and_registry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_runtime_and_registry (database : Database , task_registry : Arc < TaskRegistry > , runtime : Arc < Runtime > , config : ExecutorConfig ,) -> Self
```

Creates a new ThreadTaskExecutor with a specific runtime.

<details>
<summary>Source</summary>

```rust
    pub fn with_runtime_and_registry(
        database: Database,
        task_registry: Arc<TaskRegistry>,
        runtime: Arc<Runtime>,
        config: ExecutorConfig,
    ) -> Self {
        let dal = DAL::new(database.clone());
        let max_concurrent = config.max_concurrent_tasks;

        Self {
            database,
            dal,
            task_registry,
            runtime,
            instance_id: UniversalUuid::new_v4(),
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            total_executed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }
```

</details>



##### `with_runtime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_runtime (mut self , runtime : Arc < Runtime >) -> Self
```

Sets the runtime for this executor, replacing the default.

<details>
<summary>Source</summary>

```rust
    pub fn with_runtime(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime = runtime;
        self
    }
```

</details>



##### `with_global_registry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_global_registry (database : Database , config : ExecutorConfig ,) -> Result < Self , crate :: error :: RegistrationError >
```

Creates a TaskExecutor using the global task registry.

This method is useful when you want to use tasks registered through the global registry
rather than providing a custom registry.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | Database connection pool for task state persistence |
| `config` | `-` | Configuration parameters for executor behavior |


**Returns:**

Result containing either a new TaskExecutor instance or a RegistrationError

<details>
<summary>Source</summary>

```rust
    pub fn with_global_registry(
        database: Database,
        config: ExecutorConfig,
    ) -> Result<Self, crate::error::RegistrationError> {
        let mut registry = TaskRegistry::new();
        let global_registry = crate::global_task_registry();
        let global_tasks = global_registry.read();

        for (namespace, constructor) in global_tasks.iter() {
            let task = constructor();
            registry.register_arc(namespace.clone(), task)?;
        }

        Ok(Self::new(database, Arc::new(registry), config))
    }
```

</details>



##### `semaphore` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn semaphore (& self) -> & Arc < Semaphore >
```

Returns a reference to the concurrency semaphore.

Used by TaskHandle to release and reclaim concurrency slots
during deferred execution.

<details>
<summary>Source</summary>

```rust
    pub fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }
```

</details>



##### `build_task_context` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn build_task_context (& self , claimed_task : & ClaimedTask , dependencies : & [crate :: task :: TaskNamespace] ,) -> Result < Context < serde_json :: Value > , ExecutorError >
```

Builds the execution context for a task by loading its dependencies.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The task to build context for |
| `dependencies` | `-` | Task dependencies |


**Returns:**

Result containing the task's execution context

<details>
<summary>Source</summary>

```rust
    async fn build_task_context(
        &self,
        claimed_task: &ClaimedTask,
        dependencies: &[crate::task::TaskNamespace],
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        // Debug: Log dependencies for troubleshooting
        tracing::debug!(
            "Building context for task '{}' with {} dependencies: {:?}",
            claimed_task.task_name,
            dependencies.len(),
            dependencies
        );
        tracing::debug!(
            "DEBUG: Building context for task '{}' with {} dependencies: {:?}",
            claimed_task.task_name,
            dependencies.len(),
            dependencies
        );
        let execution_scope = ExecutionScope {
            pipeline_execution_id: claimed_task.pipeline_execution_id,
            task_execution_id: Some(claimed_task.task_execution_id),
            task_name: Some(claimed_task.task_name.clone()),
        };

        // Create context for task execution
        // Dependencies are pre-loaded below using batch loading for better performance
        let mut context = Context::new();

        // Track execution scope for logging/metrics (not stored in context)
        let _execution_scope = execution_scope;

        // Load initial pipeline context if task has no dependencies
        if dependencies.is_empty() {
            if let Ok(pipeline_execution) = self
                .dal
                .workflow_execution()
                .get_by_id(claimed_task.pipeline_execution_id)
                .await
            {
                if let Some(context_id) = pipeline_execution.context_id {
                    if let Ok(initial_context) = self
                        .dal
                        .context()
                        .read::<serde_json::Value>(context_id)
                        .await
                    {
                        // Merge initial context data
                        for (key, value) in initial_context.data() {
                            let _ = context.insert(key, value.clone());
                        }
                        debug!(
                            "Loaded initial pipeline context with {} keys",
                            initial_context.data().len()
                        );
                    }
                }
            }
        }

        // Batch load dependency contexts in a single query (eager loading strategy)
        // This provides better performance for tasks that access many dependency values
        if !dependencies.is_empty() {
            debug!(
                "Loading dependency contexts for {} dependencies: {:?}",
                dependencies.len(),
                dependencies
            );
            if let Ok(dep_metadata_with_contexts) = self
                .dal
                .task_execution_metadata()
                .get_dependency_metadata_with_contexts(
                    claimed_task.pipeline_execution_id,
                    dependencies,
                )
                .await
            {
                debug!(
                    "Found {} dependency metadata records",
                    dep_metadata_with_contexts.len()
                );
                for (_task_metadata, context_json) in dep_metadata_with_contexts {
                    if let Some(json_str) = context_json {
                        // Parse the JSON context data
                        if let Ok(dep_context) = Context::<serde_json::Value>::from_json(json_str) {
                            debug!(
                                "Merging dependency context with {} keys: {:?}",
                                dep_context.data().len(),
                                dep_context.data().keys().collect::<Vec<_>>()
                            );
                            // Merge context data (smart merging strategy)
                            for (key, value) in dep_context.data() {
                                if let Some(existing_value) = context.get(key) {
                                    // Key exists - perform smart merging
                                    let merged_value =
                                        Self::merge_context_values(existing_value, value);
                                    let _ = context.update(key, merged_value);
                                } else {
                                    // Key doesn't exist - insert new value
                                    let _ = context.insert(key, value.clone());
                                }
                            }
                        } else {
                            debug!("Failed to parse dependency context JSON");
                        }
                    }
                }
            } else {
                debug!(
                    "Failed to load dependency metadata for dependencies: {:?}",
                    dependencies
                );
            }
        }

        debug!(
            "Final context for task {} has {} keys: {:?}",
            claimed_task.task_name,
            context.data().len(),
            context.data().keys().collect::<Vec<_>>()
        );
        Ok(context)
    }
```

</details>



##### `merge_context_values` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn merge_context_values (existing : & serde_json :: Value , new : & serde_json :: Value ,) -> serde_json :: Value
```

Merges two context values using smart merging strategy.

For arrays: concatenates unique values maintaining order
For objects: merges recursively (latest wins for conflicting keys)
For primitives: latest wins

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `existing` | `-` | The existing value in the context |
| `new` | `-` | The new value from dependency context |


**Returns:**

The merged value

<details>
<summary>Source</summary>

```rust
    fn merge_context_values(
        existing: &serde_json::Value,
        new: &serde_json::Value,
    ) -> serde_json::Value {
        use serde_json::Value;

        match (existing, new) {
            // Both are arrays - concatenate and deduplicate
            (Value::Array(existing_arr), Value::Array(new_arr)) => {
                let mut merged = existing_arr.clone();
                for item in new_arr {
                    if !merged.contains(item) {
                        merged.push(item.clone());
                    }
                }
                Value::Array(merged)
            }
            // Both are objects - merge recursively
            (Value::Object(existing_obj), Value::Object(new_obj)) => {
                let mut merged = existing_obj.clone();
                for (key, value) in new_obj {
                    if let Some(existing_value) = merged.get(key) {
                        merged.insert(
                            key.clone(),
                            Self::merge_context_values(existing_value, value),
                        );
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                Value::Object(merged)
            }
            // For all other cases (different types or primitives), latest wins
            (_, new_value) => new_value.clone(),
        }
    }
```

</details>



##### `execute_with_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn execute_with_timeout (& self , task : & dyn Task , context : Context < serde_json :: Value > ,) -> Result < Context < serde_json :: Value > , ExecutorError >
```

Executes a task with timeout protection.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `-` | The task implementation to execute |
| `context` | `-` | The execution context |


**Returns:**

Result containing either the updated context or an error

<details>
<summary>Source</summary>

```rust
    async fn execute_with_timeout(
        &self,
        task: &dyn Task,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, ExecutorError> {
        match tokio::time::timeout(self.config.task_timeout, task.execute(context)).await {
            Ok(result) => result.map_err(ExecutorError::TaskExecution),
            Err(_) => Err(ExecutorError::TaskTimeout),
        }
    }
```

</details>



##### `handle_task_result` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn handle_task_result (& self , claimed_task : ClaimedTask , result : Result < Context < serde_json :: Value > , ExecutorError > ,) -> Result < () , ExecutorError >
```

Handles the result of task execution.

This method:
- Saves successful task contexts
- Updates task state
- Handles retries for failed tasks
- Logs execution results

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The executed task |
| `result` | `-` | The execution result |


**Returns:**

Result indicating success or failure of result handling

<details>
<summary>Source</summary>

```rust
    async fn handle_task_result(
        &self,
        claimed_task: ClaimedTask,
        result: Result<Context<serde_json::Value>, ExecutorError>,
    ) -> Result<(), ExecutorError> {
        match result {
            Ok(result_context) => {
                // Complete task in a single transaction (save context + mark completed)
                self.complete_task_transaction(&claimed_task, result_context)
                    .await?;

                metrics::counter!("cloacina_tasks_total", "status" => "completed").increment(1);
                info!("Task completed successfully: {}", claimed_task.task_name);
            }
            Err(error) => {
                // Get task retry policy to determine if we should retry.
                // If the task namespace can't be parsed or the task isn't found
                // in the registry, default to no retries (mark as permanently failed).
                let retry_policy = parse_namespace(&claimed_task.task_name)
                    .ok()
                    .and_then(|ns| self.runtime.get_task(&ns))
                    .map(|task| task.retry_policy())
                    .unwrap_or_default();

                // Check if we should retry this task
                if self
                    .should_retry_task(&claimed_task, &error, &retry_policy)
                    .await?
                {
                    self.schedule_task_retry(&claimed_task, &retry_policy)
                        .await?;
                    warn!(
                        "Task failed, scheduled for retry: {} (attempt {})",
                        claimed_task.task_name, claimed_task.attempt
                    );
                } else {
                    // Mark task as permanently failed
                    self.mark_task_failed(claimed_task.task_execution_id, &error)
                        .await?;
                    metrics::counter!("cloacina_tasks_total", "status" => "failed").increment(1);
                    error!(
                        "Task failed permanently: {} - {}",
                        claimed_task.task_name, error
                    );
                }
            }
        }

        Ok(())
    }
```

</details>



##### `save_task_context` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn save_task_context (& self , claimed_task : & ClaimedTask , context : Context < serde_json :: Value > ,) -> Result < () , ExecutorError >
```

Saves the task's execution context to the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The task whose context to save |
| `context` | `-` | The context to save |


**Returns:**

Result indicating success or failure of the save operation

<details>
<summary>Source</summary>

```rust
    async fn save_task_context(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        use crate::models::task_execution_metadata::NewTaskExecutionMetadata;

        // Save context data to the contexts table
        let context_id = self.dal.context().create(&context).await?;

        // Create task execution metadata record with reference to context
        let task_metadata_record = NewTaskExecutionMetadata {
            task_execution_id: claimed_task.task_execution_id,
            pipeline_execution_id: claimed_task.pipeline_execution_id,
            task_name: claimed_task.task_name.clone(),
            context_id,
        };

        self.dal
            .task_execution_metadata()
            .upsert_task_execution_metadata(task_metadata_record)
            .await?;

        let key_count = context.data().len();
        let keys: Vec<_> = context.data().keys().collect();
        info!(
            "Context saved: {} (pipeline: {}, {} keys: {:?}, context_id: {:?})",
            claimed_task.task_name, claimed_task.pipeline_execution_id, key_count, keys, context_id
        );
        Ok(())
    }
```

</details>



##### `mark_task_completed` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn mark_task_completed (& self , task_execution_id : UniversalUuid ,) -> Result < () , ExecutorError >
```

Marks a task as completed in the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `-` | ID of the task to mark as completed |


**Returns:**

Result indicating success or failure of the operation

<details>
<summary>Source</summary>

```rust
    async fn mark_task_completed(
        &self,
        task_execution_id: UniversalUuid,
    ) -> Result<(), ExecutorError> {
        // Get task info for logging before updating
        let task = self
            .dal
            .task_execution()
            .get_by_id(task_execution_id)
            .await?;

        self.dal
            .task_execution()
            .mark_completed(task_execution_id)
            .await?;

        info!(
            "Task state change: {} -> Completed (task: {}, pipeline: {})",
            task.status, task.task_name, task.pipeline_execution_id
        );
        Ok(())
    }
```

</details>



##### `complete_task_transaction` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn complete_task_transaction (& self , claimed_task : & ClaimedTask , context : Context < serde_json :: Value > ,) -> Result < () , ExecutorError >
```

Completes a task by saving its context and marking it as completed.

These two operations (context save + status update) are performed sequentially.
If context save succeeds but mark_completed fails, the error is logged at
ERROR level with the context_id so the inconsistency can be diagnosed. The
stale claim sweeper will eventually reset the task to Ready, but the context
is already persisted and will not be lost.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The task to complete |
| `context` | `-` | The execution context to save |


**Returns:**

Result indicating success or failure of the operation

<details>
<summary>Source</summary>

```rust
    async fn complete_task_transaction(
        &self,
        claimed_task: &ClaimedTask,
        context: Context<serde_json::Value>,
    ) -> Result<(), ExecutorError> {
        // Save context and update metadata first (idempotent via upsert)
        self.save_task_context(claimed_task, context).await?;

        // Mark task as completed — if this fails after context save, log critically
        if let Err(e) = self
            .mark_task_completed(claimed_task.task_execution_id)
            .await
        {
            error!(
                task_id = %claimed_task.task_execution_id,
                task_name = %claimed_task.task_name,
                pipeline_id = %claimed_task.pipeline_execution_id,
                error = %e,
                "CRITICAL: Context saved but mark_completed failed — task may be re-executed by stale claim sweeper"
            );
            return Err(e);
        }

        Ok(())
    }
```

</details>



##### `mark_task_failed` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn mark_task_failed (& self , task_execution_id : UniversalUuid , error : & ExecutorError ,) -> Result < () , ExecutorError >
```

Marks a task as failed in the database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `-` | ID of the task to mark as failed |
| `error` | `-` | The error that caused the failure |


**Returns:**

Result indicating success or failure of the operation

<details>
<summary>Source</summary>

```rust
    async fn mark_task_failed(
        &self,
        task_execution_id: UniversalUuid,
        error: &ExecutorError,
    ) -> Result<(), ExecutorError> {
        // Get task info for logging before updating
        let task = self
            .dal
            .task_execution()
            .get_by_id(task_execution_id)
            .await?;

        self.dal
            .task_execution()
            .mark_failed(task_execution_id, &error.to_string())
            .await?;

        error!(
            "Task state change: {} -> Failed (task: {}, pipeline: {}, error: {})",
            task.status, task.task_name, task.pipeline_execution_id, error
        );

        Ok(())
    }
```

</details>



##### `should_retry_task` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn should_retry_task (& self , claimed_task : & ClaimedTask , error : & ExecutorError , retry_policy : & RetryPolicy ,) -> Result < bool , ExecutorError >
```

Determines if a failed task should be retried.

Considers:
- Maximum retry attempts
- Retry policy conditions
- Error type and patterns

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The failed task |
| `error` | `-` | The error that caused the failure |
| `retry_policy` | `-` | The task's retry policy |


**Returns:**

Result containing a boolean indicating whether to retry

<details>
<summary>Source</summary>

```rust
    async fn should_retry_task(
        &self,
        claimed_task: &ClaimedTask,
        error: &ExecutorError,
        retry_policy: &RetryPolicy,
    ) -> Result<bool, ExecutorError> {
        // Check if we've exceeded max retry attempts
        if claimed_task.attempt >= retry_policy.max_attempts {
            debug!(
                "Task {} exceeded max retry attempts ({}/{})",
                claimed_task.task_name, claimed_task.attempt, retry_policy.max_attempts
            );
            return Ok(false);
        }

        // Check retry conditions (all must be satisfied)
        let should_retry = retry_policy
            .retry_conditions
            .iter()
            .all(|condition| match condition {
                RetryCondition::Never => false,
                RetryCondition::AllErrors => true,
                RetryCondition::TransientOnly => self.is_transient_error(error),
                RetryCondition::ErrorPattern { patterns } => {
                    let error_msg = error.to_string().to_lowercase();
                    patterns
                        .iter()
                        .any(|pattern| error_msg.contains(&pattern.to_lowercase()))
                }
            });

        debug!(
            "Retry decision for task {}: {} (conditions: {:?}, error: {})",
            claimed_task.task_name, should_retry, retry_policy.retry_conditions, error
        );

        Ok(should_retry)
    }
```

</details>



##### `is_transient_error` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn is_transient_error (& self , error : & ExecutorError) -> bool
```

Determines if an error is transient and potentially retryable.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `error` | `-` | The error to check |


**Returns:**

Boolean indicating if the error is transient

<details>
<summary>Source</summary>

```rust
    fn is_transient_error(&self, error: &ExecutorError) -> bool {
        match error {
            ExecutorError::TaskTimeout => true,
            ExecutorError::Database(_) => true,
            ExecutorError::ConnectionPool(_) => true,
            ExecutorError::TaskNotFound(_) => false,
            ExecutorError::TaskExecution(task_error) => {
                // Check for common transient error patterns in task errors
                let error_msg = task_error.to_string().to_lowercase();
                error_msg.contains("timeout")
                    || error_msg.contains("connection")
                    || error_msg.contains("network")
                    || error_msg.contains("temporary")
                    || error_msg.contains("unavailable")
            }
            _ => false,
        }
    }
```

</details>



##### `schedule_task_retry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn schedule_task_retry (& self , claimed_task : & ClaimedTask , retry_policy : & RetryPolicy ,) -> Result < () , ExecutorError >
```

Schedules a task for retry execution.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `claimed_task` | `-` | The task to retry |
| `retry_policy` | `-` | The task's retry policy |


**Returns:**

Result indicating success or failure of retry scheduling

<details>
<summary>Source</summary>

```rust
    async fn schedule_task_retry(
        &self,
        claimed_task: &ClaimedTask,
        retry_policy: &RetryPolicy,
    ) -> Result<(), ExecutorError> {
        // Calculate retry delay using the backoff strategy
        let retry_delay = retry_policy.calculate_delay(claimed_task.attempt);
        let retry_at = Utc::now() + retry_delay;

        // Use DAL to schedule retry
        self.dal
            .task_execution()
            .schedule_retry(
                claimed_task.task_execution_id,
                crate::database::UniversalTimestamp(retry_at),
                claimed_task.attempt + 1,
            )
            .await?;

        info!(
            "Scheduled retry for task {} in {:?} (attempt {})",
            claimed_task.task_name,
            retry_delay,
            claimed_task.attempt + 1
        );

        Ok(())
    }
```

</details>
