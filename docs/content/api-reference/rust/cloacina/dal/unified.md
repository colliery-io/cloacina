# cloacina::dal::unified <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Unified Data Access Layer with runtime backend selection

This module provides a unified DAL implementation that works with both
PostgreSQL and SQLite backends, selecting the appropriate implementation
at runtime based on the database connection type.

**Examples:**

```rust,ignore
use cloacina::dal::unified::DAL;
use cloacina::database::Database;

// Create database with runtime backend detection
let db = Database::new("postgres://localhost/mydb", "mydb", 10);
let dal = DAL::new(db);

// Operations automatically use the correct backend
let contexts = dal.context().list().await?;
```

## Structs

### `cloacina::dal::unified::DAL`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Clone`, `Debug`

Helper macro for dispatching operations based on backend type.

This macro simplifies writing code that needs to execute different
implementations based on the database backend.
The unified Data Access Layer struct.
This struct provides access to all database operations through a single
interface that works with both PostgreSQL and SQLite backends.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database` | `Database` | The database instance with connection pool |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (database : Database) -> Self
```

Creates a new unified DAL instance.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database` | `-` | A Database instance configured for either PostgreSQL or SQLite |


**Returns:**

A new DAL instance ready for database operations.

<details>
<summary>Source</summary>

```rust
    pub fn new(database: Database) -> Self {
        DAL { database }
    }
```

</details>



##### `backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn backend (& self) -> BackendType
```

Returns the backend type for this DAL instance.

<details>
<summary>Source</summary>

```rust
    pub fn backend(&self) -> BackendType {
        self.database.backend()
    }
```

</details>



##### `database` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn database (& self) -> & Database
```

Returns a reference to the underlying database.

<details>
<summary>Source</summary>

```rust
    pub fn database(&self) -> &Database {
        &self.database
    }
```

</details>



##### `pool` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pool (& self) -> AnyPool
```

Returns the connection pool.

<details>
<summary>Source</summary>

```rust
    pub fn pool(&self) -> AnyPool {
        self.database.pool()
    }
```

</details>



##### `api_keys` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn api_keys (& self) -> ApiKeyDAL < '_ >
```

Returns an API key DAL (Postgres only).

<details>
<summary>Source</summary>

```rust
    pub fn api_keys(&self) -> ApiKeyDAL<'_> {
        ApiKeyDAL::new(self)
    }
```

</details>



##### `checkpoint` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn checkpoint (& self) -> CheckpointDAL < '_ >
```

Returns a checkpoint DAL for computation graph state persistence.

<details>
<summary>Source</summary>

```rust
    pub fn checkpoint(&self) -> CheckpointDAL<'_> {
        CheckpointDAL::new(self)
    }
```

</details>



##### `context` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn context (& self) -> ContextDAL < '_ >
```

Returns a context DAL for context operations.

<details>
<summary>Source</summary>

```rust
    pub fn context(&self) -> ContextDAL<'_> {
        ContextDAL::new(self)
    }
```

</details>



##### `workflow_execution` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn workflow_execution (& self) -> WorkflowExecutionDAL < '_ >
```

Returns a workflow execution DAL for workflow execution operations.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_execution(&self) -> WorkflowExecutionDAL<'_> {
        WorkflowExecutionDAL::new(self)
    }
```

</details>



##### `task_execution` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_execution (& self) -> TaskExecutionDAL < '_ >
```

Returns a task execution DAL for task operations.

<details>
<summary>Source</summary>

```rust
    pub fn task_execution(&self) -> TaskExecutionDAL<'_> {
        TaskExecutionDAL::new(self)
    }
```

</details>



##### `task_execution_metadata` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_execution_metadata (& self) -> TaskExecutionMetadataDAL < '_ >
```

Returns a task execution metadata DAL for metadata operations.

<details>
<summary>Source</summary>

```rust
    pub fn task_execution_metadata(&self) -> TaskExecutionMetadataDAL<'_> {
        TaskExecutionMetadataDAL::new(self)
    }
```

</details>



##### `task_outbox` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_outbox (& self) -> TaskOutboxDAL < '_ >
```

Returns a task outbox DAL for work distribution operations.

<details>
<summary>Source</summary>

```rust
    pub fn task_outbox(&self) -> TaskOutboxDAL<'_> {
        TaskOutboxDAL::new(self)
    }
```

</details>



##### `recovery_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn recovery_event (& self) -> RecoveryEventDAL < '_ >
```

Returns a recovery event DAL for recovery operations.

<details>
<summary>Source</summary>

```rust
    pub fn recovery_event(&self) -> RecoveryEventDAL<'_> {
        RecoveryEventDAL::new(self)
    }
```

</details>



##### `execution_event` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn execution_event (& self) -> ExecutionEventDAL < '_ >
```

Returns an execution event DAL for execution event operations.

<details>
<summary>Source</summary>

```rust
    pub fn execution_event(&self) -> ExecutionEventDAL<'_> {
        ExecutionEventDAL::new(self)
    }
```

</details>



##### `schedule` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn schedule (& self) -> ScheduleDAL < '_ >
```

Returns a unified schedule DAL for schedule operations.

<details>
<summary>Source</summary>

```rust
    pub fn schedule(&self) -> ScheduleDAL<'_> {
        ScheduleDAL::new(self)
    }
```

</details>



##### `schedule_execution` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn schedule_execution (& self) -> ScheduleExecutionDAL < '_ >
```

Returns a unified schedule execution DAL for schedule execution operations.

<details>
<summary>Source</summary>

```rust
    pub fn schedule_execution(&self) -> ScheduleExecutionDAL<'_> {
        ScheduleExecutionDAL::new(self)
    }
```

</details>



##### `workflow_packages` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn workflow_packages (& self) -> WorkflowPackagesDAL < '_ >
```

Returns a workflow packages DAL for package operations.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_packages(&self) -> WorkflowPackagesDAL<'_> {
        WorkflowPackagesDAL::new(self)
    }
```

</details>



##### `workflow_registry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn workflow_registry < S : crate :: registry :: traits :: RegistryStorage + 'static > (& self , storage : S ,) -> crate :: registry :: workflow_registry :: WorkflowRegistryImpl < S >
```

Creates a workflow registry implementation with the given storage backend.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage` | `-` | A storage backend implementing `RegistryStorage` |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Panic` | Panics if the workflow registry cannot be created. Use [`try_workflow_registry`](Self::try_workflow_registry) for fallible construction. |


<details>
<summary>Source</summary>

```rust
    pub fn workflow_registry<S: crate::registry::traits::RegistryStorage + 'static>(
        &self,
        storage: S,
    ) -> crate::registry::workflow_registry::WorkflowRegistryImpl<S> {
        self.try_workflow_registry(storage)
            .expect("Failed to create workflow registry")
    }
```

</details>



##### `try_workflow_registry` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn try_workflow_registry < S : crate :: registry :: traits :: RegistryStorage + 'static > (& self , storage : S ,) -> Result < crate :: registry :: workflow_registry :: WorkflowRegistryImpl < S > , crate :: registry :: error :: RegistryError , >
```

Creates a workflow registry implementation with the given storage backend.

This is the fallible version of [`workflow_registry`](Self::workflow_registry).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `storage` | `-` | A storage backend implementing `RegistryStorage` |


**Raises:**

| Exception | Description |
|-----------|-------------|
| `Error` | Returns an error if the workflow registry cannot be initialized. |


<details>
<summary>Source</summary>

```rust
    pub fn try_workflow_registry<S: crate::registry::traits::RegistryStorage + 'static>(
        &self,
        storage: S,
    ) -> Result<
        crate::registry::workflow_registry::WorkflowRegistryImpl<S>,
        crate::registry::error::RegistryError,
    > {
        crate::registry::workflow_registry::WorkflowRegistryImpl::new(
            storage,
            self.database.clone(),
        )
    }
```

</details>
