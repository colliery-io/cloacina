# Operability Review

## Summary

Cloacina's operability story is split: the `cloacinactl serve` mode is reasonably well-prepared for production with health/ready endpoints, Prometheus metrics, request-ID tracing, structured JSON log files, graceful shutdown with timeout, and optional OpenTelemetry support behind a feature flag. The `cloacinactl daemon` mode, however, has no health check surface at all, making it invisible to container orchestrators and monitoring systems. Across both modes, the metrics actually emitted by the core engine are limited to two counters (pipeline and task completion), leaving the four golden signals mostly uncovered -- no latency histograms, no saturation gauges, and no error-rate metrics are recorded despite being described in the Prometheus registry. Configuration is cleanly layered (CLI > env > config file) with sensible defaults, but values are not validated at startup, so invalid configurations silently produce surprising runtime behavior.

## Observability Assessment

**Logging**: Good. Both server and daemon modes produce dual-output logging: human-readable to stderr and structured JSON to rolling daily files under `~/.cloacina/logs/`. Database URLs are masked in logs. The `tracing` crate provides structured context throughout. The core scheduler loop has rate-limited error logging with circuit breaker backoff. However, log output from the core library lacks request-scoped correlation -- the request-ID middleware in the server does not propagate into the scheduler/executor spans.

**Metrics**: Partial. Seven Prometheus metrics are described (declared) in the server startup, but only two counters are actually emitted by the engine code: `cloacina_pipelines_total` (completed/failed) and `cloacina_tasks_total` (completed/failed). The four described-but-never-recorded metrics -- `cloacina_api_requests_total`, `cloacina_pipeline_duration_seconds`, `cloacina_task_duration_seconds`, `cloacina_active_pipelines`, `cloacina_active_tasks` -- will always show zero on the `/metrics` endpoint. An operator relying on these metrics for alerting would get no signal.

**Tracing**: Optional. Distributed tracing via OpenTelemetry OTLP is available behind the `telemetry` feature flag in `cloacinactl`. When `OTEL_EXPORTER_OTLP_ENDPOINT` is set, spans are exported via gRPC/Tonic. However, the core library itself creates only two `info_span!` calls (in the runner service startup), so trace data for the hot path (scheduler loop, task execution, context building) is absent. You cannot trace a workflow execution through the system using distributed tracing.

## Failure Mode Analysis

**Database unavailable**: The `/ready` endpoint correctly returns 503 when PostgreSQL is unreachable. The scheduler loop uses a circuit breaker with exponential backoff (capped at ~25s) after 5 consecutive errors, with rate-limited warning logs. Tasks classified as transient errors (timeout, connection, network) are retried per policy. The system degrades but does not crash.

**Crashed computation graph components**: The ReactiveScheduler has a supervision loop that detects crashed accumulator/reactor tasks and restarts them with exponential backoff and failure counting. Recovery events are persisted to the database for audit. The `/ready` endpoint incorporates crashed graph detection, returning 503 with the names of crashed graphs.

**Stale task claims**: The StaleClaimSweeper runs periodically (default 30s) and releases claims from crashed runners after a threshold (default 60s). It has a startup grace period equal to the stale threshold to avoid false positives. However, as noted in the correctness review, heartbeat ClaimLost does not cancel the running task (COR-005).

**Shutdown**: Both server and daemon modes handle SIGTERM and SIGINT gracefully. The daemon has a configurable shutdown timeout (default 30s) with a second-SIGINT force exit. The server uses a 30s hard-coded timeout for the runner shutdown. The daemon also handles SIGHUP for configuration reload without restart.

## Findings

## OPS-001: Daemon mode has no health check endpoint
**Severity**: Critical
**Location**: `crates/cloacinactl/src/commands/daemon.rs`
**Confidence**: High

### Description
The daemon mode (`cloacinactl daemon`) has zero health check surface. There is no HTTP listener, no Unix socket, no PID-based health file, and no status mechanism. When running in a container (Docker, Kubernetes), the orchestrator has no way to determine if the daemon is alive, healthy, or functioning correctly. The container health check can only rely on process existence, which does not detect deadlocked schedulers, database connectivity loss, or reconciliation failures.

### Evidence
The daemon's `run()` function at `daemon.rs:118` enters a `tokio::select!` loop handling filesystem events, periodic reconciliation, and signals. There is no HTTP listener or health reporting mechanism. The `docker-compose.production.yml` only defines a healthcheck for the PostgreSQL service, not for the cloacina service when running in daemon mode.

### Suggested Resolution
Add a minimal health reporting mechanism to the daemon. Options include:
1. A lightweight HTTP endpoint (e.g., on a configurable port) serving `/health`
2. A health status file written periodically (e.g., `~/.cloacina/health.json` with timestamp, last reconciliation time, active workflow count)
3. A Unix domain socket for health queries

Option 2 is the lowest-impact approach and can be checked via a Docker `HEALTHCHECK` command using `test -f` with a file age check.

---

## OPS-002: Metrics described but never recorded
**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 128-150, `crates/cloacina/src/execution_planner/scheduler_loop.rs` lines 334/344, `crates/cloacina/src/executor/thread_task_executor.rs` lines 399/427
**Confidence**: High

### Description
The server startup registers seven Prometheus metric descriptions, but only two counters are actually incremented anywhere in the codebase. The five phantom metrics create a false sense of observability: an operator looking at the `/metrics` endpoint sees metric names and descriptions suggesting comprehensive monitoring, but the values are always zero.

Metrics described but never recorded:
- `cloacina_api_requests_total` -- no request counting middleware exists
- `cloacina_pipeline_duration_seconds` -- no histogram recording at pipeline completion
- `cloacina_task_duration_seconds` -- no histogram recording at task completion
- `cloacina_active_pipelines` -- no gauge increment/decrement around pipeline lifecycle
- `cloacina_active_tasks` -- no gauge increment/decrement around task lifecycle

Metrics that ARE recorded:
- `cloacina_pipelines_total` with `status => "completed"` or `"failed"` (scheduler_loop.rs lines 334, 344)
- `cloacina_tasks_total` with `status => "completed"` or `"failed"` (thread_task_executor.rs lines 399, 427)

### Evidence
A search for `metrics::counter!`, `metrics::gauge!`, and `metrics::histogram!` across the entire `crates/cloacina/src` directory finds exactly four call sites, all using `counter!`. Zero `gauge!` or `histogram!` calls exist in the core engine.

### Suggested Resolution
Either remove the phantom metric descriptions (to avoid misleading operators) or implement the recording:
1. Add request-counting middleware to the axum router for `cloacina_api_requests_total`
2. Record `Instant::now()` at pipeline/task start and `histogram!(...).record(elapsed)` at completion for the duration metrics
3. Increment/decrement the active gauges in `schedule_workflow_execution` / `complete_pipeline` and in `execute()` / `complete_task_transaction`

---

## OPS-003: No configuration validation at startup
**Severity**: Major
**Location**: `crates/cloacina/src/runner/default_runner/config.rs` lines 259-293, `crates/cloacinactl/src/commands/config.rs` lines 90-119
**Confidence**: High

### Description
Neither the `DefaultRunnerConfig` builder nor the `CloacinaConfig` TOML loader validates configuration values at construction time. Invalid configurations are silently accepted and produce surprising runtime behavior:

- `max_concurrent_tasks: 0` would create a zero-permit semaphore, deadlocking all task execution
- `scheduler_poll_interval: Duration::from_millis(0)` would create a busy-loop consuming 100% CPU
- `stale_claim_threshold` less than `heartbeat_interval` would cause all claims to be immediately swept as stale
- `cron_max_catchup_executions: usize::MAX` (the default) can cause runaway execution storms after downtime (also flagged as PERF-004)
- `db_pool_size: 0` would create a pool with no connections

The TOML config loader at `config.rs:90` silently falls back to defaults on parse errors (which is correct for robustness), but this means a typo in a config key (e.g., `poll_intervla_ms`) produces no warning -- the intended setting is silently ignored.

### Evidence
The `DefaultRunnerConfigBuilder` at `config.rs:259-293` sets defaults but the `build()` method at line 557 simply returns `self.config` with no validation. Each setter method (lines 297+) directly assigns the value without bounds checking.

### Suggested Resolution
Add a `validate()` method to `DefaultRunnerConfig` that checks:
- `max_concurrent_tasks > 0`
- `scheduler_poll_interval >= 10ms`
- `stale_claim_threshold > heartbeat_interval`
- `cron_max_catchup_executions <= 1000` (or some reasonable upper bound)
- `db_pool_size > 0`
- `task_timeout > 0`

Call `validate()` from the builder's `build()` method, returning a `Result`. For the TOML config, consider using `#[serde(deny_unknown_fields)]` to catch typos.

---

## OPS-004: No distributed tracing through the execution hot path
**Severity**: Major
**Location**: `crates/cloacina/src/execution_planner/scheduler_loop.rs`, `crates/cloacina/src/executor/thread_task_executor.rs`, `crates/cloacina/src/runner/default_runner/services.rs` lines 43-51
**Confidence**: High

### Description
The core execution path -- from workflow submission through scheduling, dispatching, task execution, and pipeline completion -- has no tracing spans. The only `info_span!` calls in the runner create top-level spans for the "task_scheduler" and "executor" background tasks (services.rs lines 43-51), but individual workflow executions, task dispatches, and task executions are not traced.

When OpenTelemetry is enabled (via the `telemetry` feature flag), the server creates request-level spans via the `request_id_middleware`, but these spans do not propagate into the scheduler or executor because they run on separate tokio tasks with no span context propagation.

An operator cannot answer the question: "Why did workflow execution X take 45 seconds?" using trace data, because there are no spans for individual pipeline lifecycle events.

### Evidence
Searching for `info_span!`, `tracing::instrument`, or `#[instrument]` across `crates/cloacina/src/` yields only two results, both in `services.rs`. The scheduler loop, state manager, context manager, dispatcher, and executor have zero span creation or instrumentation.

### Suggested Resolution
Add spans at key lifecycle boundaries:
1. `schedule_workflow_execution`: span with `pipeline_execution_id`, `workflow_name`
2. `dispatch_ready_tasks` -> per-task: span with `task_execution_id`, `task_name`
3. `ThreadTaskExecutor::execute`: span with `task_execution_id`, `task_name`, `attempt`
4. `complete_pipeline`: span with `pipeline_execution_id`, `status`

Use `#[tracing::instrument]` on these methods or create explicit spans. This enables both local log correlation and distributed trace visualization.

---

## OPS-005: Docker image has no HEALTHCHECK or STOPSIGNAL
**Severity**: Minor
**Location**: `docker/Dockerfile` lines 1-37
**Confidence**: High

### Description
The production Dockerfile does not declare a `HEALTHCHECK` instruction or a `STOPSIGNAL`. Without `HEALTHCHECK`, Docker and orchestrators relying on Dockerfile-defined health checks will consider the container healthy as long as the process is running, even if the database is unreachable or the scheduler is deadlocked. Without `STOPSIGNAL`, Docker defaults to SIGTERM (which is actually correct for this application since it handles SIGTERM), but making it explicit is an operational best practice.

The docker-compose.production.yml defines a healthcheck for PostgreSQL but not for the cloacina service itself.

### Evidence
The Dockerfile at `docker/Dockerfile` contains no `HEALTHCHECK` or `STOPSIGNAL` directives. The compose file at `docker/docker-compose.production.yml` has `healthcheck` only on the `postgres` service (line 21), not on the `cloacina` service.

### Suggested Resolution
Add to the Dockerfile:
```dockerfile
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
  CMD ["curl", "-sf", "http://localhost:8080/health"] || exit 1
```
Note: this requires adding `curl` to the runtime image, or using a compiled health-check binary. Alternatively, use `wget` which is lighter.

Add to docker-compose.production.yml for the cloacina service:
```yaml
healthcheck:
  test: ["CMD", "curl", "-sf", "http://localhost:8080/health"]
  interval: 10s
  timeout: 5s
  retries: 3
```

---

## OPS-006: Request-ID does not propagate to background scheduler/executor
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 271-292 (request_id_middleware), `crates/cloacina/src/runner/default_runner/services.rs`
**Confidence**: High

### Description
The server creates a UUID request-ID for every incoming HTTP request and attaches it to a tracing span. However, when a request triggers a workflow execution (via `POST /tenants/{id}/workflows/{name}/execute`), the execution is handed off to the scheduler and executor on separate background tasks. The request-ID span is not propagated to these background tasks, so logs from the scheduler loop and executor that process this workflow cannot be correlated back to the originating HTTP request.

For an on-call engineer investigating "why did execution XYZ fail?", they would need to manually cross-reference the execution ID from the HTTP response with the scheduler/executor logs, which log `pipeline_execution_id` and `task_id` but not the originating request ID.

### Evidence
The `request_id_middleware` at serve.rs:271 creates an `info_span!("request", request_id = %id)`. The `execute_workflow` handler in `executions.rs` calls `runner.execute()` which schedules the workflow and returns an execution ID. The actual task execution happens asynchronously in the scheduler loop on a different tokio task, which has its own span ("task_scheduler") with no reference to the request ID.

### Suggested Resolution
Store the request ID (or a correlation ID) in the pipeline execution metadata or initial context. This way, when the scheduler and executor log events for this pipeline, the request ID is available as a structured field. Alternatively, include the `pipeline_execution_id` in the HTTP response (which is already done) and ensure all scheduler/executor logs include it (which they partially do).

---

## OPS-007: Database URL is logged at server startup without full masking context
**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/serve.rs` line 124, `crates/cloacina/src/logging.rs` lines 211-220
**Confidence**: High

### Description
The server correctly masks the password portion of the database URL before logging it (line 124: `mask_db_url(&database_url)`). However, the daemon mode at `daemon.rs:175` logs `info!("Database: {}", db_path.display())` which exposes the full SQLite path. While SQLite paths are less sensitive than PostgreSQL credentials, the masking function is only used in the server path.

More importantly, the `mask_db_url` function uses a simple heuristic (find `@`, then find the last `:` before it) which does not handle edge cases: URLs with port numbers in the password field, URLs with query parameters containing credentials, or connection strings that use non-standard formats.

The daemon's `cron_max_catchup` default at `config.rs:74` is `None` which maps to unlimited -- the same PERF-004 issue from the performance review surfaces here as an operational concern: after daemon downtime, cron jobs will stampede.

### Evidence
`logging.rs:211-220`: The `mask_db_url` function only masks between the last `:` before `@` and `@`. It does not handle `sslpassword=` or `?password=` query parameters. The daemon at `daemon.rs:175` does not use masking at all (though SQLite URLs typically have no credentials).

### Suggested Resolution
Apply `mask_db_url` consistently across both server and daemon paths. Consider using a URL-parsing approach (via the `url` crate, already a dependency) to identify and mask credential fields more robustly.

---

## OPS-008: No runbook documentation for common operational tasks
**Severity**: Minor
**Location**: Project-wide (absence)
**Confidence**: High

### Description
An on-call engineer would need to perform several operational tasks without any runbook guidance:

1. **How to drain a node**: No documentation on how to stop accepting new workflows while completing in-flight ones. The `shutdown()` method exists but the API has no "drain" endpoint.
2. **How to clean up old data**: `cloacinactl admin cleanup-events` exists but is not documented in the deployment guides. No guidance on retention policy sizing.
3. **How to rotate API keys**: Keys can be created and revoked via the API, but there is no documented rotation procedure.
4. **How to recover from a split-brain**: If two daemon instances watch the same directories with SQLite, the single-connection pool will serialize but there is no locking to prevent dual operation.
5. **How to debug a stuck workflow**: No query interface to find pipelines in specific states, no "force-complete" or "cancel" API endpoint.

### Evidence
The `docs/` directory contains user-facing guides for workflow development, computation graphs, and Python bindings, but no operations or runbook documentation. The `docker/docker-compose.production.yml` has a comment referencing `docs/content/how-to-guides/service/production-deployment.md` but this file would need to cover the scenarios above.

### Suggested Resolution
Create an operations guide covering: monitoring setup (which metrics to alert on), common failure scenarios and resolution steps, data retention policy, key rotation procedure, and troubleshooting stuck workflows. This would be the single most impactful improvement for on-call readiness.

---

## OPS-009: SIGHUP configuration reload is a strong operational feature (positive)
**Severity**: Observation
**Location**: `crates/cloacinactl/src/commands/daemon.rs` lines 302-327
**Confidence**: High

### Description
The daemon correctly handles SIGHUP for live configuration reload. When SIGHUP is received, it re-reads the config file, diffs the watch directories, applies changes to the filesystem watcher (adding new directories, removing old ones), and triggers a reconciliation. This allows operational changes (adding/removing watch directories) without restarting the daemon.

### Evidence
The SIGHUP handler at `daemon.rs:302-327` loads new config, computes directory diffs via `apply_watch_dir_changes`, and triggers reconciliation. The `apply_watch_dir_changes` function at lines 61-84 logs each directory add/remove.

### Suggested Resolution
No change needed. This is a well-implemented operational feature.

---

## OPS-010: Graceful shutdown with timeout and force-exit is well-implemented (positive)
**Severity**: Observation
**Location**: `crates/cloacinactl/src/commands/daemon.rs` lines 331-357, `crates/cloacinactl/src/commands/serve.rs` lines 231-258
**Confidence**: High

### Description
Both daemon and server modes implement graceful shutdown with appropriate timeout handling. The daemon uses a configurable timeout (default 30s from config) and supports force-exit on second SIGINT. The server uses a hard-coded 30s timeout with `tokio::time::timeout`. Both modes shut down subsystems in the correct order: reactive scheduler first, then workflow runner, then database pool.

### Evidence
Daemon at `daemon.rs:339-354`: `tokio::select!` races runner shutdown against timeout and second Ctrl+C. Server at `serve.rs:235-253`: Sequential shutdown of reactive scheduler, then workflow runner with timeout. The `DefaultRunner::shutdown()` at `mod.rs:304-341` sequentially awaits all service handles and closes the database pool.

### Suggested Resolution
Consider making the server shutdown timeout configurable (currently hard-coded to 30s). Otherwise, no changes needed.

---

## OPS-011: Bootstrap key written to filesystem with proper permissions (positive)
**Severity**: Observation
**Location**: `crates/cloacinactl/src/commands/serve.rs` lines 498-550
**Confidence**: High

### Description
The bootstrap admin key creation process follows security best practices: the plaintext key is never logged, is written to a file with mode 0600 (owner-only), and the function is clearly documented. The bootstrap only runs when no API keys exist, preventing accidental key overwriting.

### Evidence
Line 501: "The key is never logged." Line 540: `Permissions::from_mode(0o600)`. Line 510: Check `has_any_keys()` before creating.

### Suggested Resolution
No change needed.

---

## OPS-012: Server TLS warning is operationally appropriate (positive)
**Severity**: Observation
**Location**: `crates/cloacinactl/src/commands/serve.rs` line 126
**Confidence**: High

### Description
The server startup logs a clear warning: "Server running without TLS -- use a TLS-terminating reverse proxy (nginx, Caddy, Envoy) in production". This is the correct operational guidance for an application server that delegates TLS to a reverse proxy, and the warning ensures operators are aware.

### Evidence
Line 126: `warn!("Server running without TLS -- use a TLS-terminating reverse proxy (nginx, Caddy, Envoy) in production");`

### Suggested Resolution
No change needed.

---

## OPS-013: Error responses include machine-readable error codes
**Severity**: Observation
**Location**: `crates/cloacinactl/src/server/error.rs`
**Confidence**: High

### Description
All API error responses use a standardized `ApiError` type that includes both a human-readable `error` message and a machine-readable `code` field (e.g., `"unauthorized"`, `"key_not_found"`, `"rate_limited"`, `"internal_error"`). This enables automated error handling by clients and monitoring systems without parsing error message text.

### Evidence
`error.rs:84-88`: Response body includes `{"error": self.message, "code": self.code}`. Error constructors at lines 57-79 use descriptive codes like `"rate_limited"`, `"internal_error"`.

### Suggested Resolution
No change needed.
