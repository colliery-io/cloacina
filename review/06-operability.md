# Operability Review

## Summary

Cloacina's operability posture is mixed. The daemon mode has an excellent lifecycle model -- SIGTERM/SIGINT/SIGHUP handling, graceful shutdown with configurable timeout, and a force-exit escape hatch. The server mode has genuine health/ready separation and graceful shutdown via axum's built-in mechanism. However, the system has no real metrics (the `/metrics` endpoint is a static string), no distributed tracing, no request-level correlation IDs, and the logging -- while consistent in use of `tracing` -- logs full database URLs containing credentials through the Python bindings. The `SchedulerLoop::run()` has no shutdown mechanism (confirmed by COR-03), which means the task readiness engine runs forever once started, preventing truly clean shutdown. For a 3am incident, you would have structured logs and health endpoints, but no metrics dashboards, no way to correlate a failing request through the system, and no production Dockerfile or K8s manifests to reference.

## Observability Assessment

**Logging**: The system uses `tracing` consistently throughout with appropriate level usage (error for failures, warn for degraded states, info for lifecycle events, debug for loop iterations). The daemon writes dual output: JSON to file (daily-rotated) and human-readable to stderr. The server does the same. Structured fields are used in security audit events (`event_type`, `org_id`, `key_id`, `key_fingerprint`). The `security/audit.rs` module provides well-structured SIEM-compatible audit logging for key and package operations. However, there are no request-scoped spans for HTTP requests, and the Python bindings log full database URLs (including credentials) via both `info!` and `eprintln!`.

**Metrics**: Effectively nonexistent. The `/metrics` endpoint returns a hardcoded string (`cloacina_up 1`) with a TODO comment. The `ExecutorMetrics` struct exists internally (active_tasks, total_executed, max_concurrent) but is not exported to Prometheus or any metrics collector. The four golden signals (latency, traffic, errors, saturation) cannot be measured from outside the process.

**Tracing**: Tracing spans exist only in the `services.rs` background task spawning code (5 `.instrument()` calls). There is no OpenTelemetry integration, no trace ID propagation through HTTP requests, no span creation for individual pipeline or task executions. The `create_runner_span` helper produces a static `"runner_task"` span with component and operation fields, but these are not propagated through to downstream operations.

**Health checks**: The server has a proper three-endpoint model (`/health` for liveness, `/ready` for readiness, `/metrics` for monitoring). The `/ready` endpoint checks database connectivity and computation graph health, returning 503 with structured error details when degraded. The reactive health endpoints (`/v1/health/accumulators`, `/v1/health/reactors`, `/v1/health/reactors/{name}`) provide per-component status. The daemon has no health endpoints (appropriate for its local-process model).

## Failure Mode Analysis

**Database unavailable at startup**: Both daemon and server fail fast with clear `anyhow` context ("Failed to create DefaultRunner", "Failed to connect to database"). The error messages include the chain. The `Database::new_with_schema` constructor panics (via `.expect()`) on pool creation failure; the `try_new_with_schema` fallible variant exists but is not used in the serve/daemon startup paths (they use `DefaultRunner::with_config` which delegates to the builder which does use the fallible path).

**Database unavailable at runtime**: The scheduler loop logs errors and continues (`Err(e) => error!("Scheduling loop error: {}", e)`). The readiness endpoint correctly reports `503 { "status": "not ready", "reason": "database unreachable" }`. Task claiming will fail and log errors but the process continues. There is no circuit breaker for sustained database outages -- the scheduler will continue polling at its configured interval (100ms default) and logging errors on every tick, potentially flooding logs.

**Computation graph crash**: Well handled. The `ReactiveScheduler` supervision loop detects crashed accumulators and reactors, applies exponential backoff (1s base, 60s max), limits recovery attempts (5 max), and records recovery events in the database. A circuit breaker permanently abandons components after 5 consecutive failures. Individual accumulators are restarted in-place without tearing down the reactor. Reactor crashes trigger a full graph restart.

**Plugin load failure**: Logged as a warning during reconciliation but does not prevent other packages from loading. Continuous via `continue_on_package_error: true` in the reconciler config.

**Graceful shutdown -- daemon**: Thorough. SIGINT/SIGTERM break the event loop, runner shutdown is attempted with a configurable timeout (default 30s), a second SIGINT forces immediate exit via `std::process::exit(1)`. Shutdown drains in-flight pipelines.

**Graceful shutdown -- server**: Reasonable. The shutdown signal triggers the reactive scheduler to shut down first (flush and persist), then axum's graceful shutdown drains in-flight HTTP requests. However, the `SchedulerLoop` (task readiness checker) has no shutdown mechanism -- it will be killed when the tokio runtime drops, potentially leaving database operations incomplete.

## Findings

### OPS-01: /metrics endpoint is a static placeholder -- no real metrics exist
**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs`, lines 321-332
**Confidence**: High

#### Description
The `/metrics` endpoint returns a hardcoded string containing a single gauge (`cloacina_up 1`) with a TODO comment: "Wire in prometheus metrics in a future task." The system has no Prometheus, StatsD, or any other metrics export. The `ExecutorMetrics` struct exists internally but is not connected to any export path.

Without metrics, operators cannot answer basic questions: How many workflows are running? What is the task execution latency? How many tasks are failing? Is the connection pool saturated? How many API requests per second? What is the scheduler loop processing time?

The four golden signals (latency, traffic, errors, saturation) are completely unmeasured. An operator paged at 3am has logs and a binary health check, but no trending data to determine when a problem started, how fast it is growing, or whether it is getting worse.

#### Evidence
```rust
async fn metrics() -> impl IntoResponse {
    // TODO: Wire in prometheus metrics in a future task
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4")],
        "# HELP cloacina_up Server is running\n# TYPE cloacina_up gauge\ncloacina_up 1\n",
    )
}
```

#### Suggested Resolution
Introduce `prometheus` or `metrics` crate and instrument the critical paths:
- **Latency**: pipeline execution duration, task execution duration, scheduler loop cycle time
- **Traffic**: pipelines started/completed per interval, API requests per second, WebSocket messages received
- **Errors**: task failures, pipeline failures, database errors, auth rejections
- **Saturation**: connection pool utilization, executor semaphore utilization, active concurrent tasks vs max
- **Business**: packages loaded, cron schedules active, trigger schedules active, computation graphs running

---

### OPS-02: No distributed tracing or request correlation
**Severity**: Major
**Location**: System-wide; `crates/cloacina/src/logging.rs`, `crates/cloacinactl/src/server/`
**Confidence**: High

#### Description
The system has no OpenTelemetry integration, no trace ID propagation, and no request correlation IDs. When an HTTP request triggers a pipeline execution, there is no way to correlate the HTTP request log with the pipeline creation log, the individual task execution logs, or the final completion log. Each log line is independent.

In a multi-tenant server with concurrent executions, this makes incident investigation extremely difficult. The scheduler loop logs, executor logs, and HTTP handler logs are interleaved with no connecting thread.

The `tracing` crate is already in use throughout, which means adding OpenTelemetry integration (`tracing-opentelemetry`) is relatively straightforward -- the spans just need to be created at the right boundaries (HTTP request, pipeline execution, task execution) and propagated through the call chain.

#### Evidence
- `logging.rs` uses `tracing_subscriber` with plain text or JSON formatting but no OpenTelemetry layer
- `services.rs` has 5 `.instrument()` calls but they create static spans without request-specific context
- HTTP handlers do not create request-scoped spans
- Pipeline executions do not carry a trace/correlation ID

#### Suggested Resolution
1. Add `tracing-opentelemetry` and an OTLP exporter as optional dependencies
2. Create a request-scoped span in an axum middleware layer with a request ID
3. Propagate pipeline_execution_id through executor and task logs as a span field
4. Optionally accept `traceparent` headers for distributed trace propagation

---

### OPS-03: Python bindings log database URLs with credentials
**Severity**: Major
**Location**: `crates/cloacina/src/python/bindings/runner.rs`, lines 308-312 and 1150-1151
**Confidence**: High

#### Description
The Python `PyDefaultRunner` constructor logs the full database URL, including embedded credentials, via both `info!()` (structured logging) and `eprintln!()` (raw stderr). This means anyone with access to logs or terminal output can read database passwords.

The server's `serve.rs` correctly uses a `mask_db_url()` helper that replaces passwords with `****`, but the Python bindings bypass this protection entirely. The `eprintln!` calls appear to be debug instrumentation that was never removed -- they include `"THREAD:"` prefixes.

#### Evidence
```rust
// runner.rs line 308-312
eprintln!(
    "THREAD: Creating DefaultRunner with database_url: {}",
    database_url
);
info!("Creating DefaultRunner with database_url: {}", database_url);

// runner.rs line 1150-1151
eprintln!("THREAD: Creating DefaultRunner with schema: {} and database_url: {}", schema, database_url);
info!("Creating DefaultRunner with schema: {} and database_url: {}", schema, database_url);
```

#### Suggested Resolution
1. Remove all `eprintln!("THREAD: ...")` debug statements from `runner.rs`
2. Apply `mask_db_url()` (or equivalent) to database URLs before logging
3. Consider auditing all `info!` calls that log URL-shaped strings

---

### OPS-04: SchedulerLoop::run() has no shutdown mechanism
**Severity**: Major
**Location**: `crates/cloacina/src/task_scheduler/scheduler_loop.rs`, lines 82-97
**Confidence**: High

#### Description
This is a restatement of COR-03 from the operability perspective. The `SchedulerLoop::run()` method is an infinite `loop {}` with no shutdown channel. When the runner shuts down, it signals the `broadcast::Sender` which is handled by the wrapper in `services.rs` via `tokio::select!`, but this aborts the scheduler loop mid-iteration rather than letting it complete its current cycle.

For the on-call engineer, this means:
- During shutdown, the scheduler loop may be interrupted mid-database-operation
- If the scheduler loop is in the middle of `process_active_pipelines()` when the runtime is dropped, the database transaction may not complete cleanly
- There is no log line indicating the scheduler loop accepted and processed a shutdown signal -- it just stops

The `StaleClaimSweeper` and `UnifiedScheduler` both have proper `watch::Receiver<bool>` shutdown support with `tokio::select!` in their run loops. The `SchedulerLoop` is the outlier.

#### Evidence
```rust
pub async fn run(&self) -> Result<(), ValidationError> {
    let mut interval = time::interval(self.poll_interval);
    loop {
        interval.tick().await;
        match self.process_active_pipelines().await {
            Ok(_) => debug!("Scheduling loop completed successfully"),
            Err(e) => error!("Scheduling loop error: {}", e),
        }
    }
}
```

#### Suggested Resolution
Add a `watch::Receiver<bool>` shutdown channel to `SchedulerLoop` and use `tokio::select!` in the loop body, matching the pattern in `StaleClaimSweeper::run()`.

---

### OPS-05: No circuit breaker for sustained database outages in the scheduler loop
**Severity**: Minor
**Location**: `crates/cloacina/src/task_scheduler/scheduler_loop.rs`, lines 89-96
**Confidence**: High

#### Description
When the database becomes unavailable, the scheduler loop logs an error on every tick and continues. With the default poll interval of 100ms, this produces 600 error log lines per minute. There is no backoff, no circuit breaker, and no log rate limiting.

The computation graph supervisor has proper backoff (exponential, capped at 60s) and a circuit breaker (5 attempts before permanent abandon). The scheduler loop has neither.

In a sustained outage, the log volume will obscure other diagnostic information and potentially exhaust disk space if writing to files (the daemon writes JSON logs to `~/.cloacina/logs/` with daily rotation but no size limit).

#### Evidence
```rust
loop {
    interval.tick().await;
    match self.process_active_pipelines().await {
        Ok(_) => debug!("Scheduling loop completed successfully"),
        Err(e) => error!("Scheduling loop error: {}", e),
    }
}
```

#### Suggested Resolution
Add a consecutive error counter with exponential backoff: after N consecutive errors, increase the poll interval (e.g., double it up to 30s). Reset to normal on success. This reduces log flood during outages while preserving responsiveness during normal operation. Optionally emit a rate-limited "circuit open" warning.

---

### OPS-06: No production Dockerfile or Kubernetes manifests
**Severity**: Minor
**Location**: `docker/Dockerfile.test` (the only Dockerfile)
**Confidence**: High

#### Description
The repository contains one Dockerfile (`docker/Dockerfile.test`) which is explicitly for reproducing CI tests locally. It is based on `catthehacker/ubuntu:full-latest` (a large GitHub Actions runner image), installs development tools like `gdb`, and has no multi-stage build or production optimization.

There is no production Dockerfile for building and running `cloacinactl serve`. There are no Kubernetes manifests, Helm charts, or Docker Compose files for deploying the server with PostgreSQL. The `install.sh` script handles binary distribution but not containerized deployment.

For an operator deploying Cloacina, there is no reference deployment configuration to start from -- they must build their own from scratch, which increases the risk of misconfiguration.

#### Evidence
- `docker/Dockerfile.test` -- CI reproduction only
- `install.sh` -- binary installer for local use
- No `Dockerfile`, `docker-compose.yml`, or `k8s/` directory for production

#### Suggested Resolution
Add a minimal production Dockerfile using a multi-stage build (Rust builder + distroless/alpine runtime). Add a `docker-compose.yml` that starts `cloacinactl serve` with a PostgreSQL instance for local development/testing of the server mode.

---

### OPS-07: Configuration is not validated at startup
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/config.rs`, lines 88-119; `crates/cloacina/src/runner/default_runner/config.rs`
**Confidence**: High

#### Description
Configuration values are accepted without validation. The `DefaultRunnerConfigBuilder::build()` returns the config directly without checking for invalid combinations or unreasonable values. The `CloacinaConfig::load()` method deserializes the TOML file with `#[serde(default)]` and falls back to defaults on any parse error, but never validates the semantic correctness of the values.

Examples of unvalidated configurations:
- `max_concurrent_tasks: 0` -- no tasks can execute, silent deadlock
- `scheduler_poll_interval: 0ms` -- tight polling loop consuming CPU
- `stale_claim_threshold` shorter than `heartbeat_interval` -- claims will always appear stale
- `db_pool_size: 0` -- pool creation will likely fail at runtime, not at config time
- `shutdown_timeout_s: 0` -- immediate forced shutdown, defeating the purpose of draining

The `DefaultRunnerBuilder::validate_schema_name()` does validate the schema name, showing that validation was intended but not applied comprehensively.

#### Evidence
```rust
pub fn build(self) -> DefaultRunnerConfig {
    self.config  // No validation
}
```

#### Suggested Resolution
Add a `validate()` method to `DefaultRunnerConfig` that checks invariants:
- `max_concurrent_tasks > 0`
- `stale_claim_threshold > heartbeat_interval`
- `scheduler_poll_interval > 0`
- `db_pool_size > 0`
Call it from `build()` and return a `Result`.

---

### OPS-08: Server graceful shutdown does not drain the runner's scheduler/executor
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/serve.rs`, lines 154-167
**Confidence**: Medium

#### Description
The server's graceful shutdown sequence shuts down the `ReactiveScheduler` and then relies on axum's `with_graceful_shutdown` to drain HTTP connections. However, it does not call `runner.shutdown()` to drain in-flight pipeline and task executions. The daemon correctly calls `runner.shutdown()` with a timeout, but the server omits this step.

This means that when the server receives SIGTERM:
1. The reactive scheduler shuts down (computation graphs flushed)
2. Axum stops accepting new HTTP connections and waits for in-flight requests
3. The process exits -- but the runner's background scheduler loop, task executor, cron recovery service, and stale claim sweeper are all terminated abruptly

In contrast, the daemon's shutdown calls `runner.shutdown()` which sends a shutdown signal, waits for the scheduler handle, waits for the executor handle, waits for the cron recovery handle, and closes the database pool.

#### Evidence
```rust
// Server shutdown (serve.rs)
.with_graceful_shutdown(async move {
    shutdown_signal().await;
    let _ = shutdown_tx.send(true);         // Signals reactive scheduler only
    let _ = scheduler_handle.await;          // Waits for reactive scheduler
})                                           // Runner.shutdown() is never called

// Daemon shutdown (daemon.rs)
tokio::select! {
    result = runner.shutdown() => { ... }    // Properly shuts down everything
    _ = tokio::time::sleep(shutdown_timeout) => { ... }
    _ = tokio::signal::ctrl_c() => { ... }
}
```

#### Suggested Resolution
After the reactive scheduler shutdown completes and before the process exits, call `runner.shutdown()` with a timeout (matching the daemon pattern).

---

### OPS-09: Daemon log rotation has no size limit
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/daemon.rs`, lines 139-146
**Confidence**: Medium

#### Description
The daemon uses `tracing_appender::rolling::daily()` for log rotation, which creates a new log file each day but does not limit the total size or number of retained files. In a long-running daemon with verbose logging, log files accumulate indefinitely in `~/.cloacina/logs/`.

Combined with OPS-05 (no circuit breaker during database outages), a sustained outage could produce hundreds of megabytes of error logs per day. The server has the same issue with its `cloacina-server.log`.

#### Evidence
```rust
let file_appender = rolling::daily(&logs_dir, "cloacina.log");
```

#### Suggested Resolution
Add `tracing_appender::rolling::RollingFileAppender` with max file count, or use a separate log rotation mechanism (e.g., `logrotate` on Linux). Document the expected log path and rotation behavior.

---

### OPS-10: SIGHUP config reload in daemon is well-designed
**Severity**: Observation (Positive)
**Location**: `crates/cloacinactl/src/commands/daemon.rs`, lines 302-327
**Confidence**: High

#### Description
The daemon handles SIGHUP for configuration hot-reloading, which is a strong operability pattern. On SIGHUP:
1. The config file is re-read
2. Watch directories are diffed against the current set
3. New directories are watched, removed directories are unwatched
4. A reconciliation is triggered to pick up packages in new directories

This allows operators to add new package watch directories without restarting the daemon, which is essential for a long-running local process.

---

### OPS-11: Security audit logging follows SIEM conventions
**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/security/audit.rs`
**Confidence**: High

#### Description
The `security/audit.rs` module provides structured audit logging with consistent event type constants (`package.load.success`, `key.signing.created`, `verification.failure`, etc.) and structured fields (`event_type`, `org_id`, `key_id`, `key_fingerprint`). These follow SIEM integration conventions and use appropriate log levels (info for success, warn for security-sensitive operations like revocations, error for failures).

This is a genuine operational asset -- security events can be filtered, alerted on, and forwarded to a SIEM system via the JSON log output.

---

### OPS-12: Database URL masking in serve.rs is a good practice
**Severity**: Observation (Positive)
**Location**: `crates/cloacinactl/src/commands/serve.rs`, lines 421-431
**Confidence**: High

#### Description
The `mask_db_url()` function correctly masks the password portion of database URLs before logging:
```rust
fn mask_db_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            return format!("{}****{}", &url[..colon_pos + 1], &url[at_pos..]);
        }
    }
    url.to_string()
}
```
This prevents credential leakage in server logs. The problem is that this function is not used in the Python bindings (see OPS-03), making it an inconsistently applied protection.

---

### OPS-13: Reactive scheduler supervision with circuit breaker is production-grade
**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/computation_graph/scheduler.rs`, lines 358-618
**Confidence**: High

#### Description
The `ReactiveScheduler::check_and_restart_failed()` method implements a mature supervision pattern:
- Exponential backoff (1s base, 60s max) for restarts
- Circuit breaker (5 max consecutive failures before permanent abandon)
- Failure counter reset after 60s of successful operation
- Individual accumulator restart without full graph teardown
- Reactor crash triggers a full graph restart with fresh channels
- Recovery events persisted to the database for audit
- Structured logging with graph name, component name, attempt count, and backoff duration

This is one of the most operationally mature subsystems in the codebase and could serve as a template for the scheduler loop's error handling.

---

### OPS-14: Graceful daemon shutdown with force-exit escape hatch
**Severity**: Observation (Positive)
**Location**: `crates/cloacinactl/src/commands/daemon.rs`, lines 331-354
**Confidence**: High

#### Description
The daemon shutdown sequence is well-designed for operational use:
1. First SIGINT/SIGTERM breaks the event loop
2. Runner shutdown is attempted with a configurable timeout (default 30s)
3. Timeout produces an error log and forces exit
4. A second SIGINT triggers immediate `process::exit(1)`

This three-tier approach (graceful drain, timeout, force) matches production best practices. The configurable timeout (`daemon.shutdown_timeout_s` in config.toml) allows operators to tune for their workload.
