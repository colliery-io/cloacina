# cloacina::executor::types <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Core types and structures for the Cloacina task execution system.

This module defines the fundamental types used throughout the executor system,
including execution scopes, dependency management, and configuration structures.
These types are used to coordinate task execution, manage dependencies between tasks,
and configure the behavior of the execution engine.

## Structs

### `cloacina::executor::types::ExecutionScope`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Execution scope information for a context

This structure holds metadata about the current execution context, including
identifiers for both pipeline and task executions. It is used to track and
correlate execution contexts throughout the system.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `pipeline_execution_id` | `UniversalUuid` | Unique identifier for the pipeline execution |
| `task_execution_id` | `Option < UniversalUuid >` | Optional unique identifier for the specific task execution |
| `task_name` | `Option < String >` | Optional name of the task being executed |



### `cloacina::executor::types::DependencyLoader`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

Dependency loader for automatic context merging with lazy loading

This structure manages the loading and caching of task dependencies,
implementing a "latest wins" strategy for context merging. It provides
thread-safe access to dependency contexts through a read-write lock.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | Database connection for loading dependency data |
| `pipeline_execution_id` | `UniversalUuid` | ID of the pipeline execution being processed |
| `dependency_tasks` | `Vec < crate :: task :: TaskNamespace >` | List of task namespaces that this loader depends on |
| `loaded_contexts` | `RwLock < HashMap < String , HashMap < String , serde_json :: Value > > >` | Thread-safe cache of loaded dependency contexts |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (database : Database , pipeline_execution_id : UniversalUuid , dependency_tasks : Vec < crate :: task :: TaskNamespace > ,) -> Self
```

Creates a new dependency loader instance

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | Database connection for loading dependencies |
| `pipeline_execution_id` | `-` | ID of the pipeline execution |
| `dependency_tasks` | `-` | List of task namespaces that this loader depends on |


<details>
<summary>Source</summary>

```rust
    pub fn new(
        database: Database,
        pipeline_execution_id: UniversalUuid,
        dependency_tasks: Vec<crate::task::TaskNamespace>,
    ) -> Self {
        Self {
            database,
            pipeline_execution_id,
            dependency_tasks,
            loaded_contexts: RwLock::new(HashMap::new()),
        }
    }
```

</details>



##### `load_from_dependencies` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn load_from_dependencies (& self , key : & str ,) -> Result < Option < serde_json :: Value > , ExecutorError >
```

Loads a value from dependency contexts using a "latest wins" strategy

This method searches through all dependency contexts in reverse order,
returning the first matching value found. If no value is found, returns None.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `-` | The key to look up in the dependency contexts |


**Returns:**

* `Result<Option<serde_json::Value>, ExecutorError>` - The found value or None if not found

<details>
<summary>Source</summary>

```rust
    pub async fn load_from_dependencies(
        &self,
        key: &str,
    ) -> Result<Option<serde_json::Value>, ExecutorError> {
        // Search dependencies in reverse order (latest wins for overwrites)
        for dep_task_namespace in self.dependency_tasks.iter().rev() {
            let dep_task_name = dep_task_namespace.to_string();
            // Check cache first (read lock)
            {
                let cache = self.loaded_contexts.read().await;
                if let Some(context_data) = cache.get(&dep_task_name) {
                    if let Some(value) = context_data.get(key) {
                        return Ok(Some(value.clone())); // Found! (overwrite strategy)
                    }
                }
            }

            // Lazy load dependency context if not cached (write lock)
            {
                let mut cache = self.loaded_contexts.write().await;
                if !cache.contains_key(&dep_task_name) {
                    let dep_context_data = self
                        .load_dependency_context_data(dep_task_namespace)
                        .await?;
                    cache.insert(dep_task_name.clone(), dep_context_data);
                }

                // Check the newly loaded context
                if let Some(context_data) = cache.get(&dep_task_name) {
                    if let Some(value) = context_data.get(key) {
                        return Ok(Some(value.clone())); // Found! (overwrite strategy)
                    }
                }
            }
        }

        Ok(None) // Key not found in any dependency
    }
```

</details>



##### `load_dependency_context_data` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn load_dependency_context_data (& self , task_namespace : & crate :: task :: TaskNamespace ,) -> Result < HashMap < String , serde_json :: Value > , ExecutorError >
```

Loads the context data for a specific dependency task

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_namespace` | `-` | Namespace of the task to load context data for |


**Returns:**

* `Result<HashMap<String, serde_json::Value>, ExecutorError>` - The loaded context data

<details>
<summary>Source</summary>

```rust
    async fn load_dependency_context_data(
        &self,
        task_namespace: &crate::task::TaskNamespace,
    ) -> Result<HashMap<String, serde_json::Value>, ExecutorError> {
        let dal = DAL::new(self.database.clone());
        let task_metadata = dal
            .task_execution_metadata()
            .get_by_pipeline_and_task(self.pipeline_execution_id, task_namespace)
            .await?;

        if let Some(context_id) = task_metadata.context_id {
            let context = dal.context().read::<serde_json::Value>(context_id).await?;
            Ok(context.data().clone())
        } else {
            // Task has no output context
            Ok(HashMap::new())
        }
    }
```

</details>





### `cloacina::executor::types::ExecutorConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Configuration settings for the executor

This structure defines various parameters that control the behavior of the
task execution system, including concurrency limits and timing parameters.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_concurrent_tasks` | `usize` | Maximum number of tasks that can run concurrently |
| `task_timeout` | `std :: time :: Duration` | Maximum time a task is allowed to run before timing out |
| `enable_claiming` | `bool` | Enable runner-level task claiming for horizontal scaling.
When enabled, the executor claims tasks before executing and heartbeats during. |
| `heartbeat_interval` | `std :: time :: Duration` | Heartbeat interval for claimed tasks (only used when claiming is enabled). |



### `cloacina::executor::types::ClaimedTask`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`

Represents a task that has been claimed for execution

This structure contains the metadata for a task that has been claimed
by an executor instance and is ready to be processed.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_execution_id` | `UniversalUuid` | Unique identifier for this task execution |
| `pipeline_execution_id` | `UniversalUuid` | ID of the pipeline this task belongs to |
| `task_name` | `String` | Name of the task being executed |
| `attempt` | `i32` | Current attempt number for this task execution |
