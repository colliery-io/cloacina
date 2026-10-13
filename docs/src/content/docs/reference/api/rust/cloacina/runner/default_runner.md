---
title: "cloacina::runner::default_runner"
description: "Default runner for workflow execution."
editUrl: false
sidebar:
  order: 178
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Default runner for workflow execution.

This module provides the DefaultRunner which coordinates workflow scheduling
and task execution. It combines the functionality of the TaskScheduler and
TaskExecutor into a unified interface.

**Components**

- `DefaultRunner`: Main runner struct
- `DefaultRunnerConfig`: Configuration options
- `DefaultRunnerBuilder`: Builder for creating runners with custom settings

## Re-exports

- [`DefaultRunnerBuilder`](../../../../rust/cloacina/runner/default_runner/config/#struct.DefaultRunnerBuilder)
- [`DefaultRunnerConfig`](../../../../rust/cloacina/runner/default_runner/config/#struct.DefaultRunnerConfig)
- [`DefaultRunnerConfigBuilder`](../../../../rust/cloacina/runner/default_runner/config/#struct.DefaultRunnerConfigBuilder)

## Structs

### <span id="struct.DefaultRunner"></span><code style="font-size: 0.85em">struct DefaultRunner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Default runner that coordinates workflow scheduling and task execution.

Holds only top-level state — runtime, database, config, and the task
scheduler. All background services (cron, recovery, registry reconciler,
graph scheduler, stale-claim sweeper, ...) live inside the
`ServiceManager`, which owns their handles and shutdown wiring.

**Shutdown**

Call `shutdown()` before dropping to gracefully stop
background tasks and release database connections.

#### Methods

##### <span id="DefaultRunner.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn new(database_url: &str) -> Result<Self, WorkflowExecutionError>
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

##### <span id="DefaultRunner.builder"></span><code>builder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn builder() -> DefaultRunnerBuilder
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

##### <span id="DefaultRunner.with_schema"></span><code>with_​schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn with_schema(
    database_url: &str,
    schema: &str,
) -> Result<Self, WorkflowExecutionError>
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

##### <span id="DefaultRunner.with_config"></span><code>with_​config</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn with_config(
    database_url: &str,
    config: DefaultRunnerConfig,
) -> Result<Self, WorkflowExecutionError>
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

##### <span id="DefaultRunner.with_database"></span><code>with_​database</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn with_database(
    database: Database,
    config: DefaultRunnerConfig,
    shared_runtime: Option<Arc<Runtime>>,
) -> Result<Self, WorkflowExecutionError>
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
        Self::with_database_secrets(database, config, shared_runtime, None).await
    }
```

</details>

##### <span id="DefaultRunner.with_database_secrets"></span><code>with_​database_​secrets</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn with_database_secrets(
    database: Database,
    config: DefaultRunnerConfig,
    shared_runtime: Option<Arc<Runtime>>,
    secret_resolver: Option<Arc<dyn cloacina_workflow::secret::SecretResolver>>,
) -> Result<Self, WorkflowExecutionError>
```

`Self::with_database` plus a secret resolution side channel (CLOACI-T-0858 / I-0133 D-1) threaded onto the in-process thread executor, so `context.secret(...)` resolves for locally-executed tasks. Hosts that construct runners per tenant (cloacina-server's `TenantRunnerCache`) pass a tenant-scoped [`crate::security::SecretStoreResolver`](../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver); `None` keeps the fail-closed "secrets backend not configured" behavior.

<details>
<summary>Source</summary>

```rust
    pub async fn with_database_secrets(
        database: Database,
        config: DefaultRunnerConfig,
        shared_runtime: Option<Arc<Runtime>>,
        secret_resolver: Option<Arc<dyn cloacina_workflow::secret::SecretResolver>>,
    ) -> Result<Self, WorkflowExecutionError> {
        // CLOACI-T-0924: `cloacina-server` hands EVERY tenant's runner the same
        // `Runtime` allocation (`TenantRunnerCache::shared_runtime`), so bind
        // this runner's handle to its own tenant. The registries underneath are
        // keyed `(tenant, name)`; everything this constructor builds
        // (TaskScheduler, ThreadTaskExecutor, and later the reconciler and cron
        // scheduler) clones the handle and therefore inherits the scope, so two
        // tenants shipping a workflow called `pipeline` no longer collide.
        // Registries stay shared — only the view differs.
        let runtime = Arc::new(
            shared_runtime
                .unwrap_or_else(|| Arc::new(Runtime::new()))
                .scoped_to_tenant(config.tenant_id()),
        );

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
        )
        // CLOACI-T-0858: thread the secret side channel so task contexts can
        // resolve secrets on the in-process execution path.
        .with_secret_resolver(secret_resolver);

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
            cron_change: Arc::new(tokio::sync::Notify::new()),
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
    }
```

</details>

##### <span id="DefaultRunner.database"></span><code>database</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn database(&self) -> &Database
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

##### <span id="DefaultRunner.dal"></span><code>dal</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn dal(&self) -> DAL
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

##### <span id="DefaultRunner.register_executor"></span><code>register_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn register_executor(
    &self,
    key: &str,
    executor: Arc<dyn TaskExecutor>,
) -> bool
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

##### <span id="DefaultRunner.has_executor"></span><code>has_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn has_executor(&self, key: &str) -> bool
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

##### <span id="DefaultRunner.runtime"></span><code>runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runtime(&self) -> Arc<Runtime>
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

##### <span id="DefaultRunner.unified_scheduler"></span><code>unified_​scheduler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn unified_scheduler(&self) -> Option<Arc<Scheduler>>
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

##### <span id="DefaultRunner.notify_cron_change"></span><code>notify_​cron_​change</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn notify_cron_change(&self)
```

Wake the timer-driven cron scheduler so it re-reads the due set (CLOACI-T-0927).

The embedded `register_cron_*` / `unregister_*` APIs do this for
themselves. Anything that writes a cron `schedules` row through the DAL
directly — notably the server's named-instance routes — must call this,
or the scheduler keeps sleeping against a stale cached due-time and the
new schedule does not fire.

<details>
<summary>Source</summary>

```rust
    pub fn notify_cron_change(&self) {
        self.cron_change.notify_one();
    }
```

</details>

##### <span id="DefaultRunner.set_graph_scheduler"></span><code>set_​graph_​scheduler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn set_graph_scheduler(
    &self,
    scheduler: Arc<crate::computation_graph::scheduler::ComputationGraphScheduler>,
)
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

##### <span id="DefaultRunner.shutdown"></span><code>shutdown</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn shutdown(&self) -> Result<(), WorkflowExecutionError>
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
