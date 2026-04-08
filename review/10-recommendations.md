# Recommendations

## Overview

This document translates the findings from the architecture review into actionable recommendations, organized by urgency. Every Critical and Major finding has a corresponding recommendation. Related findings are grouped into single recommendations where fixing one root cause resolves multiple issues.

**How to read this:**
- **Immediate Actions**: Must fix before deploying the server to any environment accessible by untrusted users. These are security and correctness issues that could cause data loss, privilege escalation, or silent failures.
- **Short-Term Actions**: Fix in the next sprint or development cycle. These improve reliability, operability, and API quality.
- **Structural Improvements**: Larger efforts (days to weeks) that reduce long-term maintenance cost and improve the developer experience.
- **Architectural Recommendations**: Systemic changes to patterns and infrastructure that should be planned over multiple cycles.

**Effort scale**: Hours (< 1 day), Days (1-5 days), Weeks (1-4 weeks).

---

## Immediate Actions

### REC-01: Lock Down Server Authorization -- Close the Privilege Escalation Chain

**Addresses**: SEC-01 (Critical), SEC-04 (Major), SEC-02 (Critical)
**Severity**: Critical
**Effort**: Days (2-3 days)

#### What to do
1. In `create_key` (`keys.rs:50`), extract `Extension(auth): Extension<AuthenticatedKey>` and require `auth.can_admin()` before creating keys. Restrict non-admin users from creating keys with higher permissions than their own.
2. In `list_tenants` (`tenants.rs:122`), extract `Extension(auth)` and require `auth.is_admin` before returning the tenant list.
3. For tenant data isolation (SEC-02), add a per-request middleware or extractor that sets the PostgreSQL `search_path` to the authenticated tenant's schema before executing queries. This ensures DAL queries are scoped to the correct tenant.

#### Why it matters
These three findings together form a complete privilege escalation chain: authenticate with any key, call `POST /auth/keys` to mint an admin key, enumerate all tenants via `GET /tenants`, then access any tenant's data because queries are unfiltered. This is exploitable by any authenticated user in a multi-tenant deployment.

#### Suggested approach
- Start with SEC-01 and SEC-04 since they are the smallest changes (add auth extraction and checks, following the existing pattern in `create_tenant` and `revoke_key`).
- For SEC-02, create an axum middleware that resolves the tenant schema from the path parameter and executes `SET search_path TO <schema>` on the connection before the handler runs. The `Database::try_new_with_schema` method already supports schema-based isolation -- the gap is that the server uses a single shared `Database` instance and never switches schemas per-request.
- Add integration tests that verify: (a) a read-only key cannot create an admin key, (b) a tenant-scoped key cannot list other tenants, (c) a tenant-scoped key cannot query another tenant's executions.

---

### REC-02: Enforce Package Signature Verification in Server Mode

**Addresses**: SEC-03 (Critical), SEC-09 (Major), SEC-16 (Observation)
**Severity**: Critical
**Effort**: Days (1-2 days)

#### What to do
1. Change `cloacinactl serve` to default to `require_signatures: true` in `SecurityConfig`.
2. In the `upload_workflow` handler (`workflows.rs:82`), verify the package signature before passing to `register_workflow_package()`.
3. Remove or clearly label the heuristic security scan (SEC-16) as non-security -- it provides no real protection and creates a false sense of security.

#### Why it matters
Uploading a package results in `dlopen` of a compiled shared library, which executes arbitrary native code with the full privileges of the server process. Without mandatory signature verification, any authenticated user can execute arbitrary code on the server.

#### Suggested approach
- The signing and verification infrastructure already exists (`security/verification.rs`, `SecurityConfig`). The change is to wire it into the upload endpoint and flip the default.
- In the upload handler, call `verify_package_signature()` before `register_workflow_package()`. If verification fails, return 403 with a clear error.
- Keep the development/daemon defaults permissive (`require_signatures: false`) for local use.
- Add CLI flags `--require-signatures` (default true) and `--no-require-signatures` to `cloacinactl serve` for explicit control.

---

### REC-03: Fix Pipeline Completion Status to Reflect Task Failures

**Addresses**: COR-01 (Critical)
**Severity**: Critical
**Effort**: Hours (2-4 hours)

#### What to do
In `complete_pipeline()` (`task_scheduler/scheduler_loop.rs`), inspect the task statuses after `check_pipeline_completion()` returns true. If any task has `Failed` status, mark the pipeline as `"Failed"` instead of `"Completed"`.

#### Why it matters
Every consumer relying on pipeline status to detect failures -- API clients, cron retry logic, monitoring, the eventual metrics system -- will miss failures entirely. A pipeline where every task failed is currently indistinguishable from a fully successful one.

#### Suggested approach
```rust
// After check_pipeline_completion returns true:
let has_failures = task_statuses.iter().any(|s| s == "Failed");
if has_failures {
    self.dal.pipeline_execution().mark_failed(execution.id, "One or more tasks failed").await?;
} else {
    self.dal.pipeline_execution().mark_completed(execution.id).await?;
}
```
- The DAL already has `update_status` which accepts an arbitrary status string.
- Also consider a `PartiallyCompleted` or `CompletedWithFailures` status if some tasks succeeded and some failed.
- Add tests that verify: a pipeline with all tasks completed is marked "Completed", a pipeline with any failed task is marked "Failed", a pipeline with mixed completed/skipped/failed tasks is marked appropriately.

---

### REC-04: Remove Credential Logging from Python Bindings

**Addresses**: OPS-03 (Major)
**Severity**: Major
**Effort**: Hours (1-2 hours)

#### What to do
1. Remove all `eprintln!("THREAD: ...")` debug statements from `python/bindings/runner.rs`.
2. Apply `mask_db_url()` (or a shared equivalent) to database URLs before passing them to `info!()` calls.

#### Why it matters
The Python bindings log full database URLs including passwords to both structured logs and raw stderr. This bypasses the `mask_db_url()` protection that is correctly applied in `serve.rs`. Anyone with access to logs or terminal output can read database credentials.

#### Suggested approach
- The `mask_db_url()` function in `serve.rs` is already correct. Move it to a shared utility module (e.g., `logging.rs` or `database/mod.rs`) and use it in the Python bindings.
- Search `runner.rs` for all occurrences of `database_url` in log/print statements (at least lines 308-312 and 1150-1151). Replace with masked versions.
- The `eprintln!("THREAD: ...")` calls appear to be debug instrumentation that should have been removed before release.

---

### REC-05: Add Shutdown Channel to SchedulerLoop and Fix Server Shutdown

**Addresses**: COR-03 (Major), OPS-04 (Major), OPS-08 (Minor)
**Severity**: Major
**Effort**: Hours (3-4 hours)

#### What to do
1. Add a `watch::Receiver<bool>` shutdown channel to `SchedulerLoop`, following the established pattern in `StaleClaimSweeper::run()`.
2. Integrate it via `tokio::select!` in the `run()` loop body.
3. In `serve.rs`, call `runner.shutdown()` with a timeout in the graceful shutdown sequence (matching the daemon pattern).

#### Why it matters
Without a shutdown channel, `SchedulerLoop::run()` can only be stopped by aborting its task handle, which may interrupt database operations mid-transaction. The server compounds this by never calling `runner.shutdown()` at all -- background scheduler, executor, cron recovery, and stale claim sweeper are all terminated abruptly on SIGTERM.

#### Suggested approach
- Copy the shutdown pattern from `StaleClaimSweeper`:
  - Add `shutdown_rx: watch::Receiver<bool>` field to `SchedulerLoop`
  - In `run()`, replace `loop { interval.tick().await; ... }` with `loop { tokio::select! { _ = interval.tick() => { ... }, _ = self.shutdown_rx.changed() => { info!("SchedulerLoop shutting down"); break; } } }`
- In `serve.rs` shutdown sequence, after the reactive scheduler shuts down, call `runner.shutdown()` with a timeout (e.g., 30s), following the daemon's pattern in `daemon.rs:331-354`.

---

### REC-06: Make Task Completion Atomic

**Addresses**: COR-02 (Major)
**Severity**: Major
**Effort**: Hours (2-3 hours)

#### What to do
Combine `save_task_context` and `mark_task_completed` into a single database transaction in `complete_task_transaction` (`executor/thread_task_executor.rs:498-511`).

#### Why it matters
A crash between context save and status update leaves a task with persisted context but still in "Running" status. The stale claim sweeper will reset it to "Ready," causing re-execution with potential duplicate side effects.

#### Suggested approach
- The DAL already has examples of multi-table atomic transactions (e.g., `schedule_retry` in `task_execution`).
- Acquire a single connection, begin a transaction, save context, mark completed, and commit. On failure, the entire operation rolls back.
- The `dispatch_backend!` macro supports this pattern -- use `get_postgres_connection()` or `get_sqlite_connection()` once, then perform both operations within the same transaction closure.

---

## Short-Term Actions

### REC-07: Add TLS Support or Document Reverse Proxy Requirement

**Addresses**: SEC-06 (Major)
**Severity**: Major
**Effort**: Days (1-2 days for documentation; 3-5 days for native TLS)

#### What to do
Option A (minimum): Add documentation to the README and `cloacinactl serve --help` stating that a TLS-terminating reverse proxy (nginx, Caddy, Envoy) is required for production deployment. Add an example `docker-compose.yml` with a reverse proxy.

Option B (preferred): Add native TLS support via `axum-server` with `rustls`, with `--tls-cert` and `--tls-key` CLI options.

#### Why it matters
All API traffic -- including Bearer tokens, tenant credentials, WebSocket auth tokens, and workflow package uploads -- is transmitted in cleartext. Combined with SEC-05 (WebSocket token in query parameter), this exposes credentials across the full network path.

#### Suggested approach
- For Option A: Add a `## Production Deployment` section to documentation with reverse proxy configuration examples. Add a startup warning log when `--tls-cert` is not provided.
- For Option B: Add `axum-server` and `rustls-pemfile` dependencies. Modify the `serve` command to accept `--tls-cert` and `--tls-key` flags. Fall back to plain HTTP when flags are omitted, with a warning.

---

### REC-08: Add Rate Limiting to HTTP Endpoints

**Addresses**: SEC-07 (Major), SEC-13 (Minor)
**Severity**: Major
**Effort**: Days (1-2 days)

#### What to do
1. Add `tower::limit::RateLimitLayer` or `tower_governor` to the router in `serve.rs`.
2. Apply stricter limits to auth endpoints and upload endpoints.
3. Add `axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)` to the router to match the `PackageValidator` limit.
4. Add connection limits for WebSocket endpoints.

#### Why it matters
Without rate limiting, the system is vulnerable to auth brute-force attacks (the key cache miss path hits the database on every invalid key attempt), upload abuse (no body size limit, OOM via large uploads), execution flooding, and WebSocket connection exhaustion.

#### Suggested approach
- Use `tower_governor` for per-IP rate limiting with separate policies for: auth endpoints (strict, e.g., 10 req/s), upload endpoints (strict, e.g., 2 req/s), read endpoints (moderate, e.g., 100 req/s).
- Add `DefaultBodyLimit::max()` as a single line in the router builder.
- For WebSocket connection limits, add a `Semaphore`-based guard in the WebSocket handler (similar to the executor's concurrency limiter).

---

### REC-09: Standardize REST API Error Responses and Add Versioning

**Addresses**: API-02 (Major), API-03 (Major), API-13 (Minor)
**Severity**: Major
**Effort**: Days (2-3 days)

#### What to do
1. Define a standard `ApiError` struct with `error` (human-readable), `code` (machine-readable), `status` (HTTP code), and `request_id` fields.
2. Implement `IntoResponse` for `ApiError` and use it consistently across all handlers.
3. Add `/v1/` prefix to all tenant/workflow/execution/auth routes. Keep existing routes as aliases during a deprecation period.
4. Standardize status value casing (lowercase: `"completed"`, `"running"`, `"scheduled"`).

#### Why it matters
API clients cannot programmatically handle errors without string matching. The missing API versioning means breaking changes to the core API will break all clients with no migration path. The inconsistent status casing requires case-insensitive comparison in every client.

#### Suggested approach
- Start with the `ApiError` struct and a conversion function. Create it in a new `server/error.rs` module. Replace all inline `Json(serde_json::json!({"error": ...}))` calls with `ApiError::new(StatusCode::NOT_FOUND, "workflow_not_found", "Workflow not found")`.
- For versioning, nest all authenticated routes under `/v1/` in the axum router. The CG endpoints already use `/v1/` -- extend this to all routes.
- Generate a `request_id` (UUID) in a middleware layer and attach it to the response and all log spans.

---

### REC-10: Protect Tenant Credentials in API Responses

**Addresses**: SEC-08 (Major), SEC-05 (Major)
**Severity**: Major
**Effort**: Hours (3-4 hours)

#### What to do
1. In the `create_tenant` response (`tenants.rs:66-78`), do not embed the password in the `connection_string` field. Consider returning the password only once and requiring the caller to store it.
2. For WebSocket auth (SEC-05), implement a short-lived ticket exchange: the client calls a REST endpoint to exchange their API key for a single-use, time-limited ticket (e.g., 60s TTL), which is then used as the query parameter for WebSocket upgrade.

#### Why it matters
Tenant database passwords in HTTP responses traverse the full network stack (especially problematic without TLS) and may be logged by reverse proxies. WebSocket tokens in query parameters are logged by web servers, stored in browser history, and visible in Referer headers.

#### Suggested approach
- For SEC-08: Remove `password` and `connection_string` from the response, or encrypt them with a key provided by the caller. At minimum, stop embedding the password in the connection string.
- For SEC-05: Add a `POST /auth/ws-ticket` endpoint that returns `{"ticket": "<random>", "expires_at": "<timestamp>"}`. The WebSocket handler validates the ticket against a server-side store (e.g., an in-memory `DashMap` with TTL eviction) and consumes it on use.

---

### REC-11: Implement Metrics Export

**Addresses**: OPS-01 (Major)
**Severity**: Major
**Effort**: Days (3-5 days)

#### What to do
Replace the static `/metrics` endpoint with real Prometheus metrics. Instrument the critical paths with counters, histograms, and gauges.

#### Why it matters
Without metrics, operators cannot answer basic operational questions: How many workflows are running? What is task execution latency? How many tasks are failing? Is the connection pool saturated? The four golden signals (latency, traffic, errors, saturation) are completely unmeasured. COR-01 (pipeline always "Completed") would be invisible in production without metrics that track failure rates.

#### Suggested approach
- Add the `prometheus` or `metrics` crate with a `metrics-exporter-prometheus` backend.
- Instrument in priority order:
  1. **Counters**: pipelines_started, pipelines_completed, pipelines_failed, tasks_executed, tasks_failed, api_requests (by method, path, status)
  2. **Histograms**: pipeline_execution_duration_seconds, task_execution_duration_seconds, scheduler_loop_duration_seconds
  3. **Gauges**: active_pipelines, active_tasks, connection_pool_size, connection_pool_available, executor_semaphore_available
- The `ExecutorMetrics` struct already tracks `active_tasks`, `total_executed`, and `max_concurrent` internally -- connect these to the metrics export.

---

### REC-12: Add Circuit Breaker to Scheduler Loop

**Addresses**: OPS-05 (Minor)
**Severity**: Minor
**Effort**: Hours (2-3 hours)

#### What to do
Add a consecutive error counter with exponential backoff to the scheduler loop. After N consecutive errors, increase the poll interval up to a maximum (e.g., 30s). Reset to the normal interval on success.

#### Why it matters
During a sustained database outage, the scheduler loop logs an error on every 100ms tick, producing 600 error lines per minute with no backoff. This floods logs and obscures other diagnostic information.

#### Suggested approach
- Follow the `ReactiveScheduler::check_and_restart_failed()` pattern (OPS-13), which already implements exponential backoff (1s base, 60s max) and a circuit breaker (5 max failures).
- Add a `consecutive_errors: u32` field to `SchedulerLoop`. On error, increment and adjust the interval: `min(poll_interval * 2^consecutive_errors, Duration::from_secs(30))`. On success, reset to 0 and restore the original interval.
- Emit a rate-limited warning when the circuit opens.

---

### REC-13: Fix Lossy Error Conversion Across Crate Boundary

**Addresses**: COR-04 (Major), LEG-05 (Minor), EVO-04 (Minor)
**Severity**: Major
**Effort**: Hours (2-3 hours)

#### What to do
Add `Database(String)` and `ConnectionPool(String)` variants to `cloacina_workflow::ContextError`. Update the `From<ContextError> for TaskError` conversion to use the new variants instead of mapping infrastructure errors to `KeyNotFound`.

#### Why it matters
A database connectivity failure currently surfaces as "Key not found: Database error: connection refused" -- misleading for debugging and incorrect for retry logic that checks error types.

#### Suggested approach
- In `cloacina-workflow/src/error.rs`, add two new variants to `ContextError`: `Database(String)` and `ConnectionPool(String)`.
- In `cloacina/src/error.rs`, update the `From<ContextError> for TaskError` impl (lines 354-378) to map `ContextError::Database` to `cloacina_workflow::ContextError::Database` and `ContextError::ConnectionPool` to `cloacina_workflow::ContextError::ConnectionPool`.
- This is a cross-crate change but a small one (two new enum variants + updated match arms).

---

### REC-14: Add Configuration Validation

**Addresses**: OPS-07 (Minor), API-04 (Minor)
**Severity**: Minor
**Effort**: Hours (3-4 hours)

#### What to do
1. Add a `validate()` method to `DefaultRunnerConfig` that checks invariants and return `Result` from `build()`.
2. Replace freeform `String` fields with enums where possible (`StorageBackend`, `BackoffStrategy`, `KeyRole`).

#### Why it matters
Invalid configurations like `max_concurrent_tasks: 0` cause silent deadlocks, and typos in string-typed config fields silently fall through to defaults. A typo in `KeyRole` ("writ" instead of "write") creates a key with no permissions.

#### Suggested approach
- Validate at `build()` time: `max_concurrent_tasks > 0`, `stale_claim_threshold > heartbeat_interval`, `scheduler_poll_interval > 0`, `db_pool_size > 0`.
- Define `enum StorageBackend { Filesystem, Database }` and use it in `registry_storage_backend()`. Define `enum KeyRole { Admin, Write, Read }` and use it in `CreateKeyRequest`.
- For Python bindings, validate `retry_backoff` against valid values and return a `ValueError` for unrecognized strings.

---

### REC-15: Clean Up Error Type Duplicates and Legacy Variants

**Addresses**: LEG-04 (Minor), LEG-03 (Minor)
**Severity**: Minor
**Effort**: Hours (2-3 hours)

#### What to do
1. Remove `MissingDependencyOld` from `ValidationError` and migrate its 3 call sites to `MissingDependency`.
2. Choose either `CyclicDependency` or `CircularDependency` and remove the other. Update `WorkflowError::CyclicDependency` to use the same variant.
3. Remove or deprecate `backend_dispatch!` and `connection_match!` macros (each used only once) in favor of the dominant `dispatch_backend!` (132 uses).

#### Why it matters
Duplicate error variants force contributors to choose between near-identical options and make pattern matching more verbose. The three dispatch macros with overlapping names cause confusion for newcomers.

#### Suggested approach
- These are mechanical refactorings. Use find-and-replace for each variant name change. Run tests to verify no match arms are broken.
- For the dispatch macros: inline the single use of `backend_dispatch!` and `connection_match!` into their call sites, then delete the macro definitions.

---

## Structural Improvements

### REC-16: Rename Pipeline Types to Workflow and Clarify Scheduler Module Names

**Addresses**: LEG-01 (Major), LEG-02 (Major), API-01 (Major), API-06 (Minor)
**Severity**: Major
**Effort**: Weeks (1-2 weeks, phased)

#### What to do
**Part A -- Rename execution-layer types from "Pipeline" to "Workflow":**
- `PipelineExecutor` -> `WorkflowExecutor`
- `PipelineResult` -> `WorkflowResult`
- `PipelineStatus` -> `WorkflowStatus`
- `PipelineExecution` -> `WorkflowExecution`
- `PipelineError` -> `WorkflowExecutionError`
- Rename the existing `WorkflowError` -> `WorkflowBuildError` (construction errors)
- Update the prelude, Python bindings, and REST API response fields

Keep the database table names (`pipeline_executions`) for migration compatibility.

**Part B -- Clarify the scheduler module naming collision (LEG-02):**
- Rename `scheduler.rs` -> `cron_trigger_scheduler.rs` (or `schedule_manager.rs`)
- Rename `task_scheduler/` -> `execution_planner/` (or `readiness_manager/`)
- At minimum, add cross-referencing `//!` comments in each module explaining its relationship to the other

#### Why it matters
The terminology split between "Pipeline" (execution engine) and "Workflow" (user-facing concept) leaks through every consumer surface. Users searching docs for "workflow error" find `WorkflowError` (construction) when they want `PipelineError` (execution). API clients see `pipeline_name` in responses for something uploaded as a "workflow." This naming confusion also makes COR-01 (pipeline always "Completed") harder to spot, because readers must first untangle whether "Pipeline" and "Workflow" completion mean different things.

#### Suggested approach
- Phase 1: Add type aliases (`type WorkflowResult = PipelineResult`, etc.) and deprecation attributes on the old names. Update the prelude to export the new names.
- Phase 2: Migrate internal usages to the new names.
- Phase 3: Remove the deprecated aliases.
- This is a high-churn, low-risk refactoring. Use `replace_all` tooling and run the full test suite after each phase.

**Dependency**: Should be done after REC-03 (fix completion status), since changing the type names while the status logic is wrong would be confusing.

---

### REC-17: Add Distributed Tracing Infrastructure

**Addresses**: OPS-02 (Major)
**Severity**: Major
**Effort**: Days (3-5 days)

#### What to do
1. Add `tracing-opentelemetry` and an OTLP exporter as optional dependencies behind a feature flag.
2. Create a request-scoped span in an axum middleware layer with a generated request ID.
3. Propagate `pipeline_execution_id` through executor and task logs as a span field.
4. Accept `traceparent` headers for distributed trace propagation.

#### Why it matters
When an HTTP request triggers a pipeline execution, there is no way to correlate the HTTP request log with pipeline creation, task execution, or completion logs. In a multi-tenant server with concurrent executions, incident investigation requires correlating interleaved logs from scheduler, executor, and HTTP handler -- which is impossible without trace propagation.

#### Suggested approach
- The `tracing` crate is already used throughout, making OpenTelemetry integration straightforward.
- Start with the axum middleware: generate a UUID request ID, create a span, attach it to the request extensions. This gives immediate value for HTTP-level debugging.
- Next, pass the request ID through to `PipelineExecutor::execute()` and include it in all scheduler/executor log spans.
- The OTLP exporter can be configured via environment variables (`OTEL_EXPORTER_OTLP_ENDPOINT`), following OpenTelemetry conventions.

---

### REC-18: Add Production Dockerfile and Docker Compose

**Addresses**: OPS-06 (Minor)
**Severity**: Minor
**Effort**: Days (1-2 days)

#### What to do
1. Add a multi-stage production Dockerfile for `cloacinactl serve` (Rust builder + distroless/alpine runtime).
2. Add a `docker-compose.yml` that starts `cloacinactl serve` with PostgreSQL.
3. Document the production deployment path in the README.

#### Why it matters
The only Dockerfile in the repository is for CI test reproduction. Operators deploying Cloacina must build their own containerization from scratch, increasing misconfiguration risk.

#### Suggested approach
```dockerfile
# Stage 1: Build
FROM rust:1.77-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p cloacinactl

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cloacinactl /usr/local/bin/
EXPOSE 8080
ENTRYPOINT ["cloacinactl"]
CMD ["serve"]
```

---

### REC-19: Fix WorkflowBuilder Context Manager to Preserve Metadata

**Addresses**: API-08 (Minor)
**Severity**: Minor
**Effort**: Hours (1-2 hours)

#### What to do
In `python/workflow.rs`, update the `__exit__` method to carry the `description` and `tags` from `self.inner` to the new `Workflow` object created during auto-discovery.

#### Why it matters
Users who set `builder.description("ETL pipeline")` and `builder.tag("production")` inside the `with` block will find those values missing from the registered workflow. The manual `builder.build()` path preserves them, creating an inconsistency.

#### Suggested approach
- After creating the new `Workflow::new(workflow_id)` in `__exit__`, copy over `self.inner.description` and `self.inner.tags`.
- Add a Python test that verifies description and tags survive the context manager path.

---

### REC-20: Add Dependency Vulnerability Auditing to CI

**Addresses**: SEC-14 (Minor)
**Severity**: Minor
**Effort**: Hours (1-2 hours)

#### What to do
Add `cargo audit` to the CI pipeline, at minimum in the nightly workflow.

#### Why it matters
The project depends on security-critical crates (`ed25519-dalek`, `aes-gcm`, `sha2`, `rand`, `rdkafka`). Known vulnerabilities in dependencies will not be detected without automated scanning.

#### Suggested approach
- Add a step to `nightly.yml`: `cargo install cargo-audit && cargo audit`.
- Consider also adding `cargo deny` for license compliance auditing.
- Start as a non-blocking step (allow failures) to avoid false-positive CI breakage, then promote to blocking once the initial audit passes.

---

## Architectural Recommendations

### REC-21: Introduce Scoped Registries via a Runtime/Engine Struct

**Addresses**: EVO-02 (Major), EVO-10 (Observation), PERF-11 (Observation)
**Root cause**: RC-01 (Global Mutable State)
**Effort**: Weeks (2-4 weeks, phased)

#### What to do
Introduce a `Runtime` or `Engine` struct that owns registry instances (task registry, workflow registry, trigger registry, CG registry, stream backend registry). `DefaultRunner` would take a `Runtime` reference rather than reading from process-global static registries.

#### Why it matters
The process-global registries are the highest-leverage architectural issue:
- Tests require `#[serial]` (160 instances), making the test suite slow and sequential
- Cannot run multiple independent workflow environments in the same process
- `#[ctor]`-based registration is invisible -- tasks appear "by magic," creating debugging difficulties
- The global state prevents meaningful crate decomposition

#### Suggested approach
**Phase 1 (1 week)**: Create a `Runtime` struct that wraps `HashMap` registries for each type. Add a `Runtime::from_global()` factory that copies the current global registries into a scoped instance. `DefaultRunner::with_config()` accepts a `Runtime`. Existing code continues to work via `from_global()`.

**Phase 2 (1 week)**: Update tests to create per-test `Runtime` instances. Remove `#[serial]` from tests that no longer share global state. This immediately enables parallel test execution.

**Phase 3 (future)**: Introduce a `RuntimeBuilder` that replaces `#[ctor]` registration with explicit `runtime.register_task(...)` calls. The `#[ctor]` path remains as a convenience for the simple case.

---

### REC-22: Separate Python Bindings into a Distinct Crate

**Addresses**: EVO-01 (Major), EVO-05 (Minor), RC-04
**Effort**: Weeks (1-2 weeks)

#### What to do
Extract the Python bindings from `crates/cloacina/` into a new `crates/cloacina-python/` crate with crate-type `["cdylib"]`. The core `cloacina` crate becomes crate-type `["lib"]` only.

#### Why it matters
The dual crate-type `["lib", "cdylib"]` forces PyO3 compilation for all Rust consumers. The Python bindings are the largest module (2,888 lines in `runner.rs` alone) and require a separate Tokio runtime with message-passing coordination. Separating them eliminates the PyO3 dependency for Rust consumers and creates a cleaner boundary.

#### Suggested approach
- Move `src/python/` to a new `crates/cloacina-python/` crate.
- The new crate depends on `cloacina` (lib) and `pyo3`.
- The `extension-module` feature flag moves to the new crate.
- The maturin build targets the new crate.
- This is a mechanical restructuring with no behavior change.

---

### REC-23: Consolidate Dual-Backend DAL Duplication (Investigation)

**Addresses**: EVO-03 (Major), LEG-11 (Minor), LEG-03 (Minor)
**Root cause**: RC-03 (Dual-Backend Code Duplication)
**Effort**: Investigation (Days), Implementation (Weeks if feasible)

#### What to do
Investigate whether Diesel's `MultiConnection` or generic connection abstractions can be used to write DAL methods once rather than as `_postgres` / `_sqlite` pairs. If not feasible with current Diesel, apply mitigations:
1. Add explanatory comments at the top of each duplicated method pair (LEG-11 recommendation).
2. Remove `backend_dispatch!` and `connection_match!` macros, consolidating on `dispatch_backend!`.
3. Add a CI check that verifies both backend implementations produce equivalent results for a standard test suite.

#### Why it matters
Each DAL method exists as a near-identical pair, producing roughly doubled code in the DAL layer. Every schema change requires parallel modification. The cost is manageable for two backends but prohibitive for three. Even without adding a third backend, the duplication increases the risk of subtle behavioral drift between backends.

#### Suggested approach
- Start with the investigation: can `dispatch_backend!` be modified to call a single generic function that takes `&mut impl diesel::Connection`? The universal types (`DbUuid`, `DbTimestamp`) already bridge the type differences -- the remaining gap is the pool/connection acquisition.
- If the generic approach is feasible, pilot it on one simple DAL module (e.g., `checkpoint.rs`) to validate the pattern before applying broadly.
- If not feasible, apply the mitigation steps (comments, macro cleanup, CI checks) and document the architectural constraint.

---

## Summary Roadmap

The recommendations are sequenced to resolve dependencies and maximize early impact:

```
Phase 1: Security Foundation (Immediate, 1 week)
  REC-01: Lock down server authorization [SEC-01, SEC-02, SEC-04]
  REC-02: Enforce package signature verification [SEC-03, SEC-09]
  REC-03: Fix pipeline completion status [COR-01]
  REC-04: Remove credential logging [OPS-03]
      |
      v
Phase 2: Reliability (Short-term, 1-2 weeks)
  REC-05: Add shutdown channel to SchedulerLoop [COR-03, OPS-04, OPS-08]
  REC-06: Make task completion atomic [COR-02]
  REC-13: Fix lossy error conversion [COR-04, LEG-05, EVO-04]
  REC-12: Add circuit breaker to scheduler loop [OPS-05]
      |
      v
Phase 3: API Hardening (Short-term, 2-3 weeks)
  REC-07: Add TLS support or document reverse proxy [SEC-06]
  REC-08: Add rate limiting [SEC-07, SEC-13]
  REC-09: Standardize REST API errors and add versioning [API-02, API-03]
  REC-10: Protect tenant credentials [SEC-08, SEC-05]
      |
      v
Phase 4: Observability (Short-term, 1-2 weeks)
  REC-11: Implement metrics export [OPS-01]
  REC-17: Add distributed tracing [OPS-02]
      |  (metrics + tracing enable validating performance findings)
      v
Phase 5: Code Quality (Ongoing, parallel with Phase 3-4)
  REC-14: Add configuration validation [OPS-07, API-04]
  REC-15: Clean up error/macro duplicates [LEG-04, LEG-03]
  REC-19: Fix WorkflowBuilder context manager [API-08]
  REC-20: Add cargo audit to CI [SEC-14]
  REC-18: Add production Dockerfile [OPS-06]
      |
      v
Phase 6: Structural (Planned, 2-4 weeks)
  REC-16: Rename Pipeline -> Workflow types + clarify scheduler names [LEG-01, LEG-02, API-01, API-06]
      (depends on: REC-03 fixing completion status first)
  REC-22: Separate Python bindings crate [EVO-01, EVO-05]
      |
      v
Phase 7: Architectural (Planned, 3-6 weeks)
  REC-21: Scoped registries via Runtime struct [EVO-02]
      (highest leverage, highest cost -- enables parallel tests, crate decomposition)
  REC-23: Investigate DAL duplication consolidation [EVO-03]
      (investigation first, implementation only if feasible)
```

**Key dependencies:**
- REC-03 (fix completion status) should be done before REC-16 (rename Pipeline -> Workflow) to avoid renaming broken code
- REC-11 (metrics) and REC-17 (tracing) enable validation of performance findings (PERF-01 through PERF-11)
- REC-21 (scoped registries) unblocks REC-22 (crate decomposition) and enables parallel test execution
- REC-01 (authorization) is a prerequisite for meaningful multi-tenant deployment; all other server-mode improvements build on it
- Phase 1 items are independent of each other and can be done in parallel
