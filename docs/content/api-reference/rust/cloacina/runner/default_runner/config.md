# cloacina::runner::default_runner::config <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Configuration types for the DefaultRunner.

This module contains the configuration structs and builders for
configuring the DefaultRunner's behavior.

## Structs

### `cloacina::runner::default_runner::config::DefaultRunnerConfig`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Configuration for the default runner

This struct defines the configuration parameters that control the behavior
of the DefaultRunner. It includes settings for concurrency, timeouts,
polling intervals, and database connection management.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `max_concurrent_tasks` | `usize` |  |
| `scheduler_poll_interval` | `Duration` |  |
| `task_timeout` | `Duration` |  |
| `pipeline_timeout` | `Option < Duration >` |  |
| `db_pool_size` | `u32` |  |
| `enable_recovery` | `bool` |  |
| `enable_cron_scheduling` | `bool` |  |
| `cron_poll_interval` | `Duration` |  |
| `cron_max_catchup_executions` | `usize` |  |
| `cron_enable_recovery` | `bool` |  |
| `cron_recovery_interval` | `Duration` |  |
| `cron_lost_threshold_minutes` | `i32` |  |
| `cron_max_recovery_age` | `Duration` |  |
| `cron_max_recovery_attempts` | `usize` |  |
| `enable_trigger_scheduling` | `bool` |  |
| `trigger_base_poll_interval` | `Duration` |  |
| `trigger_poll_timeout` | `Duration` |  |
| `enable_registry_reconciler` | `bool` |  |
| `registry_reconcile_interval` | `Duration` |  |
| `registry_enable_startup_reconciliation` | `bool` |  |
| `registry_storage_path` | `Option < std :: path :: PathBuf >` |  |
| `registry_storage_backend` | `String` |  |
| `enable_claiming` | `bool` |  |
| `heartbeat_interval` | `Duration` |  |
| `stale_claim_sweep_interval` | `Duration` |  |
| `stale_claim_threshold` | `Duration` |  |
| `runner_id` | `Option < String >` |  |
| `runner_name` | `Option < String >` |  |
| `routing_config` | `Option < RoutingConfig >` |  |

#### Methods

##### `builder` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn builder () -> DefaultRunnerConfigBuilder
```

Creates a new configuration builder with default values.

<details>
<summary>Source</summary>

```rust
    pub fn builder() -> DefaultRunnerConfigBuilder {
        DefaultRunnerConfigBuilder::default()
    }
```

</details>



##### `max_concurrent_tasks` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn max_concurrent_tasks (& self) -> usize
```

Maximum number of concurrent task executions allowed.

<details>
<summary>Source</summary>

```rust
    pub fn max_concurrent_tasks(&self) -> usize {
        self.max_concurrent_tasks
    }
```

</details>



##### `scheduler_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn scheduler_poll_interval (& self) -> Duration
```

How often the scheduler checks for ready tasks.

<details>
<summary>Source</summary>

```rust
    pub fn scheduler_poll_interval(&self) -> Duration {
        self.scheduler_poll_interval
    }
```

</details>



##### `task_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_timeout (& self) -> Duration
```

Maximum time allowed for a single task to execute.

<details>
<summary>Source</summary>

```rust
    pub fn task_timeout(&self) -> Duration {
        self.task_timeout
    }
```

</details>



##### `pipeline_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pipeline_timeout (& self) -> Option < Duration >
```

Optional maximum time for an entire pipeline execution.

<details>
<summary>Source</summary>

```rust
    pub fn pipeline_timeout(&self) -> Option<Duration> {
        self.pipeline_timeout
    }
```

</details>



##### `db_pool_size` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn db_pool_size (& self) -> u32
```

Number of database connections in the pool.

<details>
<summary>Source</summary>

```rust
    pub fn db_pool_size(&self) -> u32 {
        self.db_pool_size
    }
```

</details>



##### `enable_recovery` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_recovery (& self) -> bool
```

Whether automatic recovery is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn enable_recovery(&self) -> bool {
        self.enable_recovery
    }
```

</details>



##### `enable_cron_scheduling` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_cron_scheduling (& self) -> bool
```

Whether cron scheduling is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn enable_cron_scheduling(&self) -> bool {
        self.enable_cron_scheduling
    }
```

</details>



##### `cron_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_poll_interval (& self) -> Duration
```

Poll interval for cron schedules.

<details>
<summary>Source</summary>

```rust
    pub fn cron_poll_interval(&self) -> Duration {
        self.cron_poll_interval
    }
```

</details>



##### `cron_max_catchup_executions` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_max_catchup_executions (& self) -> usize
```

Maximum catchup executions for missed cron runs.

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.cron_max_catchup_executions
    }
```

</details>



##### `cron_enable_recovery` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_enable_recovery (& self) -> bool
```

Whether cron recovery is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn cron_enable_recovery(&self) -> bool {
        self.cron_enable_recovery
    }
```

</details>



##### `cron_recovery_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_recovery_interval (& self) -> Duration
```

How often to check for lost cron executions.

<details>
<summary>Source</summary>

```rust
    pub fn cron_recovery_interval(&self) -> Duration {
        self.cron_recovery_interval
    }
```

</details>



##### `cron_lost_threshold_minutes` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_lost_threshold_minutes (& self) -> i32
```

Minutes before an execution is considered lost.

<details>
<summary>Source</summary>

```rust
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.cron_lost_threshold_minutes
    }
```

</details>



##### `cron_max_recovery_age` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_max_recovery_age (& self) -> Duration
```

Maximum age of executions to recover.

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_age(&self) -> Duration {
        self.cron_max_recovery_age
    }
```

</details>



##### `cron_max_recovery_attempts` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_max_recovery_attempts (& self) -> usize
```

Maximum recovery attempts per execution.

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.cron_max_recovery_attempts
    }
```

</details>



##### `enable_trigger_scheduling` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_trigger_scheduling (& self) -> bool
```

Whether trigger scheduling is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn enable_trigger_scheduling(&self) -> bool {
        self.enable_trigger_scheduling
    }
```

</details>



##### `trigger_base_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn trigger_base_poll_interval (& self) -> Duration
```

Base poll interval for trigger readiness checks.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_base_poll_interval(&self) -> Duration {
        self.trigger_base_poll_interval
    }
```

</details>



##### `trigger_poll_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn trigger_poll_timeout (& self) -> Duration
```

Timeout for trigger poll operations.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_poll_timeout(&self) -> Duration {
        self.trigger_poll_timeout
    }
```

</details>



##### `enable_registry_reconciler` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_registry_reconciler (& self) -> bool
```

Whether the registry reconciler is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn enable_registry_reconciler(&self) -> bool {
        self.enable_registry_reconciler
    }
```

</details>



##### `registry_reconcile_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_reconcile_interval (& self) -> Duration
```

How often to run registry reconciliation.

<details>
<summary>Source</summary>

```rust
    pub fn registry_reconcile_interval(&self) -> Duration {
        self.registry_reconcile_interval
    }
```

</details>



##### `registry_enable_startup_reconciliation` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_enable_startup_reconciliation (& self) -> bool
```

Whether startup reconciliation is enabled.

<details>
<summary>Source</summary>

```rust
    pub fn registry_enable_startup_reconciliation(&self) -> bool {
        self.registry_enable_startup_reconciliation
    }
```

</details>



##### `registry_storage_path` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_storage_path (& self) -> Option < & std :: path :: Path >
```

Path for registry storage (filesystem backend).

<details>
<summary>Source</summary>

```rust
    pub fn registry_storage_path(&self) -> Option<&std::path::Path> {
        self.registry_storage_path.as_deref()
    }
```

</details>



##### `registry_storage_backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_storage_backend (& self) -> & str
```

Registry storage backend type.

<details>
<summary>Source</summary>

```rust
    pub fn registry_storage_backend(&self) -> &str {
        &self.registry_storage_backend
    }
```

</details>



##### `enable_claiming` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_claiming (& self) -> bool
```

Whether task claiming is enabled for horizontal scaling.

<details>
<summary>Source</summary>

```rust
    pub fn enable_claiming(&self) -> bool {
        self.enable_claiming
    }
```

</details>



##### `heartbeat_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn heartbeat_interval (& self) -> Duration
```

Heartbeat interval for claimed tasks.

<details>
<summary>Source</summary>

```rust
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }
```

</details>



##### `stale_claim_sweep_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn stale_claim_sweep_interval (& self) -> Duration
```

Interval for stale claim sweep (only when claiming is enabled).

<details>
<summary>Source</summary>

```rust
    pub fn stale_claim_sweep_interval(&self) -> Duration {
        self.stale_claim_sweep_interval
    }
```

</details>



##### `stale_claim_threshold` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn stale_claim_threshold (& self) -> Duration
```

How old a heartbeat must be to consider a claim stale.

<details>
<summary>Source</summary>

```rust
    pub fn stale_claim_threshold(&self) -> Duration {
        self.stale_claim_threshold
    }
```

</details>



##### `runner_id` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runner_id (& self) -> Option < & str >
```

Optional runner identifier for logging.

<details>
<summary>Source</summary>

```rust
    pub fn runner_id(&self) -> Option<&str> {
        self.runner_id.as_deref()
    }
```

</details>



##### `runner_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runner_name (& self) -> Option < & str >
```

Optional runner name for logging.

<details>
<summary>Source</summary>

```rust
    pub fn runner_name(&self) -> Option<&str> {
        self.runner_name.as_deref()
    }
```

</details>



##### `routing_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn routing_config (& self) -> Option < & RoutingConfig >
```

Routing configuration for task dispatch.

<details>
<summary>Source</summary>

```rust
    pub fn routing_config(&self) -> Option<&RoutingConfig> {
        self.routing_config.as_ref()
    }
```

</details>





### `cloacina::runner::default_runner::config::DefaultRunnerConfigBuilder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Builder for [`DefaultRunnerConfig`].

Use this builder to create a customized configuration:
```rust,ignore
let config = DefaultRunnerConfig::builder()
.max_concurrent_tasks(8)
.enable_cron_scheduling(false)
.build();
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `config` | `DefaultRunnerConfig` |  |

#### Methods

##### `max_concurrent_tasks` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn max_concurrent_tasks (mut self , value : usize) -> Self
```

Sets the maximum number of concurrent task executions.

<details>
<summary>Source</summary>

```rust
    pub fn max_concurrent_tasks(mut self, value: usize) -> Self {
        self.config.max_concurrent_tasks = value;
        self
    }
```

</details>



##### `scheduler_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn scheduler_poll_interval (mut self , value : Duration) -> Self
```

Sets the scheduler poll interval.

<details>
<summary>Source</summary>

```rust
    pub fn scheduler_poll_interval(mut self, value: Duration) -> Self {
        self.config.scheduler_poll_interval = value;
        self
    }
```

</details>



##### `task_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn task_timeout (mut self , value : Duration) -> Self
```

Sets the task timeout.

<details>
<summary>Source</summary>

```rust
    pub fn task_timeout(mut self, value: Duration) -> Self {
        self.config.task_timeout = value;
        self
    }
```

</details>



##### `pipeline_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn pipeline_timeout (mut self , value : Option < Duration >) -> Self
```

Sets the pipeline timeout.

<details>
<summary>Source</summary>

```rust
    pub fn pipeline_timeout(mut self, value: Option<Duration>) -> Self {
        self.config.pipeline_timeout = value;
        self
    }
```

</details>



##### `db_pool_size` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn db_pool_size (mut self , value : u32) -> Self
```

Sets the database pool size.

<details>
<summary>Source</summary>

```rust
    pub fn db_pool_size(mut self, value: u32) -> Self {
        self.config.db_pool_size = value;
        self
    }
```

</details>



##### `enable_recovery` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_recovery (mut self , value : bool) -> Self
```

Enables or disables automatic recovery.

<details>
<summary>Source</summary>

```rust
    pub fn enable_recovery(mut self, value: bool) -> Self {
        self.config.enable_recovery = value;
        self
    }
```

</details>



##### `enable_cron_scheduling` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_cron_scheduling (mut self , value : bool) -> Self
```

Enables or disables cron scheduling.

<details>
<summary>Source</summary>

```rust
    pub fn enable_cron_scheduling(mut self, value: bool) -> Self {
        self.config.enable_cron_scheduling = value;
        self
    }
```

</details>



##### `cron_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_poll_interval (mut self , value : Duration) -> Self
```

Sets the cron poll interval.

<details>
<summary>Source</summary>

```rust
    pub fn cron_poll_interval(mut self, value: Duration) -> Self {
        self.config.cron_poll_interval = value;
        self
    }
```

</details>



##### `cron_max_catchup_executions` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_max_catchup_executions (mut self , value : usize) -> Self
```

Sets the maximum catchup executions for cron.

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_catchup_executions(mut self, value: usize) -> Self {
        self.config.cron_max_catchup_executions = value;
        self
    }
```

</details>



##### `cron_enable_recovery` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_enable_recovery (mut self , value : bool) -> Self
```

Enables or disables cron recovery.

<details>
<summary>Source</summary>

```rust
    pub fn cron_enable_recovery(mut self, value: bool) -> Self {
        self.config.cron_enable_recovery = value;
        self
    }
```

</details>



##### `cron_recovery_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_recovery_interval (mut self , value : Duration) -> Self
```

Sets the cron recovery interval.

<details>
<summary>Source</summary>

```rust
    pub fn cron_recovery_interval(mut self, value: Duration) -> Self {
        self.config.cron_recovery_interval = value;
        self
    }
```

</details>



##### `cron_lost_threshold_minutes` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_lost_threshold_minutes (mut self , value : i32) -> Self
```

Sets the cron lost threshold in minutes.

<details>
<summary>Source</summary>

```rust
    pub fn cron_lost_threshold_minutes(mut self, value: i32) -> Self {
        self.config.cron_lost_threshold_minutes = value;
        self
    }
```

</details>



##### `cron_max_recovery_age` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_max_recovery_age (mut self , value : Duration) -> Self
```

Sets the maximum cron recovery age.

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_age(mut self, value: Duration) -> Self {
        self.config.cron_max_recovery_age = value;
        self
    }
```

</details>



##### `cron_max_recovery_attempts` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn cron_max_recovery_attempts (mut self , value : usize) -> Self
```

Sets the maximum cron recovery attempts.

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_attempts(mut self, value: usize) -> Self {
        self.config.cron_max_recovery_attempts = value;
        self
    }
```

</details>



##### `enable_trigger_scheduling` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_trigger_scheduling (mut self , value : bool) -> Self
```

Enables or disables trigger scheduling.

<details>
<summary>Source</summary>

```rust
    pub fn enable_trigger_scheduling(mut self, value: bool) -> Self {
        self.config.enable_trigger_scheduling = value;
        self
    }
```

</details>



##### `trigger_base_poll_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn trigger_base_poll_interval (mut self , value : Duration) -> Self
```

Sets the trigger base poll interval.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_base_poll_interval(mut self, value: Duration) -> Self {
        self.config.trigger_base_poll_interval = value;
        self
    }
```

</details>



##### `trigger_poll_timeout` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn trigger_poll_timeout (mut self , value : Duration) -> Self
```

Sets the trigger poll timeout.

<details>
<summary>Source</summary>

```rust
    pub fn trigger_poll_timeout(mut self, value: Duration) -> Self {
        self.config.trigger_poll_timeout = value;
        self
    }
```

</details>



##### `enable_registry_reconciler` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_registry_reconciler (mut self , value : bool) -> Self
```

Enables or disables the registry reconciler.

<details>
<summary>Source</summary>

```rust
    pub fn enable_registry_reconciler(mut self, value: bool) -> Self {
        self.config.enable_registry_reconciler = value;
        self
    }
```

</details>



##### `registry_reconcile_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_reconcile_interval (mut self , value : Duration) -> Self
```

Sets the registry reconcile interval.

<details>
<summary>Source</summary>

```rust
    pub fn registry_reconcile_interval(mut self, value: Duration) -> Self {
        self.config.registry_reconcile_interval = value;
        self
    }
```

</details>



##### `registry_enable_startup_reconciliation` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_enable_startup_reconciliation (mut self , value : bool) -> Self
```

Enables or disables startup reconciliation.

<details>
<summary>Source</summary>

```rust
    pub fn registry_enable_startup_reconciliation(mut self, value: bool) -> Self {
        self.config.registry_enable_startup_reconciliation = value;
        self
    }
```

</details>



##### `registry_storage_path` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_storage_path (mut self , value : Option < std :: path :: PathBuf >) -> Self
```

Sets the registry storage path.

<details>
<summary>Source</summary>

```rust
    pub fn registry_storage_path(mut self, value: Option<std::path::PathBuf>) -> Self {
        self.config.registry_storage_path = value;
        self
    }
```

</details>



##### `registry_storage_backend` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn registry_storage_backend (mut self , value : impl Into < String >) -> Self
```

Sets the registry storage backend.

<details>
<summary>Source</summary>

```rust
    pub fn registry_storage_backend(mut self, value: impl Into<String>) -> Self {
        self.config.registry_storage_backend = value.into();
        self
    }
```

</details>



##### `runner_id` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runner_id (mut self , value : Option < String >) -> Self
```

Sets the runner identifier.

<details>
<summary>Source</summary>

```rust
    pub fn runner_id(mut self, value: Option<String>) -> Self {
        self.config.runner_id = value;
        self
    }
```

</details>



##### `runner_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runner_name (mut self , value : Option < String >) -> Self
```

Sets the runner name.

<details>
<summary>Source</summary>

```rust
    pub fn runner_name(mut self, value: Option<String>) -> Self {
        self.config.runner_name = value;
        self
    }
```

</details>



##### `routing_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn routing_config (mut self , value : Option < RoutingConfig >) -> Self
```

Sets the routing configuration.

<details>
<summary>Source</summary>

```rust
    pub fn routing_config(mut self, value: Option<RoutingConfig>) -> Self {
        self.config.routing_config = value;
        self
    }
```

</details>



##### `enable_claiming` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn enable_claiming (mut self , value : bool) -> Self
```

Enables or disables task claiming for horizontal scaling.

<details>
<summary>Source</summary>

```rust
    pub fn enable_claiming(mut self, value: bool) -> Self {
        self.config.enable_claiming = value;
        self
    }
```

</details>



##### `heartbeat_interval` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn heartbeat_interval (mut self , value : Duration) -> Self
```

Sets the heartbeat interval for claimed tasks.

<details>
<summary>Source</summary>

```rust
    pub fn heartbeat_interval(mut self, value: Duration) -> Self {
        self.config.heartbeat_interval = value;
        self
    }
```

</details>



##### `build` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build (self) -> DefaultRunnerConfig
```

Builds the configuration.

<details>
<summary>Source</summary>

```rust
    pub fn build(self) -> DefaultRunnerConfig {
        assert!(
            self.config.max_concurrent_tasks > 0,
            "max_concurrent_tasks must be > 0"
        );
        assert!(self.config.db_pool_size > 0, "db_pool_size must be > 0");
        assert!(
            self.config.stale_claim_threshold > self.config.heartbeat_interval,
            "stale_claim_threshold ({:?}) must be greater than heartbeat_interval ({:?})",
            self.config.stale_claim_threshold,
            self.config.heartbeat_interval
        );
        self.config
    }
```

</details>





### `cloacina::runner::default_runner::config::DefaultRunnerBuilder`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Builder for creating a DefaultRunner with PostgreSQL schema-based multi-tenancy

This builder supports PostgreSQL schema-based multi-tenancy for complete tenant isolation.
Each schema provides complete data isolation with zero collision risk.

**Examples:**

```rust,ignore
// Single-tenant PostgreSQL (uses public schema)
let runner = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .build()
    .await?;

// Multi-tenant PostgreSQL with schema isolation
let tenant_a = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("tenant_a")
    .build()
    .await?;

let tenant_b = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("tenant_b")
    .build()
    .await?;
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `Option < String >` |  |
| `schema` | `Option < String >` |  |
| `config` | `DefaultRunnerConfig` |  |
| `runtime` | `Option < Runtime >` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new () -> Self
```

Creates a new builder with default configuration

<details>
<summary>Source</summary>

```rust
    pub fn new() -> Self {
        Self {
            database_url: None,
            schema: None,
            config: DefaultRunnerConfig::default(),
            runtime: None,
        }
    }
```

</details>



##### `database_url` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn database_url (mut self , url : & str) -> Self
```

Sets the database URL

<details>
<summary>Source</summary>

```rust
    pub fn database_url(mut self, url: &str) -> Self {
        self.database_url = Some(url.to_string());
        self
    }
```

</details>



##### `schema` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn schema (mut self , schema : & str) -> Self
```

Sets the PostgreSQL schema for multi-tenant isolation

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schema` | `-` | The schema name (must be alphanumeric with underscores only) |


<details>
<summary>Source</summary>

```rust
    pub fn schema(mut self, schema: &str) -> Self {
        self.schema = Some(schema.to_string());
        self
    }
```

</details>



##### `with_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn with_config (mut self , config : DefaultRunnerConfig) -> Self
```

Sets the full configuration

<details>
<summary>Source</summary>

```rust
    pub fn with_config(mut self, config: DefaultRunnerConfig) -> Self {
        self.config = config;
        self
    }
```

</details>



##### `runtime` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn runtime (mut self , runtime : Runtime) -> Self
```

Sets a scoped [`Runtime`] for this runner.

When set, the runner (and all components it creates) will use this
runtime's registries instead of the process-global registries.
If not set, [`Runtime::from_global()`] is used as the default.

<details>
<summary>Source</summary>

```rust
    pub fn runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = Some(runtime);
        self
    }
```

</details>



##### `validate_schema_name` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">pub(super)</span>


```rust
fn validate_schema_name (schema : & str) -> Result < () , WorkflowExecutionError >
```

Validates the schema name contains only alphanumeric characters and underscores

<details>
<summary>Source</summary>

```rust
    pub(super) fn validate_schema_name(schema: &str) -> Result<(), WorkflowExecutionError> {
        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(WorkflowExecutionError::Configuration {
                message: "Schema name must contain only alphanumeric characters and underscores"
                    .to_string(),
            });
        }
        Ok(())
    }
```

</details>



##### `build` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn build (self) -> Result < DefaultRunner , WorkflowExecutionError >
```

Builds the DefaultRunner

<details>
<summary>Source</summary>

```rust
    pub async fn build(self) -> Result<DefaultRunner, WorkflowExecutionError> {
        let database_url =
            self.database_url
                .ok_or_else(|| WorkflowExecutionError::Configuration {
                    message: "Database URL is required".to_string(),
                })?;

        if let Some(ref schema) = self.schema {
            Self::validate_schema_name(schema)?;

            // Validate schema is only used with PostgreSQL
            if !database_url.starts_with("postgresql://")
                && !database_url.starts_with("postgres://")
            {
                return Err(WorkflowExecutionError::Configuration {
                    message: "Schema isolation is only supported with PostgreSQL. \
                             For SQLite multi-tenancy, use separate database files instead."
                        .to_string(),
                });
            }
        }

        // Create the database with schema support
        let database = Database::new_with_schema(
            &database_url,
            "cloacina",
            self.config.db_pool_size(),
            self.schema.as_deref(),
        );

        // Set up schema if specified (PostgreSQL only)
        #[cfg(feature = "postgres")]
        {
            if let Some(ref schema) = self.schema {
                database.setup_schema(schema).await.map_err(|e| {
                    WorkflowExecutionError::Configuration {
                        message: format!("Failed to set up schema '{}': {}", schema, e),
                    }
                })?;
            } else {
                // Run migrations in public schema
                database
                    .run_migrations()
                    .await
                    .map_err(|e| WorkflowExecutionError::DatabaseConnection { message: e })?;
            }
        }

        #[cfg(not(feature = "postgres"))]
        {
            // SQLite: just run migrations (schemas not supported)
            database
                .run_migrations()
                .await
                .map_err(|e| WorkflowExecutionError::DatabaseConnection { message: e })?;
        }

        // Resolve runtime: use provided or snapshot from globals
        let runtime = Arc::new(self.runtime.unwrap_or_else(Runtime::from_global));

        // Create scheduler with the scoped runtime
        let scheduler = TaskScheduler::with_poll_interval(
            database.clone(),
            self.config.scheduler_poll_interval(),
        )
        .await
        .map_err(|e| WorkflowExecutionError::Executor(e.into()))?
        .with_runtime(runtime.clone());

        // Create task executor
        let executor_config = ExecutorConfig {
            max_concurrent_tasks: self.config.max_concurrent_tasks(),
            task_timeout: self.config.task_timeout(),
            enable_claiming: self.config.enable_claiming(),
            heartbeat_interval: self.config.heartbeat_interval(),
        };

        // Create executor with the scoped runtime — skip with_global_registry() since
        // the runtime provides task lookups and the old TaskRegistry is unused.
        let executor = ThreadTaskExecutor::with_runtime_and_registry(
            database.clone(),
            Arc::new(crate::TaskRegistry::new()),
            runtime.clone(),
            executor_config,
        );

        // Configure dispatcher for push-based task execution
        let dal = crate::dal::DAL::new(database.clone());
        let routing_config = self
            .config
            .routing_config()
            .cloned()
            .unwrap_or_else(RoutingConfig::default);
        let dispatcher = DefaultDispatcher::new(dal, routing_config);

        // Register the executor with the dispatcher
        dispatcher.register_executor("default", Arc::new(executor) as Arc<dyn TaskExecutor>);

        let scheduler = scheduler.with_dispatcher(Arc::new(dispatcher));

        let default_runner = DefaultRunner {
            runtime,
            database,
            config: self.config.clone(),
            scheduler: Arc::new(scheduler),
            runtime_handles: Arc::new(RwLock::new(RuntimeHandles {
                scheduler_handle: None,
                executor_handle: None,
                cron_recovery_handle: None,
                registry_reconciler_handle: None,
                unified_scheduler_handle: None,
                shutdown_sender: None,
            })),
            cron_recovery: Arc::new(RwLock::new(None)), // Initially empty
            workflow_registry: Arc::new(RwLock::new(None)), // Initially empty
            registry_reconciler: Arc::new(RwLock::new(None)), // Initially empty
            unified_scheduler: Arc::new(RwLock::new(None)), // Initially empty
            graph_scheduler: Arc::new(RwLock::new(None)), // Initially empty
        };

        // Start the background services immediately
        default_runner.start_background_services().await?;

        Ok(default_runner)
    }
```

</details>



##### `routing_config` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn routing_config (mut self , config : RoutingConfig) -> Self
```

Sets custom routing configuration for task dispatch.

Use this to route different tasks to different executor backends.

**Examples:**

```rust,ignore
let runner = DefaultRunner::builder()
    .database_url("sqlite://test.db")
    .routing_config(
        RoutingConfig::new("default")
            .with_rule(RoutingRule::new("ml::*", "gpu"))
    )
    .build()
    .await?;
```

<details>
<summary>Source</summary>

```rust
    pub fn routing_config(mut self, config: RoutingConfig) -> Self {
        self.config.routing_config = Some(config);
        self
    }
```

</details>
