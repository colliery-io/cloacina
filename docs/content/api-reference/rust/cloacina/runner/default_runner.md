# cloacina::runner::default_runner <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Default runner for workflow execution.

This module provides the DefaultRunner which coordinates workflow scheduling
and task execution. It combines the functionality of the TaskScheduler and
TaskExecutor into a unified interface.

## Structs

### `cloacina::runner::default_runner::DefaultRunner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Default runner that coordinates workflow scheduling and task execution.

Holds only top-level state — runtime, database, config, and the task
scheduler. All background services (cron, recovery, registry reconciler,
graph scheduler, stale-claim sweeper, ...) live inside the
[`ServiceManager`], which owns their handles and shutdown wiring.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `runtime` | `Arc < Runtime >` | Scoped runtime holding isolated registries for tasks, workflows, and triggers |
| `database` | `Database` | Database connection for persistence and state management |
| `config` | `DefaultRunnerConfig` | Configuration parameters for the runner |
| `scheduler` | `Arc < TaskScheduler >` | Task scheduler for managing workflow execution scheduling |
| `service_manager` | `Arc < RwLock < ServiceManager > >` | Owns the lifecycle of every background service plus typed accessor slots. |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn new (database_url : & str) -> Result < Self , WorkflowExecutionError >
```

Creates a new default runner with default configuration

<details>
<summary>Source</summary>

```rust
    pub async fn new(database_url: &str) -> Result<Self, WorkflowExecutionError> {
        Self::with_config(database_url, DefaultRunnerConfig::default()).await
    }
```

</details>



##### `builder` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn builder () -> DefaultRunnerBuilder
```

Creates a builder for configuring the executor

<details>
<summary>Source</summary>

```rust
    pub fn builder() -> DefaultRunnerBuilder {
        DefaultRunnerBuilder::new()
    }
```

</details>



##### `with_schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn with_schema (database_url : & str , schema : & str ,) -> Result < Self , WorkflowExecutionError >
```

Creates a new executor with PostgreSQL schema-based multi-tenancy

<details>
<summary>Source</summary>

```rust
    pub async fn with_schema(
        database_url: &str,
        schema: &str,
    ) -> Result<Self, WorkflowExecutionError> {
        Self::builder()
            .database_url(database_url)
            .schema(schema)
            .build()
            .await
    }
```

</details>



##### `with_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn with_config (database_url : & str , config : DefaultRunnerConfig ,) -> Result < Self , WorkflowExecutionError >
```

Creates a new unified executor with custom configuration

<details>
<summary>Source</summary>

```rust
    pub async fn with_config(
        database_url: &str,
        config: DefaultRunnerConfig,
    ) -> Result<Self, WorkflowExecutionError> {
        // Initialize database
        let database = Database::new(database_url, "cloacina", config.db_pool_size());

        // Run migrations
        database
            .run_migrations()
            .await
            .map_err(|e| WorkflowExecutionError::DatabaseConnection { message: e })?;

        Self::with_database(database, config, None).await
    }
```

</details>



##### `with_database` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn with_database (database : Database , config : DefaultRunnerConfig , shared_runtime : Option < Arc < Runtime > > ,) -> Result < Self , WorkflowExecutionError >
```

CLOACI-T-0580: construct a runner around a pre-built `Database`, optionally sharing an inventory-seeded `Runtime` across runners.

Used by `TenantRunnerCache` to spin up per-tenant `DefaultRunner`
instances: each tenant has its own `Database` (from
`TenantDatabaseCache`) but they all share the same `Runtime`
`Arc` so inventory (TaskRegistry, WorkflowRegistry, etc.) isn't
duplicated per-tenant.
Migrations are NOT run by this path — the caller must have
already migrated the schema (the tenant-creation flow does this
in `DatabaseAdmin::create_tenant`).

<details>
<summary>Source</summary>

```rust
    pub async fn with_database(
        database: Database,
        config: DefaultRunnerConfig,
        shared_runtime: Option<Arc<Runtime>>,
    ) -> Result<Self, WorkflowExecutionError> {
        let runtime = shared_runtime.unwrap_or_else(|| Arc::new(Runtime::new()));

        // Create scheduler with the scoped runtime
        let scheduler =
            TaskScheduler::with_poll_interval(database.clone(), config.scheduler_poll_interval())
                .await
                .map_err(|e| WorkflowExecutionError::Executor(e.into()))?
                .with_runtime(runtime.clone());

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: config.max_concurrent_tasks(),
            task_timeout: config.task_timeout(),
            enable_claiming: config.enable_claiming(),
            heartbeat_interval: config.heartbeat_interval(),
        };

        let executor = ThreadTaskExecutor::with_runtime_and_registry(
            database.clone(),
            Arc::new(crate::task::TaskRegistry::new()),
            runtime.clone(),
            executor_config,
        );

        // Configure dispatcher for push-based task execution. Every task is sent
        // to the one configured executor key (CLOACI-T-0640).
        let dal = DAL::new(database.clone());
        let dispatcher = DefaultDispatcher::new(dal, config.default_executor());

        dispatcher.register_executor("default", Arc::new(executor) as Arc<dyn TaskExecutor>);

        let scheduler = scheduler.with_dispatcher(Arc::new(dispatcher));

        let default_runner = Self {
            runtime,
            database,
            config,
            scheduler: Arc::new(scheduler),
            service_manager: Arc::new(RwLock::new(ServiceManager::new())),
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
    }
```

</details>



##### `database` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn database (& self) -> & Database
```

Returns a reference to the database.

<details>
<summary>Source</summary>

```rust
    pub fn database(&self) -> &Database {
        &self.database
    }
```

</details>



##### `dal` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn dal (& self) -> DAL
```

Returns the DAL for database operations.

<details>
<summary>Source</summary>

```rust
    pub fn dal(&self) -> DAL {
        DAL::new(self.database.clone())
    }
```

</details>



##### `register_executor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn register_executor (& self , key : & str , executor : Arc < dyn TaskExecutor >) -> bool
```

Register an additional `TaskExecutor` on the runner's dispatcher under an executor key (CLOACI-T-0633). The `"default"` thread executor is registered at construction; this lets a host (e.g. `cloacina-server`) plug in extra backends — notably the `FleetExecutor` under key `"fleet"` — so the server can select it via `default_executor` (CLOACI-T-0640).

Returns `true` if registered, `false` if the scheduler has no
dispatcher configured (push-based execution disabled).

<details>
<summary>Source</summary>

```rust
    pub fn register_executor(&self, key: &str, executor: Arc<dyn TaskExecutor>) -> bool {
        if let Some(dispatcher) = self.scheduler.dispatcher() {
            dispatcher.register_executor(key, executor);
            true
        } else {
            false
        }
    }
```

</details>



##### `has_executor` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn has_executor (& self , key : & str) -> bool
```

Returns `true` if an executor is registered under `key` on this runner's dispatcher. Returns `false` when no dispatcher is configured (push-based execution disabled). Used to validate a configured `default_executor` key at startup (CLOACI-T-0640).

<details>
<summary>Source</summary>

```rust
    pub fn has_executor(&self, key: &str) -> bool {
        self.scheduler
            .dispatcher()
            .map(|d| d.has_executor(key))
            .unwrap_or(false)
    }
```

</details>



##### `runtime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runtime (& self) -> Arc < Runtime >
```

Returns a handle to the scoped `Runtime` this runner uses.

<details>
<summary>Source</summary>

```rust
    pub fn runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }
```

</details>



##### `unified_scheduler` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn unified_scheduler (& self) -> Option < Arc < Scheduler > >
```

Returns the unified scheduler if enabled.

<details>
<summary>Source</summary>

```rust
    pub async fn unified_scheduler(&self) -> Option<Arc<Scheduler>> {
        self.service_manager.read().await.unified_scheduler.clone()
    }
```

</details>



##### `set_graph_scheduler` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn set_graph_scheduler (& self , scheduler : Arc < crate :: computation_graph :: scheduler :: ComputationGraphScheduler > ,)
```

Set the graph scheduler for computation graph package routing. Must be called before `start_services()` so the reconciler can route CG packages.

<details>
<summary>Source</summary>

```rust
    pub async fn set_graph_scheduler(
        &self,
        scheduler: Arc<crate::computation_graph::scheduler::ComputationGraphScheduler>,
    ) {
        let slot = self.service_manager.read().await.graph_scheduler.clone();
        *slot.write().await = Some(scheduler);
    }
```

</details>



##### `shutdown` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn shutdown (& self) -> Result < () , WorkflowExecutionError >
```

Gracefully shuts down the executor and its background services.

<details>
<summary>Source</summary>

```rust
    pub async fn shutdown(&self) -> Result<(), WorkflowExecutionError> {
        self.service_manager.write().await.shutdown_all().await?;
        // Close the database connection pool to release all connections
        self.database.close();
        Ok(())
    }
```

</details>
