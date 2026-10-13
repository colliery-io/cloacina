---
title: "cloacina::runner::default_runner::config"
description: "Configuration types for the DefaultRunner."
editUrl: false
sidebar:
  order: 179
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Configuration types for the DefaultRunner.

This module contains the configuration structs and builders for
configuring the DefaultRunner's behavior.

## Structs

### <span id="struct.DefaultRunnerConfig"></span><code style="font-size: 0.85em">struct DefaultRunnerConfig</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Configuration for the default runner

This struct defines the configuration parameters that control the behavior
of the DefaultRunner. It includes settings for concurrency, timeouts,
polling intervals, and database connection management.

**Construction**

Use [`DefaultRunnerConfig::builder()`](../../../../../rust/cloacina/runner/default_runner/config/#DefaultRunnerConfig.builder) to create a configuration:

```rust
let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(8)
    .task_timeout(Duration::from_secs(600))
    .build();
```

Or use the default configuration:

```rust
let config = DefaultRunnerConfig::default();
```

#### Methods

##### <span id="DefaultRunnerConfig.builder"></span><code>builder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn builder() -> DefaultRunnerConfigBuilder
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

##### <span id="DefaultRunnerConfig.max_concurrent_tasks"></span><code>max_​concurrent_​tasks</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn max_concurrent_tasks(&self) -> usize
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

##### <span id="DefaultRunnerConfig.scheduler_poll_interval"></span><code>scheduler_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn scheduler_poll_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.task_timeout"></span><code>task_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn task_timeout(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.workflow_timeout"></span><code>workflow_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn workflow_timeout(&self) -> Option<Duration>
```

Optional maximum time for an entire workflow execution.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_timeout(&self) -> Option<Duration> {
        self.workflow_timeout
    }
```

</details>

##### <span id="DefaultRunnerConfig.db_pool_size"></span><code>db_​pool_​size</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn db_pool_size(&self) -> u32
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

##### <span id="DefaultRunnerConfig.enable_recovery"></span><code>enable_​recovery</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_recovery(&self) -> bool
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

##### <span id="DefaultRunnerConfig.enable_cron_scheduling"></span><code>enable_​cron_​scheduling</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_cron_scheduling(&self) -> bool
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

##### <span id="DefaultRunnerConfig.cron_poll_interval"></span><code>cron_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_poll_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.cron_max_catchup_executions"></span><code>cron_​max_​catchup_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_max_catchup_executions(&self) -> usize
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

##### <span id="DefaultRunnerConfig.cron_enable_recovery"></span><code>cron_​enable_​recovery</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_enable_recovery(&self) -> bool
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

##### <span id="DefaultRunnerConfig.cron_recovery_interval"></span><code>cron_​recovery_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_recovery_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.cron_lost_threshold_minutes"></span><code>cron_​lost_​threshold_​minutes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_lost_threshold_minutes(&self) -> i32
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

##### <span id="DefaultRunnerConfig.cron_max_recovery_age"></span><code>cron_​max_​recovery_​age</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_max_recovery_age(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.cron_max_recovery_attempts"></span><code>cron_​max_​recovery_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_max_recovery_attempts(&self) -> usize
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

##### <span id="DefaultRunnerConfig.enable_trigger_scheduling"></span><code>enable_​trigger_​scheduling</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_trigger_scheduling(&self) -> bool
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

##### <span id="DefaultRunnerConfig.trigger_base_poll_interval"></span><code>trigger_​base_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn trigger_base_poll_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.trigger_poll_timeout"></span><code>trigger_​poll_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn trigger_poll_timeout(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.enable_registry_reconciler"></span><code>enable_​registry_​reconciler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_registry_reconciler(&self) -> bool
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

##### <span id="DefaultRunnerConfig.registry_reconcile_interval"></span><code>registry_​reconcile_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_reconcile_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.registry_enable_startup_reconciliation"></span><code>registry_​enable_​startup_​reconciliation</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_enable_startup_reconciliation(&self) -> bool
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

##### <span id="DefaultRunnerConfig.registry_storage_path"></span><code>registry_​storage_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_storage_path(&self) -> Option<&std::path::Path>
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

##### <span id="DefaultRunnerConfig.require_signatures"></span><code>require_​signatures</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn require_signatures(&self) -> bool
```

CLOACI-T-0571: when true, the reconciler refuses to load packages with no matching `package_signatures` row.

<details>
<summary>Source</summary>

```rust
    pub fn require_signatures(&self) -> bool {
        self.require_signatures
    }
```

</details>

##### <span id="DefaultRunnerConfig.tenant_id"></span><code>tenant_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn tenant_id(&self) -> &str
```

The tenant this runner serves (CLOACI-T-0781). Namespaces its tasks.

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_tenant_id"></span><code>set_​tenant_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn set_tenant_id(&mut self, tenant_id: impl Into<String>)
```

Override the tenant — used by the per-tenant runner cache when cloning the base config for a tenant-scoped runner so its tasks namespace correctly.

<details>
<summary>Source</summary>

```rust
    pub fn set_tenant_id(&mut self, tenant_id: impl Into<String>) {
        self.tenant_id = tenant_id.into();
    }
```

</details>

##### <span id="DefaultRunnerConfig.verification_org_id"></span><code>verification_​org_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn verification_org_id(&self) -> Option<crate::UniversalUuid>
```

Trusted org UUID forwarded to the reconciler for audit logging when the signature-existence check fails.

<details>
<summary>Source</summary>

```rust
    pub fn verification_org_id(&self) -> Option<crate::UniversalUuid> {
        self.verification_org_id
    }
```

</details>

##### <span id="DefaultRunnerConfig.registry_storage_backend"></span><code>registry_​storage_​backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_storage_backend(&self) -> &str
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

##### <span id="DefaultRunnerConfig.enable_claiming"></span><code>enable_​claiming</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_claiming(&self) -> bool
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

##### <span id="DefaultRunnerConfig.heartbeat_interval"></span><code>heartbeat_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn heartbeat_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.stale_claim_sweep_interval"></span><code>stale_​claim_​sweep_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn stale_claim_sweep_interval(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.stale_claim_threshold"></span><code>stale_​claim_​threshold</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn stale_claim_threshold(&self) -> Duration
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

##### <span id="DefaultRunnerConfig.runner_id"></span><code>runner_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runner_id(&self) -> Option<&str>
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

##### <span id="DefaultRunnerConfig.runner_name"></span><code>runner_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runner_name(&self) -> Option<&str>
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

##### <span id="DefaultRunnerConfig.default_executor"></span><code>default_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn default_executor(&self) -> &str
```

Executor key every task is dispatched to.

<details>
<summary>Source</summary>

```rust
    pub fn default_executor(&self) -> &str {
        &self.default_executor
    }
```

</details>

### <span id="struct.DefaultRunnerConfigBuilder"></span><code style="font-size: 0.85em">struct DefaultRunnerConfigBuilder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

**Derives:** `Debug`, `Clone`

Builder for [`DefaultRunnerConfig`](../../../../../rust/cloacina/runner/default_runner/config/#struct.DefaultRunnerConfig).

Use this builder to create a customized configuration:

```rust
let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(8)
    .enable_cron_scheduling(false)
    .build();
```

#### Methods

##### <span id="DefaultRunnerConfigBuilder.max_concurrent_tasks"></span><code>max_​concurrent_​tasks</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn max_concurrent_tasks(mut self, value: usize) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.scheduler_poll_interval"></span><code>scheduler_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn scheduler_poll_interval(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.task_timeout"></span><code>task_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn task_timeout(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.workflow_timeout"></span><code>workflow_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn workflow_timeout(mut self, value: Option<Duration>) -> Self
```

Sets the workflow timeout.

<details>
<summary>Source</summary>

```rust
    pub fn workflow_timeout(mut self, value: Option<Duration>) -> Self {
        self.config.workflow_timeout = value;
        self
    }
```

</details>

##### <span id="DefaultRunnerConfigBuilder.db_pool_size"></span><code>db_​pool_​size</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn db_pool_size(mut self, value: u32) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.enable_recovery"></span><code>enable_​recovery</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_recovery(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.enable_cron_scheduling"></span><code>enable_​cron_​scheduling</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_cron_scheduling(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_poll_interval"></span><code>cron_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_poll_interval(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_max_catchup_executions"></span><code>cron_​max_​catchup_​executions</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_max_catchup_executions(mut self, value: usize) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_enable_recovery"></span><code>cron_​enable_​recovery</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_enable_recovery(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_recovery_interval"></span><code>cron_​recovery_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_recovery_interval(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_lost_threshold_minutes"></span><code>cron_​lost_​threshold_​minutes</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_lost_threshold_minutes(mut self, value: i32) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_max_recovery_age"></span><code>cron_​max_​recovery_​age</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_max_recovery_age(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.cron_max_recovery_attempts"></span><code>cron_​max_​recovery_​attempts</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn cron_max_recovery_attempts(mut self, value: usize) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.enable_trigger_scheduling"></span><code>enable_​trigger_​scheduling</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_trigger_scheduling(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.trigger_base_poll_interval"></span><code>trigger_​base_​poll_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn trigger_base_poll_interval(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.trigger_poll_timeout"></span><code>trigger_​poll_​timeout</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn trigger_poll_timeout(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.enable_registry_reconciler"></span><code>enable_​registry_​reconciler</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_registry_reconciler(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.registry_reconcile_interval"></span><code>registry_​reconcile_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_reconcile_interval(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.registry_enable_startup_reconciliation"></span><code>registry_​enable_​startup_​reconciliation</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_enable_startup_reconciliation(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.registry_storage_path"></span><code>registry_​storage_​path</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_storage_path(
    mut self,
    value: Option<std::path::PathBuf>,
) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.registry_storage_backend"></span><code>registry_​storage_​backend</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn registry_storage_backend(mut self, value: impl Into<String>) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.require_signatures"></span><code>require_​signatures</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn require_signatures(mut self, value: bool) -> Self
```

CLOACI-T-0571: enable the reconciler's defense-in-depth signature-existence check. Mirrors `cloacina-server`'s `--require-signatures` CLI flag.

<details>
<summary>Source</summary>

```rust
    pub fn require_signatures(mut self, value: bool) -> Self {
        self.config.require_signatures = value;
        self
    }
```

</details>

##### <span id="DefaultRunnerConfigBuilder.verification_org_id"></span><code>verification_​org_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn verification_org_id(
    mut self,
    value: Option<crate::UniversalUuid>,
) -> Self
```

CLOACI-T-0571: trusted org UUID forwarded to the reconciler so signature-existence-check failures are audit-logged with the correct org_id. Mirrors `cloacina-server`'s `--verification-org-id` CLI flag.

<details>
<summary>Source</summary>

```rust
    pub fn verification_org_id(mut self, value: Option<crate::UniversalUuid>) -> Self {
        self.config.verification_org_id = value;
        self
    }
```

</details>

##### <span id="DefaultRunnerConfigBuilder.runner_id"></span><code>runner_​id</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runner_id(mut self, value: Option<String>) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.runner_name"></span><code>runner_​name</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runner_name(mut self, value: Option<String>) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.default_executor"></span><code>default_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn default_executor(mut self, value: impl Into<String>) -> Self
```

Sets the executor key every task is dispatched to (CLOACI-T-0640). Defaults to `"default"` (the thread executor).

<details>
<summary>Source</summary>

```rust
    pub fn default_executor(mut self, value: impl Into<String>) -> Self {
        self.config.default_executor = value.into();
        self
    }
```

</details>

##### <span id="DefaultRunnerConfigBuilder.enable_claiming"></span><code>enable_​claiming</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn enable_claiming(mut self, value: bool) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.heartbeat_interval"></span><code>heartbeat_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn heartbeat_interval(mut self, value: Duration) -> Self
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

##### <span id="DefaultRunnerConfigBuilder.stale_claim_threshold"></span><code>stale_​claim_​threshold</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn stale_claim_threshold(mut self, value: Duration) -> Self
```

Sets how old a claimed task's heartbeat must be before its claim is considered stale and reclaimed by the sweeper. Must be greater than `heartbeat_interval` (enforced in `build()`). Lower values shorten the crash-recovery window (default 60s). (CLOACI-T-0667)

<details>
<summary>Source</summary>

```rust
    pub fn stale_claim_threshold(mut self, value: Duration) -> Self {
        self.config.stale_claim_threshold = value;
        self
    }
```

</details>

##### <span id="DefaultRunnerConfigBuilder.stale_claim_sweep_interval"></span><code>stale_​claim_​sweep_​interval</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn stale_claim_sweep_interval(mut self, value: Duration) -> Self
```

Sets how often the stale-claim sweeper runs (default 30s). Only has effect when claiming + recovery are enabled. (CLOACI-T-0667)

<details>
<summary>Source</summary>

```rust
    pub fn stale_claim_sweep_interval(mut self, value: Duration) -> Self {
        self.config.stale_claim_sweep_interval = value;
        self
    }
```

</details>

##### <span id="DefaultRunnerConfigBuilder.build"></span><code>build</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn build(self) -> Result<DefaultRunnerConfig, ConfigError>
```

Builds and validates the configuration.

Returns an error if any configuration value is out of bounds.

<details>
<summary>Source</summary>

```rust
    pub fn build(self) -> Result<DefaultRunnerConfig, ConfigError> {
        if self.config.max_concurrent_tasks == 0 {
            return Err(ConfigError::Invalid(
                "max_concurrent_tasks must be > 0".into(),
            ));
        }
        if self.config.db_pool_size == 0 {
            return Err(ConfigError::Invalid("db_pool_size must be > 0".into()));
        }
        if self.config.scheduler_poll_interval < Duration::from_millis(10) {
            return Err(ConfigError::Invalid(
                "scheduler_poll_interval must be >= 10ms".into(),
            ));
        }
        if self.config.stale_claim_threshold <= self.config.heartbeat_interval {
            return Err(ConfigError::Invalid(format!(
                "stale_claim_threshold ({:?}) must be greater than heartbeat_interval ({:?})",
                self.config.stale_claim_threshold, self.config.heartbeat_interval
            )));
        }
        if self.config.cron_max_catchup_executions > 1000 {
            return Err(ConfigError::Invalid(format!(
                "cron_max_catchup_executions ({}) must be <= 1000",
                self.config.cron_max_catchup_executions
            )));
        }
        Ok(self.config)
    }
```

</details>

### <span id="struct.DefaultRunnerBuilder"></span><code style="font-size: 0.85em">struct DefaultRunnerBuilder</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Builder for creating a DefaultRunner with PostgreSQL schema-based multi-tenancy

This builder supports PostgreSQL schema-based multi-tenancy for complete tenant isolation.
Each schema provides complete data isolation with zero collision risk.

**Examples:**

```rust
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

#### Methods

##### <span id="DefaultRunnerBuilder.new"></span><code>new</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn new() -> Self
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
            runtime_arc: None,
            secret_resolver: None,
        }
    }
```

</details>

##### <span id="DefaultRunnerBuilder.secret_resolver"></span><code>secret_​resolver</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn secret_resolver(
    mut self,
    resolver: Arc<dyn cloacina_workflow::secret::SecretResolver>,
) -> Self
```

Wire a secret resolution side channel (CLOACI-T-0858 / I-0133 D-1).

On the embedded / in-process path the host configures this so packaged
tasks can resolve tenant secrets via `context.secret("name")`. Construct
one from [`crate::security::SecretStoreResolver`](../../../../../rust/cloacina/security/secret_resolver/#struct.SecretStoreResolver) — e.g.
`SecretStoreResolver::from_env(store, org_id)` to source the server KEK
from `CLOACINA_SECRET_KEK`. When unset, `context.secret(...)` returns a
clear "secrets backend not configured" error.

<details>
<summary>Source</summary>

```rust
    pub fn secret_resolver(
        mut self,
        resolver: Arc<dyn cloacina_workflow::secret::SecretResolver>,
    ) -> Self {
        self.secret_resolver = Some(resolver);
        self
    }
```

</details>

##### <span id="DefaultRunnerBuilder.database_url"></span><code>database_​url</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn database_url(mut self, url: &str) -> Self
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

##### <span id="DefaultRunnerBuilder.schema"></span><code>schema</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn schema(mut self, schema: &str) -> Self
```

Sets the PostgreSQL schema for multi-tenant isolation

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schema` | `&str` | The schema name (must be alphanumeric with underscores only) |

<details>
<summary>Source</summary>

```rust
    pub fn schema(mut self, schema: &str) -> Self {
        self.schema = Some(schema.to_string());
        self
    }
```

</details>

##### <span id="DefaultRunnerBuilder.with_config"></span><code>with_​config</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn with_config(mut self, config: DefaultRunnerConfig) -> Self
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

##### <span id="DefaultRunnerBuilder.runtime"></span><code>runtime</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runtime(mut self, runtime: Runtime) -> Self
```

Sets a scoped [`Runtime`](../../../../../rust/cloacina/runtime/#struct.Runtime) for this runner.

When set, the runner (and all components it creates) will use this
runtime's registries instead of the process-global registries.
If not set, a fresh inventory-seeded [`Runtime`](../../../../../rust/cloacina/runtime/#struct.Runtime) (i.e.
`Runtime::default`, equivalent to [`Runtime::new`](../../../../../rust/cloacina/runtime/#Runtime.new)) is used.

<details>
<summary>Source</summary>

```rust
    pub fn runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = Some(runtime);
        self
    }
```

</details>

##### <span id="DefaultRunnerBuilder.runtime_arc"></span><code>runtime_​arc</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn runtime_arc(mut self, runtime: Arc<Runtime>) -> Self
```

Use an existing shared [`Arc<Runtime>`] for this runner.

Unlike `runtime`, this preserves the caller's `Arc`
identity so other components (e.g. Python decorators installed via
`ScopedRuntime`) that already hold a clone of the same `Arc<Runtime>`
share state with the runner. If both are set, `runtime_arc` wins.

<details>
<summary>Source</summary>

```rust
    pub fn runtime_arc(mut self, runtime: Arc<Runtime>) -> Self {
        self.runtime_arc = Some(runtime);
        self
    }
```

</details>

##### <span id="DefaultRunnerBuilder.build"></span><code>build</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-blue-low); color: var(--sl-color-blue-high);">async</span>

```rust
async fn build(self) -> Result<DefaultRunner, WorkflowExecutionError>
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

        // Resolve runtime: prefer a caller-supplied Arc (so shared state with
        // e.g. Python ScopedRuntime is preserved), else wrap a by-value
        // Runtime, else create a fresh inventory-seeded one.
        // CLOACI-T-0924: bind the handle to this runner's tenant. The caller's
        // registries are still shared (only the scope differs), so a handle the
        // caller kept — a Python `ScopedRuntime`, say — sees the same entries.
        let runtime = Arc::new(
            self.runtime_arc
                .unwrap_or_else(|| Arc::new(self.runtime.unwrap_or_default()))
                .scoped_to_tenant(self.config.tenant_id()),
        );

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
        )
        // CLOACI-T-0858: thread the secret resolution side channel to the
        // executor so task contexts can resolve secrets in-process.
        .with_secret_resolver(self.secret_resolver.clone());

        // Configure dispatcher for push-based task execution. Every task is sent
        // to the one configured executor key (CLOACI-T-0640).
        let dal = crate::dal::DAL::new(database.clone());
        let dispatcher = DefaultDispatcher::new(dal, self.config.default_executor());

        // Register the executor with the dispatcher
        dispatcher.register_executor("default", Arc::new(executor) as Arc<dyn TaskExecutor>);

        let scheduler = scheduler.with_dispatcher(Arc::new(dispatcher));

        let default_runner = DefaultRunner {
            runtime,
            database,
            config: self.config.clone(),
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

##### <span id="DefaultRunnerBuilder.default_executor"></span><code>default_​executor</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn default_executor(mut self, executor: impl Into<String>) -> Self
```

Sets the executor key every task is dispatched to (CLOACI-T-0640).

Defaults to `"default"` (the thread executor). Set to another registered
executor key (e.g. `"fleet"`) to send all work there instead.

**Examples:**

```rust
let runner = DefaultRunner::builder()
    .database_url("sqlite://test.db")
    .default_executor("fleet")
    .build()
    .await?;
```

<details>
<summary>Source</summary>

```rust
    pub fn default_executor(mut self, executor: impl Into<String>) -> Self {
        self.config.default_executor = executor.into();
        self
    }
```

</details>

## Enums

### <span id="enum.ConfigError"></span><code style="font-size: 0.85em">enum ConfigError</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

Errors that can occur during configuration validation.

#### Variants

- **`Invalid`**
