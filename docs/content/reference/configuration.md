---
title: "Configuration Reference"
description: "Complete reference for DefaultRunnerConfig fields, builder methods, config.toml schema, and environment variables"
weight: 7
---

# Configuration Reference

This page documents all configuration options for the Cloacina runtime. Configuration is specified programmatically via `DefaultRunnerConfig` (Rust API), through `~/.cloacina/config.toml` (daemon/server), or via environment variables.

## DefaultRunnerConfig

The `DefaultRunnerConfig` struct controls all runtime behavior of the `DefaultRunner`. Create one with the builder pattern:

```rust
use cloacina::runner::DefaultRunnerConfig;
use std::time::Duration;

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(8)
    .task_timeout(Duration::from_secs(600))
    .enable_cron_scheduling(false)
    .build();
```

### Concurrency

| Field | Type | Default | Description |
|---|---|---|---|
| `max_concurrent_tasks` | `usize` | `4` | Maximum number of task executions running simultaneously. Controls the semaphore size for the task executor. |
| `scheduler_poll_interval` | `Duration` | `100ms` | How often the task scheduler checks for tasks whose dependencies are satisfied and are ready to execute. |
| `task_timeout` | `Duration` | `300s` (5 min) | Maximum time allowed for a single task to execute before it is considered timed out. |
| `pipeline_timeout` | `Option<Duration>` | `Some(3600s)` (1 hr) | Maximum time for an entire pipeline execution. `None` disables the pipeline-level timeout. |
| `db_pool_size` | `u32` | `10` | Number of database connections in the connection pool. |
| `enable_recovery` | `bool` | `true` | Whether automatic recovery of failed/stale task executions is enabled. |

### Cron Scheduling

| Field | Type | Default | Description |
|---|---|---|---|
| `enable_cron_scheduling` | `bool` | `true` | Master switch for cron scheduling. When disabled, no cron schedules are evaluated. |
| `cron_poll_interval` | `Duration` | `30s` | How often the cron scheduler checks for schedules that are due. |
| `cron_max_catchup_executions` | `usize` | `usize::MAX` | Maximum number of missed cron executions to catch up on after downtime. Set to a finite value to cap catchup behavior. |
| `cron_enable_recovery` | `bool` | `true` | Whether recovery of lost/failed cron executions is enabled. |
| `cron_recovery_interval` | `Duration` | `300s` (5 min) | How often the recovery system scans for lost cron executions. |
| `cron_lost_threshold_minutes` | `i32` | `10` | Minutes after which a started-but-not-completed cron execution is considered lost. |
| `cron_max_recovery_age` | `Duration` | `86400s` (24 hr) | Executions older than this are not recovered. Prevents unbounded catchup on long outages. |
| `cron_max_recovery_attempts` | `usize` | `3` | Maximum number of recovery attempts per cron execution before it is abandoned. |

### Trigger Scheduling

| Field | Type | Default | Description |
|---|---|---|---|
| `enable_trigger_scheduling` | `bool` | `true` | Master switch for trigger-based scheduling. |
| `trigger_base_poll_interval` | `Duration` | `1s` | Base interval for checking trigger readiness. Individual triggers can define their own interval. |
| `trigger_poll_timeout` | `Duration` | `30s` | Timeout for a single trigger poll operation. |

### Registry

| Field | Type | Default | Description |
|---|---|---|---|
| `enable_registry_reconciler` | `bool` | `true` | Whether the background registry reconciler runs to detect new/removed workflow packages. |
| `registry_reconcile_interval` | `Duration` | `60s` | How often the reconciler scans for changes. |
| `registry_enable_startup_reconciliation` | `bool` | `true` | Whether to run a full reconciliation pass on startup. |
| `registry_storage_path` | `Option<PathBuf>` | `None` | Custom path for filesystem-based registry storage. `None` uses the default location. |
| `registry_storage_backend` | `String` | `"filesystem"` | Storage backend type. Options: `"filesystem"`, `"database"`. The `serve` command uses `"database"`. |

### Task Claiming

Task claiming enables horizontal scaling by allowing multiple runner instances to coordinate work.

| Field | Type | Default | Description |
|---|---|---|---|
| `enable_claiming` | `bool` | `true` | Whether task claiming is enabled. When enabled, tasks are claimed via the database before execution. |
| `heartbeat_interval` | `Duration` | `10s` | How often a runner sends heartbeats for its claimed tasks. |
| `stale_claim_sweep_interval` | `Duration` | `30s` | How often to scan for claims whose heartbeats have expired. |
| `stale_claim_threshold` | `Duration` | `60s` | How old a heartbeat must be before the claim is considered stale and can be reclaimed. |

### Runner Identity

| Field | Type | Default | Description |
|---|---|---|---|
| `runner_id` | `Option<String>` | `None` | Optional unique identifier for this runner instance. Used in logs and claim ownership. |
| `runner_name` | `Option<String>` | `None` | Optional human-readable name for this runner instance. |
| `routing_config` | `Option<RoutingConfig>` | `None` | Task routing configuration for dispatching tasks to specific executor backends. |

## DefaultRunnerConfigBuilder

All builder methods consume and return `self` for chaining. Each method corresponds directly to a config field:

```rust
DefaultRunnerConfig::builder()
    // Concurrency
    .max_concurrent_tasks(8)
    .scheduler_poll_interval(Duration::from_millis(200))
    .task_timeout(Duration::from_secs(600))
    .pipeline_timeout(Some(Duration::from_secs(7200)))
    .db_pool_size(20)
    .enable_recovery(true)

    // Cron
    .enable_cron_scheduling(true)
    .cron_poll_interval(Duration::from_secs(60))
    .cron_max_catchup_executions(100)
    .cron_enable_recovery(true)
    .cron_recovery_interval(Duration::from_secs(300))
    .cron_lost_threshold_minutes(15)
    .cron_max_recovery_age(Duration::from_secs(86400))
    .cron_max_recovery_attempts(5)

    // Triggers
    .enable_trigger_scheduling(true)
    .trigger_base_poll_interval(Duration::from_secs(5))
    .trigger_poll_timeout(Duration::from_secs(60))

    // Registry
    .enable_registry_reconciler(true)
    .registry_reconcile_interval(Duration::from_secs(30))
    .registry_enable_startup_reconciliation(true)
    .registry_storage_path(Some(PathBuf::from("/custom/path")))
    .registry_storage_backend("database")

    // Claiming
    .enable_claiming(true)
    .heartbeat_interval(Duration::from_secs(10))
    // Note: stale_claim_sweep_interval and stale_claim_threshold are struct
    // fields on DefaultRunnerConfig but are NOT available as builder methods.
    // They use their default values (30s and 60s respectively) when using
    // the builder. To customize them, modify the struct fields directly
    // after calling .build().

    // Identity
    .runner_id(Some("runner-01".to_string()))
    .runner_name(Some("Primary Runner".to_string()))
    .routing_config(None)

    .build();
```

## DefaultRunnerBuilder

For constructing a `DefaultRunner` instance with database and schema configuration:

```rust
use cloacina::runner::DefaultRunnerBuilder;

// Single-tenant PostgreSQL
let runner = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .build()
    .await?;

// Multi-tenant with schema isolation
let tenant_runner = DefaultRunnerBuilder::new()
    .database_url("postgresql://user:pass@localhost/cloacina")
    .schema("tenant_acme")
    .with_config(config)
    .build()
    .await?;
```

| Method | Description |
|---|---|
| `database_url(&str)` | Sets the database connection URL (required) |
| `schema(&str)` | Sets the PostgreSQL schema for multi-tenant isolation. Must be alphanumeric + underscores. PostgreSQL only. |
| `with_config(DefaultRunnerConfig)` | Sets the full runner configuration |
| `routing_config(RoutingConfig)` | Sets task routing configuration |
| `build()` | Builds and starts the runner (creates DB, runs migrations, starts background services) |

## config.toml

The daemon and server read `~/.cloacina/config.toml`. See the [CLI Reference]({{< ref "cli" >}}) for the full schema and key paths.

### Mapping to DefaultRunnerConfig

The daemon maps `config.toml` values to `DefaultRunnerConfig` fields:

| config.toml Key | DefaultRunnerConfig Field |
|---|---|
| `daemon.poll_interval_ms` | `cron_poll_interval` (via CLI `--poll-interval`) |
| `daemon.trigger_poll_interval_ms` | `trigger_base_poll_interval` |
| `daemon.cron_max_catchup` | `cron_max_catchup_executions` |
| `daemon.cron_recovery_interval_s` | `cron_recovery_interval` |

> **Note:** `daemon.cron_lost_threshold_min` exists in `config.toml` but is not currently wired to `DefaultRunnerConfig` in the daemon command. The `cron_lost_threshold_minutes` field uses its default value (10 minutes).

The server uses `DefaultRunnerConfig::builder().registry_storage_backend("database").build()`.

## Environment Variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | Database connection URL for `serve` and `admin` commands |
| `CLOACINA_BOOTSTRAP_KEY` | Bootstrap API key for `serve` first startup |
| `RUST_LOG` | Log filter directive (e.g., `info`, `debug`, `cloacina=trace`) |

## See Also

- [CLI Reference]({{< ref "cli" >}}) -- config.toml schema and `config get/set/list` commands
- [Cron Scheduling Architecture]({{< ref "/explanation/workflows/cron-scheduling" >}}) -- how cron config affects scheduling behavior
- [DatabaseAdmin API]({{< ref "database-admin" >}}) -- multi-tenant database setup
