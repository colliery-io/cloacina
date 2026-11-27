---
title: "Configuration Reference"
description: "Complete reference for DefaultRunnerConfig fields, builder methods, config.toml schema, and environment variables"
sidebar:
  order: 7
---

This page documents all configuration options for the Cloacina runtime. Configuration is specified programmatically via `DefaultRunnerConfig` (Rust API), through `~/.cloacina/config.toml` (read by `cloacinactl` — the daemon, and `server start` forwarding), or via command-line flags and environment variables (the `cloacina-server` binary has no config file).

## Server flags (`cloacina-server`)

When you run Cloacina as a service, the `cloacina-server` binary takes these
command-line flags (most also accept an environment variable). The
`cloacinactl server start` wrapper forwards the same settings.

| Flag | Env var | Default | Purpose |
|------|---------|---------|---------|
| `--bind` | — | `127.0.0.1:8080` | HTTP listen address. Set `0.0.0.0:8080` to accept remote connections (containers, behind a proxy). The server exposes a **single** HTTP port unless `--metrics-bind` is set. |
| `--metrics-bind` | `CLOACINA_METRICS_BIND` | — | Address of a separate listener for `/metrics` (for example `0.0.0.0:9090`). When set, `/metrics` is served only there and the main listener answers 404 for it. When not set, `/metrics` stays on `--bind` with no authentication. |
| `--alert-webhook-url` | `CLOACINA_ALERT_WEBHOOK_URL` | — | URL that gets a JSON `POST` when a run ends. Not set = off. See [Alert on a Failed Run](../../how-to/service/alert-on-a-failed-run/). |
| `--alert-webhook-events` | `CLOACINA_ALERT_WEBHOOK_EVENTS` | `failed` | Outcomes that send an alert: `failed`, `completed`, `cancelled`, or `all`. |
| — | `CLOACINA_ALERT_WEBHOOK_SECRET` | — | Shared secret that signs each alert request. Environment only. |
| `--alert-webhook-timeout-s` | `CLOACINA_ALERT_WEBHOOK_TIMEOUT_S` | `10` | Seconds allowed for one alert request. |
| `--alert-webhook-max-attempts` | `CLOACINA_ALERT_WEBHOOK_MAX_ATTEMPTS` | `8` | The most sends of one alert before it is dropped. |
| `--public-base-url` | `CLOACINA_PUBLIC_BASE_URL` | — | Public URL of the server, for the absolute links in an alert. |
| `--log-format` | `CLOACINA_LOG_FORMAT` | `text` | Format of the log lines on stderr: `text` or `json`. The log file under `--home` is JSON in both. |
| `--task-log-url-template` | `CLOACINA_TASK_LOG_URL_TEMPLATE` | — | URL of a task's logs in your log stack, with the placeholders `{tenant}`, `{execution_id}`, `{task}` and `{attempt}`. The web UI shows it as a link on each task. Not set = no link. See [Find a Task's Logs](../../how-to/service/find-a-tasks-logs/). |
| `--database-url` | `DATABASE_URL` | _(required)_ | Postgres or SQLite connection URL. |
| `--max-package-bytes` | `CLOACINA_MAX_PACKAGE_BYTES` | `104857600` | The largest package archive, in bytes, that an upload accepts. A larger upload is refused with status 413. |
| `--max-unpacked-bytes` | `CLOACINA_MAX_UNPACKED_BYTES` | `524288000` | The largest size, in bytes, that a package archive may unpack to. A larger archive is refused at upload and at load. The compiler, the agent and `cloacinactl` read the same setting. |
| `--database-wait-timeout-s` | `CLOACINA_DATABASE_WAIT_TIMEOUT_S` | `60` | Seconds to wait at start for the database to accept a connection, with retries. After it the server exits with an error. `0` = one try. |
| `--bootstrap-key` | `CLOACINA_BOOTSTRAP_KEY` | auto-generated | First-run admin key; if unset, one is generated and written once to `~/.cloacina/bootstrap-key`. |
| `--require-signatures` | `CLOACINA_REQUIRE_SIGNATURES` | off | Reject package uploads that are unsigned, or signed by a key that neither the tenant nor the server-wide org trusts. |
| `--verification-org-id` | `CLOACINA_VERIFICATION_ORG_ID` | — | Server-wide trusted org UUID (optional). A key trusted in it is accepted for every tenant, after the tenant's own keys. |
| `--tenant-runner-cache-size` | `CLOACINA_TENANT_RUNNER_CACHE_SIZE` | `256` | Warning limit on per-tenant runners; none is stopped at the limit. |
| `--tenant-deletion-drain-timeout-s` | `CLOACINA_TENANT_DELETION_DRAIN_TIMEOUT_S` | `30` | Seconds to drain in-flight work before a tenant's runner is hard-evicted on `DELETE /v1/tenants/{name}`. |
| `--default-executor` | `CLOACINA_DEFAULT_EXECUTOR` | `default` | Executor every task is dispatched to; set `fleet` to route to the execution-agent fleet. An unknown key fails startup fast. |
| `--agent-heartbeat-interval-s` | `CLOACINA_AGENT_HEARTBEAT_INTERVAL_S` | `15` | Fleet-agent heartbeat cadence advertised to agents. |
| `--agent-liveness-misses` | `CLOACINA_AGENT_LIVENESS_MISSES` | `3` | Missed heartbeats before an agent is considered dead (dead-after = interval × misses). |
| `--fleet-task-timeout-s` | `CLOACINA_FLEET_TASK_TIMEOUT_S` | `300` | Time limit (seconds) of a fleet task that sets none of its own. |
| `--fleet-task-timeout-max-s` | `CLOACINA_FLEET_TASK_TIMEOUT_MAX_S` | `86400` | Longest time limit (seconds) a fleet task can get; a task that asks for more gets this value. |
| `--fleet-graph-firings` | `CLOACINA_FLEET_GRAPH_FIRINGS` | `inline` | Where an agent runs a computation-graph firing: `inline` or `worker`. |
| `--fleet-artifact-fetch-wait-s` | `CLOACINA_FLEET_ARTIFACT_FETCH_WAIT_S` | `300` | How long (seconds) a fleet task waits, per attempt, while agents cannot fetch its package artifact; after that the attempt fails. `0`: the first failed fetch fails the attempt. |
| `--context-warn-bytes` | `CLOACINA_CONTEXT_WARN_BYTES` | `1048576` (1 MiB) | Size in bytes above which a task context is logged as a warning and counted in `cloacina_context_size_warnings_total`. `0` = no warning. See [Context size limits](../../explanation/engine/context-management/#context-size-limits). |
| `--context-max-bytes` | `CLOACINA_CONTEXT_MAX_BYTES` | `16777216` (16 MiB) | Largest size in bytes of a task context. A task whose result or input context is larger fails and is not retried. One setting for the whole server, not per tenant. `0` = no limit. |
| `--cors-allowed-origins` | `CLOACINA_CORS_ALLOWED_ORIGINS` | unset | Comma-separated origins (`*` allowed). Unset means **CORS is disabled** — browser clients on other origins are blocked. |
| `--cors-allowed-methods` | `CLOACINA_CORS_ALLOWED_METHODS` | `GET,POST,DELETE,OPTIONS` | Applies only when CORS is enabled. |
| `--cors-allowed-headers` | `CLOACINA_CORS_ALLOWED_HEADERS` | `authorization,content-type` | Applies only when CORS is enabled. |
| `--run-history-retention-days` | `CLOACINA_RUN_HISTORY_RETENTION_DAYS` | `0` (off) | Days to keep a finished workflow run and its rows before the retention job deletes them (`0` = keep for ever). See [Run history retention](../../how-to/service/production-deployment/#run-history-retention). |
| `--outbox-acked-retention-s` | `CLOACINA_OUTBOX_ACKED_RETENTION_S` | `0` (off) | Seconds to keep a `delivery_outbox` row after its ack. `0` = keep for ever: `delivery_outbox` grows until a period is set. |
| `--retention-interval-s` | `CLOACINA_RETENTION_INTERVAL_S` | `300` | Seconds between passes of the retention job. |
| `--reconcile-interval-s` | — | runtime default | Seconds between reconciler passes. |
| `--log-retention-days` | — | `14` | Daily-rotated log files to keep (`0` = unbounded). |
| `--home` | — | `~/.cloacina` | Home directory for keys, logs, config. |
| `-v`, `--verbose` | — | off | Debug logging (overrides `RUST_LOG`). |

The server binary has **no config file** of its own — flags and
environment variables are the whole surface. `~/.cloacina/config.toml`
belongs to `cloacinactl`, whose `server start` resolves `database_url`
and the keys of the [`[server]` section](#server-section) from it and
forwards them to the server. The Helm chart `cloacina-server` sets the
retention, fleet time limit, artifact fetch wait, package size, context size
and alert webhook settings from its values `retention.*`,
`fleetTaskTimeLimits.*`, `packages.*`, `context.*` and `alertWebhook.*`. The one subcommand is `cloacina-server emit-openapi`,
which prints the OpenAPI 3.1 document to stdout without needing a
database.

For the full deployment walkthrough see [Deploying the API Server](../../how-to/service/deploying-the-api-server/) and [Running the server image](../../how-to/service/running-the-server-image/).

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
| `cron_max_catchup_executions` | `usize` | `100` | The most missed fire times that one claim of a `run_all` schedule takes. It is a batch size, not a cap: the fire times beyond it stay due and the next pass takes them. The builder rejects values above `1000`; `0` is read as `1`. |
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
| `require_signatures` | `bool` | `false` | When `true`, the registry reconciler verifies the stored signature of each package at load: a row stored for the org of the runner's tenant or for `verification_org_id`, a live trusted key of one of the two, and a signature that verifies. A package that does not pass is not loaded. |
| `verification_org_id` | `Option<UniversalUuid>` | `None` | The server-wide org of the load check, asked after the org of the runner's tenant. |
| `signature_trust_database` | `Option<Database>` | `None` | The database that holds the trusted keys of the load check. `None`: the runner's own database. |

### Executor Routing

| Field | Type | Default | Description |
|---|---|---|---|
| `default_executor` | `String` | `"default"` | Executor key every task is dispatched to. `"default"` is the in-process thread executor; the server registers `"fleet"` for the agent fleet. |

### Context Size

| Field | Type | Default | Description |
|---|---|---|---|
| `context_warn_bytes` | `u64` | `1048576` (1 MiB) | Serialized size above which a task context is logged as a warning and counted in `cloacina_context_size_warnings_total`. `0` = no warning. |
| `context_max_bytes` | `u64` | `16777216` (16 MiB) | Largest serialized size of a task context. A task whose result or input context is larger fails with `ExecutorError::ContextTooLarge` and is not retried, whatever its retry policy. `0` = no limit. |

Both have a getter and a builder setter of the same name. `context_limits()` returns both as a `ContextSizeLimits`. The server sets them from `--context-warn-bytes` and `--context-max-bytes`. The daemon and the Python `DefaultRunnerConfig` use the defaults. See [Context size limits](../../explanation/engine/context-management/#context-size-limits).

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
a config file). See the [CLI Reference](../cli/) for the
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
| `server.default_executor` | `"default"` | Executor key every task is dispatched to. There is no per-task routing — all work goes to this one executor. `"default"` is the in-process thread executor; set `"fleet"` to offload all work to the [execution-agent fleet](../../explanation/service/execution-agent-fleet/). The key is hard-matched against registered executors at server startup; an unknown key fails fast (no silent fallback). |
| `server.database_wait_timeout_s` | not set | Seconds the server waits at start for the database to accept a connection; `0` = one try. Forwarded as `--database-wait-timeout-s`. |
| `server.run_history_retention_days` | not set | Days to keep a finished run; `0` keeps run history for ever. Forwarded as `--run-history-retention-days`. |
| `server.outbox_acked_retention_s` | not set | Seconds to keep an acked `delivery_outbox` row; `0` keeps the rows for ever. Forwarded as `--outbox-acked-retention-s`. |
| `server.retention_interval_s` | not set | Seconds between passes of the retention job. Forwarded as `--retention-interval-s`. |
| `server.fleet_task_timeout_s` | not set | Time limit (seconds) of a fleet task that sets none of its own, and of a graph firing on the fleet. Forwarded as `--fleet-task-timeout-s`. |
| `server.fleet_task_timeout_max_s` | not set | Longest time limit (seconds) a fleet task can get. Forwarded as `--fleet-task-timeout-max-s`. |
| `server.alert_webhook_url` | not set | URL that gets a JSON `POST` when a run ends. Forwarded as the env var `CLOACINA_ALERT_WEBHOOK_URL`, not as an argument, because the URL can hold a token. |
| `server.alert_webhook_events` | `[]` | Run outcomes that send an alert: `failed`, `completed`, `cancelled`, or `all`. An empty list is not forwarded. Forwarded as `--alert-webhook-events`. |
| `server.alert_webhook_timeout_s` | not set | Seconds allowed for one alert request. Forwarded as `--alert-webhook-timeout-s`. |
| `server.alert_webhook_max_attempts` | not set | The most sends of one alert. Forwarded as `--alert-webhook-max-attempts`. |
| `server.public_base_url` | not set | Public URL of the server, for the links in an alert. Forwarded as `--public-base-url`. |
| `server.task_log_url_template` | not set | URL of a task's logs in your log stack, with the placeholders `{tenant}`, `{execution_id}`, `{task}` and `{attempt}`. Forwarded as `--task-log-url-template`. |
| `server.verification_org_id` | not set | Server-wide trusted organization UUID. Forwarded as `--verification-org-id`. |
| `server.reconcile_interval_s` | not set | Seconds between reconciler passes. Forwarded as `--reconcile-interval-s`. |
| `server.tenant_runner_cache_size` | not set | Warning limit on per-tenant runners. Forwarded as `--tenant-runner-cache-size`. |
| `server.tenant_deletion_drain_timeout_s` | not set | Seconds a tenant delete waits for running workflows. Forwarded as `--tenant-deletion-drain-timeout-s`. |
| `server.log_retention_days` | not set | Daily log files to keep; `0` keeps them all. Forwarded as `--log-retention-days`. |
| `server.log_format` | not set | `text` or `json`. Forwarded as `--log-format`. |
| `server.agent_heartbeat_interval_s` | not set | Heartbeat interval (seconds) of fleet agents. Forwarded as `--agent-heartbeat-interval-s`. |
| `server.agent_liveness_misses` | not set | Missed heartbeats before an agent is dead. Forwarded as `--agent-liveness-misses`. |
| `server.fleet_artifact_fetch_wait_s` | not set | Seconds a fleet task waits, per attempt, while agents cannot fetch its package artifact. Forwarded as `--fleet-artifact-fetch-wait-s`. |
| `server.fleet_graph_firings` | not set | Where an agent runs a computation-graph firing: `inline` or `worker`. Forwarded as `--fleet-graph-firings`. |
| `server.context_warn_bytes` | not set | Size in bytes above which a task context is logged as a warning. Forwarded as `--context-warn-bytes`. |
| `server.context_max_bytes` | not set | Largest size in bytes of a task context. Forwarded as `--context-max-bytes`. |
| `server.cors_allowed_origins` | `[]` | Origins allowed by CORS. An empty list is not forwarded. Forwarded as `--cors-allowed-origins`. |
| `server.cors_allowed_methods` | `[]` | Methods allowed by CORS. An empty list is not forwarded. Forwarded as `--cors-allowed-methods`. |
| `server.cors_allowed_headers` | `[]` | Request headers allowed by CORS. An empty list is not forwarded. Forwarded as `--cors-allowed-headers`. |
| `server.max_package_bytes` | not set | The largest package archive, in bytes, that an upload accepts. Forwarded as `--max-package-bytes`. |
| `server.max_unpacked_bytes` | not set | The largest size, in bytes, that a package archive may unpack to. Forwarded as `--max-unpacked-bytes`. |

A key that is not set is not forwarded, and the server uses its own default
(see [Server flags](#server-flags-cloacina-server)). The signing secret of the
alert webhook has no key: set the env var `CLOACINA_ALERT_WEBHOOK_SECRET`, which
the server started by `cloacinactl server start` inherits. Every flag of
`cloacina-server` but `--home`, `--verbose` and the base flags of `server start`
(`--bind`, `--metrics-bind`, `--database-url`, `--bootstrap-key`,
`--require-signatures`) has a key here.

```toml
[server]
default_executor = "fleet"
run_history_retention_days = 30
fleet_task_timeout_s = 600
alert_webhook_url = "https://alerts.example.com/hook"
alert_webhook_events = ["failed", "cancelled"]
public_base_url = "https://cloacina.example.com"
```

Each key has a flag of the same name on `cloacinactl server start` (for example `--default-executor <key>`, `--fleet-task-timeout-s <N>`) and the env var of the server setting (for example `CLOACINA_DEFAULT_EXECUTOR`, `CLOACINA_FLEET_TASK_TIMEOUT_S`). Precedence: flag > env var > `config.toml` `[server]` > the server's default. `--reconcile-interval-s` and `--log-retention-days` have no env var.

### `[compiler]` section

`local_addr` is the address that `cloacinactl compiler status` and
`compiler health` probe. The other keys are read by `cloacinactl compiler
start`, which forwards them to the `cloacina-compiler` binary:

| config.toml Key | Default | Description |
|---|---|---|
| `compiler.local_addr` | `"127.0.0.1:9000"` | Status and health address of the compiler. |
| `compiler.max_unpacked_bytes` | not set | The largest size, in bytes, that a package archive may unpack to. Forwarded as `--max-unpacked-bytes`. |
| `compiler.python_platforms` | `[]` | Platforms to vendor Python dependencies for, `<arch>-<os>`. An empty list is not forwarded. Forwarded as `--python-platform`. |
| `compiler.python_version` | not set | The Python to vendor for, `major.minor`. Forwarded as `--python-version`. |
| `compiler.python_index_url` | not set | The package index of the wheels. Forwarded as the env var `CLOACINA_COMPILER_PYTHON_INDEX_URL`, not as an argument, because the URL can hold a password. |
| `compiler.python_wheel_dir` | not set | A folder of wheel files. Forwarded as `--python-wheel-dir`. |
| `compiler.python_uv_cache_dir` | not set | The cache folder of `uv`. Forwarded as `--python-uv-cache-dir`. |
| `compiler.python_offline` | not set | `true`: vendor with no network. Forwarded as `--python-offline`. |
| `compiler.uv` | not set | The `uv` program. Forwarded as `--uv`. |

Each key has a flag of the same name on `cloacinactl compiler start`
(`python_platforms` is `--python-platform`) and the env var of the compiler
setting. Precedence: flag > env var > `config.toml` `[compiler]` > the
compiler's default.

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

- [CLI Reference](../cli/) -- config.toml schema and `config get/set/list` commands
- [Environment Variables](../environment-variables/) -- full env-var reference (server, compiler, daemon, install script)
- [Metrics Catalog](../metrics-catalog/) -- `cloacina_*` metric surface emitted by configured runners
- [Cron Scheduling Architecture](../../explanation/engine/cron-scheduling/) -- how cron config affects scheduling behavior
- [DatabaseAdmin API](../database-admin/) -- multi-tenant database setup
