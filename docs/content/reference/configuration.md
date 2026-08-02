---
title: "Configuration Reference"
description: "Complete reference for DefaultRunnerConfig fields, builder methods, config.toml schema, and environment variables"
weight: 7
aliases:
  - "/platform/reference/configuration/"

---

# Configuration Reference

This page documents all configuration options for the Cloacina runtime. Configuration is specified programmatically via `DefaultRunnerConfig` (Rust API), through `~/.cloacina/config.toml` (read by `cloacinactl` — the daemon, and `server start` forwarding), or via command-line flags and environment variables (the `cloacina-server` binary has no config file).

## Server flags (`cloacina-server`)

When you run Cloacina as a service, the `cloacina-server` binary takes these
command-line flags (most also accept an environment variable). The
`cloacinactl server start` wrapper forwards the same settings.

| Flag | Env var | Default | Purpose |
|------|---------|---------|---------|
| `--bind` | — | `127.0.0.1:8080` | HTTP listen address. Set `0.0.0.0:8080` to accept remote connections (containers, behind a proxy). The server exposes a **single** HTTP port. |
| `--database-url` | `DATABASE_URL` | _(required)_ | Postgres or SQLite connection URL. |
| `--bootstrap-key` | `CLOACINA_BOOTSTRAP_KEY` | auto-generated | First-run admin key; if unset, one is generated and written once to `~/.cloacina/bootstrap-key`. |
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | off | Reject unsigned package uploads. Requires `--verification-org-id`. |
| `--verification-org-id` | `CLOACINA_VERIFICATION_ORG_ID` | — | Trusted org UUID for signature verification (mandatory when signatures are required). |
| `--tenant-runner-cache-size` | `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `256` | LRU cap on cached per-tenant runners. |
| `--tenant-deletion-drain-timeout-s` | `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `30` | Seconds to drain in-flight work before a tenant's runner is hard-evicted on `DELETE /v1/tenants/{name}`. |
| `--default-executor` | `CLOACINA_DEFAULT_EXECUTOR` | `default` | Executor every task is dispatched to; set `fleet` to route to the execution-agent fleet. An unknown key fails startup fast. |
| `--agent-heartbeat-interval-s` | `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `15` | Fleet-agent heartbeat cadence advertised to agents. |
| `--agent-liveness-misses` | `CLOACINA_AGENT_LIVENESS_MISSES` | `3` | Missed heartbeats before an agent is considered dead (dead-after = interval × misses). |
| `--cors-allowed-origins` | `CLOACINA_CORS_ALLOWED_ORIGINS` | unset | Comma-separated origins (`*` allowed). Unset means **CORS is disabled** — browser clients on other origins are blocked. |
| `--cors-allowed-methods` | `CLOACINA_CORS_ALLOWED_METHODS` | `GET,POST,DELETE,OPTIONS` | Applies only when CORS is enabled. |
| `--cors-allowed-headers` | `CLOACINA_CORS_ALLOWED_HEADERS` | `authorization,content-type` | Applies only when CORS is enabled. |
| `--reconcile-interval-s` | — | runtime default | Seconds between reconciler passes. |
| `--log-retention-days` | — | `14` | Daily-rotated log files to keep (`0` = unbounded). |
| `--home` | — | `~/.cloacina` | Home directory for keys, logs, config. |
| `-v`, `--verbose` | — | off | Debug logging (overrides `RUST_LOG`). |

The server binary has **no config file** of its own — flags and
environment variables are the whole surface. `~/.cloacina/config.toml`
belongs to `cloacinactl`, whose `server start` resolves
`[server].default_executor` and `database_url` from it and forwards
them as flags. The one subcommand is `cloacina-server emit-openapi`,
which prints the OpenAPI 3.1 document to stdout without needing a
database.

For the full deployment walkthrough see [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}) and [Running the server image]({{< ref "/service/how-to/running-the-server-image" >}}).

## DefaultRunnerConfig

The `DefaultRunnerConfig` struct controls all runtime behavior of the `DefaultRunner`. Create one with the builder pattern:

```rust
use cloacina::runner::DefaultRunnerConfig;
use std::time::Duration;

let config = DefaultRunnerConfig::builder()
    .max_concurrent_tasks(8)
    .task_timeout(Duration::from_secs(600))
    .enable_cron_scheduling(false)
    .build()?;
```

`build()` returns `Result<DefaultRunnerConfig, ConfigError>` — it
validates the configuration (see [Validation
constraints](#validation-constraints)). The struct is
`#[non_exhaustive]` with private fields: read values through the
same-named getter methods, and construct only through the builder.

### Concurrency

| Field | Type | Default | Description |
|---|---|---|---|
| `max_concurrent_tasks` | `usize` | `4` | Maximum number of task executions running simultaneously. Controls the semaphore size for the task executor. |
| `scheduler_poll_interval` | `Duration` | `100ms` | How often the task scheduler checks for tasks whose dependencies are satisfied and are ready to execute. |
| `task_timeout` | `Duration` | `300s` (5 min) | Maximum time allowed for a single task to execute before it is considered timed out. |
| `workflow_timeout` | `Option<Duration>` | `Some(3600s)` (1 hr) | Maximum time the blocking `execute()` call waits for a workflow execution to finish. `None` disables the timeout. Applies only to `execute()`'s wait loop — `execute_async` handles are not bounded by it. |
| `db_pool_size` | `u32` | `10` | Number of database connections in the connection pool. |
| `enable_recovery` | `bool` | `true` | Whether the stale-claim sweeper runs to reclaim task executions whose runner heartbeats expired. The sweeper is the *sole* task-recovery path — recovery is heartbeat-driven only. Disable only if you're running outside the standard runner loop. |

### Cron Scheduling

| Field | Type | Default | Description |
|---|---|---|---|
| `enable_cron_scheduling` | `bool` | `true` | Master switch for cron scheduling. When disabled, no cron schedules are evaluated. |
| `cron_poll_interval` | `Duration` | `30s` | How often the cron scheduler checks for schedules that are due. |
| `cron_max_catchup_executions` | `usize` | `100` | Maximum number of missed cron executions to catch up on after downtime. The builder rejects values above `1000`. |
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
| `enable_registry_reconciler` | `bool` | `true` | Whether the background registry reconciler runs to detect new/removed workflow packages. If the registry backend fails to construct, the runner logs an error and continues **without** a registry — packaged workflows then never load. |
| `registry_reconcile_interval` | `Duration` | `5s` | How often the reconciler scans for changes. |
| `registry_enable_startup_reconciliation` | `bool` | `true` | Whether to run a full reconciliation pass on startup. |
| `registry_storage_path` | `Option<PathBuf>` | `None` | Path for filesystem-based registry storage. `None` falls back to `std::env::temp_dir()/cloacina_registry` — a temp location that can be cleared on reboot. |
| `registry_storage_backend` | `String` | `"filesystem"` | Storage backend type. Options: `"filesystem"`, `"sqlite"`, `"postgres"`, `"database"` (the last three all select unified database storage). Any other value is an error. The server uses `"database"`. |

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

### Package Signing

| Field | Type | Default | Description |
|---|---|---|---|
| `require_signatures` | `bool` | `false` | When `true`, the registry reconciler refuses to load packages that have no stored signature. |
| `verification_org_id` | `Option<UniversalUuid>` | `None` | Trusted org UUID used for signature-verification audit logging. |

### Executor Routing

| Field | Type | Default | Description |
|---|---|---|---|
| `default_executor` | `String` | `"default"` | Executor key every task is dispatched to. `"default"` is the in-process thread executor; the server registers `"fleet"` for the agent fleet. |

### Tenant

| Field | Type | Default | Description |
|---|---|---|---|
| `tenant_id` | `String` | `"public"` | Tenant namespace applied to reconciled package tasks (`tenant::package::workflow::task`). Getter `tenant_id()`, setter `set_tenant_id()`. |

### Validation constraints

`DefaultRunnerConfigBuilder::build()` returns
`Result<DefaultRunnerConfig, ConfigError>` and rejects:

| Constraint | Rule |
|---|---|
| `max_concurrent_tasks` | must be `> 0` |
| `scheduler_poll_interval` | must be `>= 10ms` |
| `db_pool_size` | must be `> 0` |
| `cron_max_catchup_executions` | must be `<= 1000` |
| `stale_claim_threshold` | must be greater than `heartbeat_interval` |

## DefaultRunnerConfigBuilder

All builder methods consume and return `self` for chaining. Every
config field except `tenant_id` has a same-named builder setter
(`tenant_id` is set after `build()` via
`config.set_tenant_id(...)`):

```rust
let config = DefaultRunnerConfig::builder()
    // Concurrency
    .max_concurrent_tasks(8)
    .scheduler_poll_interval(Duration::from_millis(200))
    .task_timeout(Duration::from_secs(600))
    .workflow_timeout(Some(Duration::from_secs(7200)))
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

    // Signing
    .require_signatures(false)
    .verification_org_id(None)

    // Claiming
    .enable_claiming(true)
    .heartbeat_interval(Duration::from_secs(10))
    .stale_claim_sweep_interval(Duration::from_secs(30))
    .stale_claim_threshold(Duration::from_secs(60))

    // Executor routing
    .default_executor("default")

    // Identity
    .runner_id(Some("runner-01".to_string()))
    .runner_name(Some("Primary Runner".to_string()))

    .build()?;
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
| `database_url(&str)` | Sets the database connection URL (required — `build()` errors without it) |
| `schema(&str)` | Sets the PostgreSQL schema for multi-tenant isolation. Must be alphanumeric + underscores. PostgreSQL only — combining `schema` with a SQLite URL is a build-time `Configuration` error. |
| `with_config(DefaultRunnerConfig)` | Sets the full runner configuration |
| `runtime(Runtime)` | Sets a scoped `Runtime` for this runner; its registries are used instead of a fresh one. If neither `runtime` nor `runtime_arc` is set, the default is a fresh inventory-seeded `Runtime` (`Runtime::default()`). |
| `runtime_arc(Arc<Runtime>)` | Shares an existing `Arc<Runtime>`; wins over `runtime(...)` when both are set. |
| `secret_resolver(Arc<dyn SecretResolver>)` | Wires the resolver behind `context.secret(...)`. Unset means secrets fail closed with "secrets backend not configured". |
| `default_executor(impl Into<String>)` | Executor key for dispatch (default `"default"`). |
| `build()` | `async` — returns `Result<DefaultRunner, WorkflowExecutionError>`. Runs migrations (or `setup_schema` when `schema` is set) and **starts background services immediately** — a freshly built runner is already polling. Call `shutdown()` before dropping; `Drop` only logs a warning. |

## config.toml

`~/.cloacina/config.toml` belongs to **cloacinactl**: the daemon reads
it directly, and `cloacinactl server start` resolves values from it
before exec'ing the `cloacina-server` binary (which itself never reads
a config file). See the [CLI Reference]({{< ref "cli" >}}) for the
full schema and key paths. Note the load behavior: a missing file
yields silent defaults, and a file that fails to parse (including one
unknown key — the schema is `deny_unknown_fields`) is logged as a
warning and **ignored entirely**, falling back to defaults.

### Mapping to DefaultRunnerConfig

The daemon maps `config.toml` values to `DefaultRunnerConfig` fields:

| config.toml Key | DefaultRunnerConfig Field |
|---|---|
| `daemon.poll_interval_ms` | `cron_poll_interval` (via CLI `--poll-interval`); also drives the daemon's registry-reconcile interval |
| `daemon.trigger_poll_interval_ms` | `trigger_base_poll_interval` |
| `daemon.cron_max_catchup` | `cron_max_catchup_executions` (unset = not overridden; the runtime default `100` applies) |
| `daemon.cron_recovery_interval_s` | `cron_recovery_interval` |

> **Note:** `daemon.cron_lost_threshold_min` exists in `config.toml` but is not currently wired to `DefaultRunnerConfig` in the daemon command. The `cron_lost_threshold_minutes` field uses its default value (10 minutes).

The server uses `DefaultRunnerConfig::builder().registry_storage_backend("database").build()`.

### `[server]` section

The `[server]` section configures server-level deployment knobs read by `cloacinactl server start` (which forwards them to the `cloacina-server` binary):

| config.toml Key | Default | Description |
|---|---|---|
| `server.default_executor` | `"default"` | Executor key every task is dispatched to. There is no per-task routing — all work goes to this one executor. `"default"` is the in-process thread executor; set `"fleet"` to offload all work to the [execution-agent fleet]({{< ref "/service/explanation/execution-agent-fleet" >}}). The key is hard-matched against registered executors at server startup; an unknown key fails fast (no silent fallback). |

```toml
[server]
default_executor = "fleet"
```

Overrides for ad-hoc/direct runs use `cloacina-server --default-executor <key>`, `cloacinactl server start --default-executor <key>`, or `CLOACINA_DEFAULT_EXECUTOR=<key>`. Precedence: explicit CLI/env > `config.toml` `[server].default_executor` > built-in `default`.

## Environment Variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | Database connection URL for `server start` and `admin` commands |
| `CLOACINA_BOOTSTRAP_KEY` | Bootstrap API key for `server start` first startup |
| `CLOACINA_DEFAULT_EXECUTOR` | Executor key every task is dispatched to (overrides `[server].default_executor`; default `default`, set `fleet` for the agent fleet) |
| `RUST_LOG` | Log filter directive (e.g., `info`, `debug`, `cloacina=trace`) |

## Schema naming rules

PostgreSQL schema names used for multi-tenant isolation (`DefaultRunner::with_schema`,
`DatabaseAdmin` tenant provisioning) are validated before use in SQL to prevent
injection. A schema name must satisfy all of the following:

| Rule | Detail |
|---|---|
| Length | 1–63 characters (PostgreSQL `NAMEDATALEN` − 1). |
| First character | A letter (`a`–`z`, `A`–`Z`) or an underscore (`_`). |
| Remaining characters | Alphanumeric or underscore only. No hyphens, dots, spaces, or other symbols. ASCII only — Unicode letters are rejected. |
| Reserved names | `public`, `pg_catalog`, `information_schema`, and `pg_temp` are rejected (case-insensitive). |

| Valid | Invalid |
|---|---|
| `tenant_123` | `tenant-123` (hyphen) |
| `acme_corp` | `123abc` (starts with a digit) |
| `_private` | `tenant.123` (dot) |
| `production_api` | `public` (reserved) |

An invalid name is rejected with a `SchemaError` (`InvalidLength`, `InvalidStart`,
`InvalidCharacters`, or `ReservedName`). Tenant usernames follow the same rules,
with their own reserved list of PostgreSQL role names (`postgres`, `pg_*`).

## See Also

- [CLI Reference]({{< ref "cli" >}}) -- config.toml schema and `config get/set/list` commands
- [Environment Variables]({{< ref "environment-variables" >}}) -- full env-var reference (server, compiler, daemon, install script)
- [Metrics Catalog]({{< ref "metrics-catalog" >}}) -- `cloacina_*` metric surface emitted by configured runners
- [Cron Scheduling Architecture]({{< ref "/engine/explanation/cron-scheduling" >}}) -- how cron config affects scheduling behavior
- [DatabaseAdmin API]({{< ref "database-admin" >}}) -- multi-tenant database setup
