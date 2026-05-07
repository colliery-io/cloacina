# Operability Review

## Summary

Cloacina has the *bones* of an operable production system — three out-of-process binaries with `--bind`, signal-driven shutdown, structured tracing, Prometheus metrics, daily-rotated JSON file logs, a credential-logging guard enforced as a CI check, a multi-stage Dockerfile, two reference docker-compose stacks, and a published TLS-via-reverse-proxy runbook. Inside the engine the fundamentals hold up: `mask_db_url` hides Postgres credentials at every log site, the scheduler-loop's `cloacina_active_workflows` gauge is SQL-derived per tick (the explicit fix for T-0534's gauge leak), and lifecycle ordering on the server's `with_graceful_shutdown` correctly drains the graph scheduler first, then races the runner shutdown against a 30s timeout.

The shape that makes these bones inadequate is the **asymmetry across the three deployables**. Only `cloacina-server` installs a Prometheus recorder and emits `/metrics`; `cloacina-compiler` and `cloacinactl daemon` emit zero metrics despite both being long-running services with claim-based queues that operators absolutely need to watch (build queue depth, sweeper resets, reconcile failures, package load failures). Only `cloacina-server` wires OpenTelemetry; trace IDs never propagate from `cloacinactl` → server → compiler. The server's request-id span is wired correctly but `tenant_id` is *never* added to that span, so tenant-scoped log filtering — table stakes for a multi-tenant orchestrator — has to grep through tenant ids carried in payload `info!` messages. The compiler service's HTTP `/v1/status` reports `pending`/`building`/`heartbeat_at` but no histogram of build durations, no counter of build failures, no `last_failure_reason` — operators still have to query the DB to debug.

The second problem is a **runbook gap**: `cloacinactl execution` has `list`, `status`, and `events` verbs but no `cancel`. There is no `package rebuild`, no `tenant drain`, no `workflow pause`. The engine has `WorkflowExecutor::cancel_execution` (`workflow_executor_impl.rs:231`) and the executor's cancel-channel plumbing supports cooperative + forced shutdown, but no CLI verb plumbs through to it. The third problem is in **failure-mode handling**: the active-tasks gauge uses `increment(1.0)` / `decrement(1.0)` (`thread_task_executor.rs:840, 904`) which is exactly the gauge-leak antipattern T-0534 fixed for `cloacina_active_workflows` — any panic between line 840 and 904 leaks the count permanently with no scrubbing tick. A handful of best-effort persistence sites (`let _ = persist_*`) and a multi-tenant search-path SET that fails silently (`COR-01`) round out the silent-data-loss surface. The system is well-behaved on graceful shutdown; it leaves no breadcrumbs when something less graceful happens.

## Observability Assessment

**Logging.** All three deployables (`cloacina-server`, `cloacina-compiler`, `cloacinactl daemon`) install a dual-layer `tracing-subscriber`: line-format to stderr, JSON to a daily-rotated file (`tracing-appender::rolling::daily`). Filter is `RUST_LOG` env var or `EnvFilter::new("info")` default; `--verbose` flips to `debug`. There's no built-in retention policy — logs grow until the disk fills (rolling::daily rotates by date but never expires). Credential masking is real and consistent (`crates/cloacina/src/logging.rs:195` + `scripts/check_credential_logging.py` enforced as `angreal lint credential-logging`). Server logs include a synthesized `request_id` span (`crates/cloacina-server/src/lib.rs:349-368`) that lands on every request and is also surfaced as the `X-Request-Id` response header — clean. The span's fields are `request_id`, `method`, `path` only; no `tenant_id`, no `key_name`, no `user_agent`. tenant_id is logged opportunistically inside route handlers via plain `info!("…tenant: {}", tenant_id)` (`routes/executions.rs:78, 94, 138, 146`) so JSON consumers can filter by it but only on a per-route basis. The daemon emits a `health_socket` and `health_pulse` task at 60s cadence that logs structured health (`SharedDaemonState`); compiler logs structured fields on build claim/heartbeat/result.

**Metrics.** Eight metrics are described and emitted, but only the *server* exposes a `/metrics` endpoint. The set is `cloacina_workflows_total{status,reason}`, `cloacina_tasks_total{status,reason}`, `cloacina_api_requests_total{method,status}`, `cloacina_api_request_duration_seconds`, `cloacina_workflow_duration_seconds`, `cloacina_task_duration_seconds`, `cloacina_active_workflows`, `cloacina_active_tasks`. The reasons are bounded (`task_error`, `timeout`, `validation_failed`, `infrastructure`, `task_not_found`, `claim_lost`, `unknown`) — good label hygiene. `angreal test:metrics-format` validates the exposition with `promtool check metrics` (`.angreal/test/metrics_format.py:130`). But:
- The `cloacina-compiler` binary emits zero metrics despite being the build worker — no build-queue depth, no build-duration histogram, no build-failure counter.
- The `cloacinactl daemon` emits zero metrics despite running the same scheduler stack — `cloacina_active_workflows` never reaches a Prometheus scraper because daemon mode never installs a recorder.
- `cloacina_active_tasks` uses naked `increment(1.0)` / `decrement(1.0)` (no SQL re-seed), reproducing the antipattern T-0534 fixed. A panic between increment and decrement leaks forever.
- No request-size histogram (uploaded package sizes are operationally interesting at 100 MB body limit).
- No saturation metric for the per-tenant Postgres pool, the auth `KeyCache`, or the build worker's tokio runtime.
- `tower-http`'s `trace` feature is enabled in `Cargo.toml` but `TraceLayer` is never applied — only the homegrown `request_id_middleware` is wired into the router (`crates/cloacina-server/src/lib.rs:489-490`).

**Tracing.** OpenTelemetry is wired *only* in the server, gated `feature = "telemetry"`, activated only when `OTEL_EXPORTER_OTLP_ENDPOINT` is set (`lib.rs:151-181`). There is no equivalent in `cloacina-compiler` or `cloacinactl`. There is no tracecontext header propagation: `cloacinactl` HTTP calls (the `CliClient`) do not inject a `traceparent`; the server's request-id span doesn't extract one either. Distributed traces from CLI → server → compiler (which is the canonical "package upload → build → load" flow) are simply not possible. Inside the server, a small number of `info_span!`s are created (`runner/default_runner/services.rs:47-55`); the engine itself uses scattered `span!`s but has no per-execution span pattern (`workflow_id`, `task_id`, `runner_id` are emitted as fields on individual events but not as span context).

**Health and readiness.** Server: `/health` returns `200 {"status":"ok"}` with no DB check (`crates/cloacina-server/src/lib.rs:521`); `/ready` checks the Postgres pool *and* the list of crashed CG graphs (`lib.rs:526-555`) and returns 503 with a structured payload if either fails — a good liveness/readiness split. The compiler service has its own `/health` (no DB check) and `/v1/status` (returns build queue stats from the DAL: `pending`, `building`, `last_success_at`, `last_failure_at`, `heartbeat_at` — `crates/cloacina-compiler/src/health.rs:62-77`); on DAL failure it returns 200 with `status: degraded` rather than non-2xx (probe-misleading). The daemon exposes a Unix-socket health probe at `~/.cloacina/daemon.sock` (`crates/cloacinactl/src/commands/health.rs`) carrying `DaemonHealth { status, pid, uptime_seconds, database, reconciler, active_workflows }` — clean for a single-host daemon, useless for orchestrators that expect HTTP.

## Failure Mode Analysis

Ten concrete failure paths and how they're handled:

1. **DB is reachable but `SET search_path` fails** (`COR-01`). The connection still returns; queries hit `public` instead of the tenant schema. No log, no metric, no alarm. Multi-tenant isolation breach. *Cross-cuts Security.*
2. **DB is unreachable on cold start.** Server: `DefaultRunner::with_config` returns Err and `cloacina-server` exits with the wrapped context (`Failed to connect to database`). Compiler: `Database::new` panics inside the `run_migrations` await (the construct doesn't return Result on `new`). Daemon: same as server. Reasonable. Restart loop expected via systemd / orchestrator.
3. **DB becomes unreachable mid-run.** The deadpool pools just start failing connection acquisition; the executor logs warns and the scheduler loop continues. Active claims remain held (no heartbeat completes), the stale-claim sweeper observes them stale, and on DB recovery the sweeper releases and re-dispatches. Reasonable, but no metric for "DB connection failures" — operators see only request 500s.
4. **Plugin load fails (cdylib bytes corrupt or missing symbols).** `RegistryReconciler.load_package` returns Err; the reconciler bookkeeping records `packages_failed` and the daemon logs `warn!("Package {} failed: {}", id, err)` (`commands/daemon.rs:106-108`). The package stays in pending DB state for the server path. No metric, no `last_failure_reason` field surfaced via `/v1/status`. The next reconcile tick retries forever — there is no exponential backoff per package and no quarantine.
5. **Fidius plugin panics inside execute_task FFI.** `cloacina_workflow_plugin::lib.rs:223` uses `.expect()` on `tokio::runtime::Builder::build` — if the cdylib's tokio init fails (resource limits exhausted), the cdylib `OnceLock` panics across the FFI boundary. fidius catches some panics; the macro wrapper does not catch all. `OnceLock` poisons on panic so subsequent calls hit the same path forever. Per `COR-04`. The host process can survive but the package is permanently broken until restart.
6. **Compiler queue backs up.** `claim_next_build` returns the next pending row. Cold-cache `cargo build` on a fresh runner is multi-minute; under sustained upload load the queue depth grows monotonically. The compiler has no concurrency knob (single build at a time per process), no horizontal scaling check, no metric for queue depth, and `/v1/status` returns the snapshot but nothing alerts on it. Operator must DB-query.
7. **Stale build claim** (compiler crashes mid-cargo). The sweeper resets stuck `building` rows past `stale_threshold_s`. Default `60s` (`crates/cloacina-compiler/src/config.rs`); the sweeper logs `info!(reset = n, "swept stale builds")`. No metric. Per `COR-16`, the sweep is also not claim-id-aware — the late builder can still clobber a successful re-builder's compiled bytes.
8. **Container OOMs mid-task execution.** The host kernel kills the process. In-flight tasks have heartbeats that go stale; on next runner restart the `StaleClaimSweeper` notices and resets them to `Ready`. `release_runner_claim` is unguarded (`COR-02`) so a returning runner can accidentally release a successor's claim. Recovery works but is asymmetric across the claim path.
9. **Scheduler starves under back-pressure.** `SchedulerLoop::dispatch_ready_tasks` calls `dispatcher.dispatch(event)` per ready task; on `DispatchError::NoCapacity` it logs `warn!` and the row stays Ready (`scheduler_loop.rs:271-277`). The tick fires again every 100ms, reads the same Ready rows, fans them out, hits NoCapacity again. CPU burn, no progress. No metric for "dispatch back-pressure" — operators see scheduler-tick CPU as the symptom, not the cause.
10. **SIGTERM during graceful shutdown.** Server: the runner shutdown is wrapped in `tokio::time::timeout(Duration::from_secs(30), runner_for_shutdown.shutdown())`. Daemon: explicit "Press Ctrl+C again to force exit" with `runner.shutdown()` raced against `tokio::time::sleep(shutdown_timeout)` and a second SIGINT handler that calls `std::process::exit(1)`. Compiler: `shutdown.cancel()` triggers the build loop and the HTTP task to exit; no timeout — a cargo build in flight just keeps running until it's done (the heartbeat keeps the claim warm). Three inconsistent shutdown behaviors.

## Findings

### OPS-01: `cloacina_active_tasks` gauge uses naked increment/decrement — exact antipattern T-0534 fixed for `cloacina_active_workflows`

**Severity**: Major
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs:840, 904`
**Confidence**: High

#### Description
The `cloacina_active_tasks` gauge is incremented at line 840 (`metrics::gauge!("cloacina_active_tasks").increment(1.0);`) immediately before task execution, and decremented at line 904 (`metrics::gauge!("cloacina_active_tasks").decrement(1.0);`) after the task body returns. Any panic, mid-function `return Ok(...)`, or task-cancellation path between those lines leaves the gauge incremented forever — exactly the leak T-0534 fixed for the workflow gauge. The fix for workflows was to **re-seed the gauge from SQL every scheduler tick** (`scheduler_loop.rs:166-168`: `metrics::gauge!("cloacina_active_workflows").set(active_executions.len() as f64);`). The same pattern would resolve the active-tasks leak — every tick, count `task_executions WHERE status = 'Running'` and `set` it. PERF reviewers also flagged that `update_workflow_task_readiness` already scans pending/running tasks per tick, so the count is already in hand.

#### Evidence
```rust
// thread_task_executor.rs:840
metrics::gauge!("cloacina_active_tasks").increment(1.0);
// ... 60+ lines of execution, including .await points and several conditional returns ...
// thread_task_executor.rs:904
metrics::gauge!("cloacina_active_tasks").decrement(1.0);
```
The dispatcher path's `claim_for_runner` returning `AlreadyClaimed` returns *before* line 840 (`thread_task_executor.rs:805-815`), but downstream paths after the increment include several `return Ok(ExecutionResult::failure(...))` arms (lines 824-836) — those return early without decrementing. Combined with the COR-04 cdylib-runtime panic path, the leak is reachable.

#### Suggested Resolution
Re-seed `cloacina_active_tasks` from SQL in the same tick as `cloacina_active_workflows`. Drop the increment/decrement entirely. Cost: one DAL count query per tick (cheap, indexed). Win: gauge is self-healing the way the workflow gauge already is.

**Cross-cutting note**: Performance also called out the SQL-derived gauge as a positive pattern — extending it to active_tasks is the same shape.

---

### OPS-02: `cloacinactl` exposes `execution list/status/events` but no `cancel`, `pause`, `resume`, or `rerun` — operators have to write SQL

**Severity**: Major
**Location**: `crates/cloacinactl/src/nouns/execution/mod.rs:34-57`; `crates/cloacina/src/executor/workflow_executor.rs:412` (the `cancel_execution` trait method exists)
**Confidence**: High

#### Description
The `execution` noun has three verbs: `List`, `Status`, `Events` (`mod.rs:36-57`). All read-only. The engine has `WorkflowExecutor::cancel_execution(&self, execution_id) -> Result<(), WorkflowExecutionError>` already implemented (`workflow_executor.rs:412`, called by the in-process `TaskHandle::cancel` at line 292). The server exposes no HTTP route for cancel either (`build_router` at `crates/cloacina-server/src/lib.rs:371-492` has no `DELETE /v1/tenants/.../executions/{id}` or `POST /v1/tenants/.../executions/{id}/cancel`). Same gap for "pause workflow" (the engine has paused-state support — see migration `007_add_pause_support`) but no CLI verb to trigger it. The same gap applies to `package` — there's no `cloacinactl package rebuild <id>` to force a re-build of a `failed` row; operators set `build_status='pending'` by hand. Common production ops (cancel a runaway run, drain a tenant before deletion, rebuild a failed package after fixing a Cargo dep) are DB ops, not CLI ops.

#### Evidence
- `crates/cloacinactl/src/nouns/execution/mod.rs:34-57` — `enum ExecutionVerb { List, Status, Events }`. No `Cancel`.
- `crates/cloacina/src/runner/default_runner/workflow_executor_impl.rs:231` — `async fn cancel_execution(&self, execution_id: Uuid)` is fully implemented.
- `crates/cloacina-server/src/lib.rs:371-446` — `auth_routes` has `executions::execute_workflow`, `list_executions`, `get_execution`, `get_execution_events`. No cancel/pause/rerun.
- The `--follow` flag on `events` returns `CliError::UserError("--follow streaming is tracked under spec Open Items; not in v1")` — even the read-side has stubs that explicitly defer.

#### Suggested Resolution
Add `cloacinactl execution cancel <id>` → `POST /v1/tenants/{t}/executions/{id}/cancel` → `WorkflowExecutor::cancel_execution`. Add `cloacinactl package rebuild <id>` → `UPDATE workflow_packages SET build_status='pending' WHERE id=$1` (gated by tenant + admin permission). Add `cloacinactl tenant drain <name>` to mark all running workflows for graceful cancel before delete. These are 1-2 day tasks each and they are exactly what an operator wants on incident day.

---

### OPS-03: Server's `request_id` tracing span has no `tenant_id`, `key_name`, or downstream context fields

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:349-368`
**Confidence**: High

#### Description
The `request_id_middleware` builds a span with only `request_id`, `method`, `path` (`lib.rs:355-360`). Auth middleware that follows produces an `AuthenticatedKey { tenant_id, name, role, ... }` (`crates/cloacina-server/src/routes/auth.rs:135-148`), but neither the auth middleware nor the route handlers propagate any of those fields back into the request span — they're only emitted in payload `info!("…tenant: {}", tenant_id)` calls scattered across route handlers (`routes/executions.rs:78, 94, 138, 146` etc.). For a tenant-isolated multi-tenant orchestrator, "all logs for tenant X" is a primary debugging operation; you need it during every incident. Today it requires grepping JSON log lines for `tenant: alpha` instead of a structured `tenant_id=alpha` filter on the span.

The same span gap means the OTLP tracing layer (when `OTEL_EXPORTER_OTLP_ENDPOINT` is set) emits spans without tenant_id either — distributed traces are not tenant-filterable. There's also no `traceparent` header extraction at the outer middleware — server-emitted spans are roots, never children of upstream cloacinactl traces.

#### Evidence
```rust
// crates/cloacina-server/src/lib.rs:355-360
let span = tracing::info_span!(
    "request",
    request_id = %id,
    method = %request.method(),
    path = %request.uri().path(),
);
```

`crates/cloacina-server/src/routes/auth.rs:175-194` (`require_auth` middleware) holds `AuthenticatedKey` but never calls `tracing::Span::current().record(...)`. None of the route handlers do either.

#### Suggested Resolution
Two changes: (a) After auth succeeds, record `tenant_id`, `key_id`, and `role` onto the current span (`tracing::Span::current().record("tenant_id", &tracing::field::display(tenant_id))`). The fields need to be declared on the span at creation (`tenant_id = tracing::field::Empty`) so `record` works. (b) Extract `traceparent` from the request via `opentelemetry-http`'s extractor and use `.with_context()` to make the request span a child of the upstream trace. Both are 30-line changes.

**Cross-cutting note**: Security review may want auth log lines to include the API key's `name` field for forensic auditing — same span fields are what they'd want.

---

### OPS-04: `cloacina-compiler` emits zero Prometheus metrics — the build queue is invisible to monitoring

**Severity**: Major
**Location**: `crates/cloacina-compiler/src/lib.rs:39-88`; `crates/cloacina-compiler/src/loopp.rs`; `crates/cloacina-compiler/src/health.rs:62-77`
**Confidence**: High

#### Description
The compiler service is a long-running, claim-based queue worker — exactly the kind of process that needs `queue_depth` / `claim_rate` / `build_duration_seconds` / `build_failures_total` / `sweep_resets_total` metrics. It exposes none. The `/v1/status` endpoint returns a single snapshot of `pending`, `building`, `last_success_at`, `last_failure_at`, `heartbeat_at` (`health.rs:62-77`) — usable for ad-hoc curl, useless for Prometheus scraping. To monitor queue growth, operators must either DB-poll `workflow_packages` directly or run an adapter that translates the JSON `/v1/status` into Prometheus format.

The compiler also has zero `metrics::counter!`/`histogram!` calls in source (verified by grep). Compare to the server, which describes 8 metrics at startup. The asymmetry is structural — `cloacina-compiler` doesn't pull `metrics` or `metrics-exporter-prometheus` into its dep graph at all.

#### Evidence
- `grep "metrics::"` in `crates/cloacina-compiler/`: zero hits.
- `crates/cloacina-compiler/src/health.rs:62-77` — JSON `/v1/status` body. No `/metrics` route.
- `crates/cloacina-compiler/Cargo.toml` does not list `metrics` or `metrics-exporter-prometheus`.

#### Suggested Resolution
Add a `/metrics` route to the compiler's HTTP server alongside `/v1/status`, using the same `metrics_exporter_prometheus::PrometheusBuilder` pattern as the server. Emit:
- `cloacina_compiler_builds_total{status=success|failed}` counter
- `cloacina_compiler_build_duration_seconds` histogram
- `cloacina_compiler_queue_depth{state=pending|building}` gauge (re-seeded SQL-derived per loop tick)
- `cloacina_compiler_sweep_resets_total` counter
- `cloacina_compiler_heartbeat_failures_total` counter

The current loop already has the data (claim, build duration measurable around `execute_build`, sweep result counts) — wiring is small. Pair with adding `cloacina-compiler` to `angreal test:metrics-format` so promtool validates the output.

---

### OPS-05: `cloacinactl daemon` emits zero Prometheus metrics; daemon-mode active workflows are unobservable

**Severity**: Major
**Location**: `crates/cloacinactl/src/commands/daemon.rs:121-401`; `crates/cloacinactl/src/commands/health.rs`
**Confidence**: High

#### Description
The daemon runs the same `DefaultRunner` stack as the server (scheduler loop, executor, stale-claim sweeper, registry reconciler) and so the `cloacina_active_workflows` / `cloacina_active_tasks` / `cloacina_workflows_total` / `cloacina_tasks_total` instrumentation actually fires inside the daemon process. But no Prometheus recorder is installed — the metrics go nowhere. Health observability for the daemon is limited to a Unix socket at `~/.cloacina/daemon.sock` returning a `DaemonHealth` JSON payload (active_workflows count, packages_loaded, last_reconciliation, uptime_seconds — `crates/cloacinactl/src/commands/health.rs:18-26`). That's queryable from the same host but not from a Prometheus scraper.

The daemon is the recommended single-host deploy path for SQLite-backed self-hosted users (`docs/content/platform/how-to-guides/running-the-daemon.md`). Self-hosted users absolutely will scrape Prometheus from their daemon.

#### Evidence
- `grep "install_recorder\|PrometheusBuilder" crates/cloacinactl/`: zero hits.
- `crates/cloacinactl/src/commands/daemon.rs` does not call `metrics_exporter_prometheus::PrometheusBuilder::new().install_recorder()`.
- `crates/cloacinactl/src/commands/health.rs:75-110` — `build_health` builds DaemonHealth on demand. No counters/histograms.

#### Suggested Resolution
Install a Prometheus recorder in `daemon::run` symmetric to the server's startup. Bind a small HTTP endpoint (e.g., `--metrics-bind 127.0.0.1:9091`) for `/metrics` separate from the Unix socket health. The active-workflows / active-tasks / workflow-duration / task-duration metrics will then flow naturally because the engine emit sites already exist.

---

### OPS-06: No log retention policy — `tracing-appender::rolling::daily` rotates by date but never expires old logs

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:141`; `crates/cloacina-compiler/src/lib.rs:95`; `crates/cloacinactl/src/commands/daemon.rs:142`
**Confidence**: High

#### Description
All three deployables call `rolling::daily(&logs_dir, "<service>.log")` to set up file logging. `tracing-appender`'s `Rotation::DAILY` rotates the file at midnight UTC and creates a new dated file (e.g., `cloacina-server.log.2026-05-05`); it does **not** delete old files. A long-running server accumulates a year of logs; on a small disk, this fills up. There is no `--log-retention-days` flag, no `max_log_files` parameter, no cron in the binary that prunes.

#### Evidence
- `crates/cloacina-server/src/lib.rs:141` — `rolling::daily(&logs_dir, "cloacina-server.log");`
- `tracing-appender` v0.2.3 docs: "RollingFileAppender does not delete old log files automatically." Confirmed against the appender API surface.
- No `find` / `tokio::time::interval` over logs_dir in any of the three binaries.

#### Suggested Resolution
Either (a) ship a documented logrotate config in `deploy/`, or (b) wire `tracing_appender::rolling::Builder::max_log_files(N)` (added in `tracing-appender` 0.2.3+) so the appender prunes by count. Add a `--log-retention-days` CLI flag wired to that. Default 14 days.

---

### OPS-07: Compiler service runs `cargo build` on user-uploaded source with zero sandboxing

**Severity**: Major
**Location**: `crates/cloacina-compiler/src/build.rs:132-184`
**Confidence**: High

#### Description
`cargo_build` in `cloacina-compiler` does `std::process::Command::new("cargo").args(&config.cargo_flags).current_dir(source_dir)` — no chroot, no Linux namespaces, no seccomp filter, no AppArmor profile, no Docker. The compiler service inherits the file system, network, and process privileges of whoever runs it. Cargo can run arbitrary build scripts (`build.rs`), proc-macros, and link-time hooks — all attacker-controlled in the uploaded `.cloacina` source archive. CARGO_TARGET_DIR sharing across packages also means one malicious package's build artifacts can poison the cache for legitimate packages.

The system overview Open Question 12 explicitly flagged this: "Sandboxing strategy (chroot, container, namespace, uid) was not visible." It still isn't.

#### Evidence
- `crates/cloacina-compiler/src/build.rs:135-152` — direct `Command::new("cargo")` with the source as `current_dir`. No `unshare`, no seccomp, no `landlock`.
- `crates/cloacina-compiler/Cargo.toml` does not depend on `landlock`, `seccompiler`, `tikv-jemallocator`, or any sandboxing crate.
- The `package signing` feature gates package *upload* (server enforces signature verification with `--require-signatures`), but signature verification only authenticates the publisher — it does not contain a malicious publisher.

#### Suggested Resolution
Pre-1.0: ship the `deploy/docker-compose/cloacina.yml` reference such that the compiler runs in its own minimal container with `--cap-drop=ALL`, `--security-opt=no-new-privileges`, `--read-only` filesystem except `CARGO_TARGET_DIR` and a tmpfs for the source — and document this in `production-deployment.md`. Post-1.0: integrate `landlock` (Linux) or per-build container spawn. Worth a dedicated initiative, not a single-task fix.

**Cross-cutting note**: Security review should treat this as a hard finding. The risk shape is "RCE on the compiler host via package upload."

---

### OPS-08: Compiler `/v1/status` returns 200 with `status=degraded` on DAL failure — probes can't distinguish healthy from broken

**Severity**: Minor
**Location**: `crates/cloacina-compiler/src/health.rs:62-77`
**Confidence**: High

#### Description
The compiler's `/v1/status` handler does `match registry.build_queue_stats().await { Ok(stats) => Json(...), Err(e) => Json(serde_json::json!({"status":"degraded","error":...})) }`. Both arms return HTTP 200. A Kubernetes readiness probe configured against `/v1/status` cannot tell "queue depth fine" from "DB unreachable, can't even count rows." The `/health` route is similarly unconditional (always returns `{"status":"ok"}`). Kubernetes / Nomad operators expect 5xx on degradation so the orchestrator can mark the pod NotReady.

#### Evidence
```rust
// crates/cloacina-compiler/src/health.rs:62-77
async fn status(State(registry): State<Registry>) -> Json<serde_json::Value> {
    match registry.build_queue_stats().await {
        Ok(stats) => Json(serde_json::json!({ "status": "ok", ... })),
        Err(e) => Json(serde_json::json!({
            "status": "degraded",
            "error": format!("{}", e),
        })),
    }
}
```
No `StatusCode::SERVICE_UNAVAILABLE` import; the function returns `Json<...>`, not `impl IntoResponse` with a status code.

#### Suggested Resolution
Make `/health` a true liveness probe (always 200 if the process is alive — fine), and add `/ready` returning 200 on healthy stats and 503 on DAL error, mirroring the server's `/health` vs `/ready` split. Update the docker-compose health check (`docker-compose.production.yml:21-25`) to point at `/ready`.

---

### OPS-09: Daemon and server runners hardcode SQLite pool size 1 and per-tenant Postgres pool size 2; both ignore configuration

**Severity**: Major
**Location**: `crates/cloacina/src/database/connection/mod.rs:239-245` (SQLite); `crates/cloacina-server/src/lib.rs:77-82` (per-tenant)
**Confidence**: High

#### Description
Per `PERF-01` and `PERF-02`, the SQLite path silently overrides `db_pool_size` to a hardcoded `1`, and the server's per-tenant cache hardcodes `2` connections per tenant. The reason this is an *operability* finding (in addition to a performance one): operators cannot tune their way out. There is no `--tenant-pool-size` CLI flag, no `[server.tenant_pool]` config block, no warning in logs that "your `db_pool_size = 32` is being ignored on SQLite." On a runaway-tenant incident, the operator sees 504s and queue growth, finds the per-tenant config knob in the docs, sets it, restarts — and nothing changes.

#### Evidence
- `crates/cloacina-server/src/lib.rs:77-82` — `Database::try_new_with_schema(&self.database_url, "cloacina", 2, Some(tenant_id))`. Comment "small pool per tenant" is the only signal.
- `crates/cloacina/src/database/connection/mod.rs:239-245` — `let sqlite_pool_size = 1;` overrides whatever the caller passed.
- `cloacinactl serve --help` and `cloacinactl daemon start --help` have no pool-size flag.

#### Suggested Resolution
Plumb `tenant_pool_size: usize` through `cloacina-server` CLI args → AppState → `TenantDatabaseCache::resolve`. Default 8. For SQLite: if the caller explicitly opts in to pool > 1, keep it (WAL mode supports concurrent readers); if not, default to 4 readers + serialize writes (deadpool already does this). Either way, document the override in the rustdoc and refuse to silently swap caller intent.

---

### OPS-10: Compiler shutdown does not bound running cargo builds — SIGTERM waits indefinitely

**Severity**: Minor
**Location**: `crates/cloacina-compiler/src/lib.rs:79-87`; `crates/cloacina-compiler/src/loopp.rs:94-125`
**Confidence**: High

#### Description
On SIGINT, the compiler's `tokio::signal::ctrl_c()` handler calls `signal_shutdown.cancel()`. The build loop's `tokio::select!` (`loopp.rs:94-125`) sees `shutdown.cancelled()` and returns from the polling loop — but it has no awareness of the in-flight `run_build_with_heartbeat` call. If a `cargo build` is running when SIGTERM lands, the loop returns to `lib.rs:79`, then awaits the HTTP task handle (`http_handle.await` on line 82), which itself `with_graceful_shutdown` exits cleanly. Meanwhile, the `Command::new("cargo")` child is still running because cargo doesn't know about the shutdown. The shutdown then awaits *forever* on the cargo subprocess — there's no timeout, no `Command::kill`, no SIGTERM forwarded to the child cargo.

The server bounds the `runner.shutdown()` with a 30s `tokio::time::timeout`. The daemon races shutdown vs second SIGINT vs configurable timeout. The compiler does not.

#### Evidence
- `crates/cloacina-compiler/src/lib.rs:79`: `loopp::run(registry, config, shutdown.clone()).await?;` is awaited *before* `shutdown.cancel()` on line 81. The cancel is post-loop, but the loop only exits when its `tokio::select!` sees the cancel — and it can be blocked on the inner build's `await`.
- `crates/cloacina-compiler/src/loopp.rs:100-115` — the `claim_next_build` arm calls `run_build_with_heartbeat(...).await` synchronously inside the select arm. There is no `tokio::select!` around the build itself for shutdown observability.
- `run_build_with_heartbeat` has a heartbeat-cancel token but no build-cancel — the cargo subprocess runs to completion regardless.

#### Suggested Resolution
Wrap the cargo invocation in a select against the shutdown token: spawn cargo with `tokio::process::Command` (already an option), then `tokio::select! { res = child.wait() => …, _ = shutdown.cancelled() => { child.kill().await.ok(); … } }`. Add a `--shutdown-timeout-s` flag mirroring the daemon. On hard timeout, log a warn and abandon — the row stays `building`, the sweeper resets it after `stale_threshold_s`.

---

### OPS-11: `RegistryReconciler` retries failed packages forever with no quarantine — a single bad package floods logs every poll tick

**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/daemon.rs:296-323`; `crates/cloacina/src/registry/reconciler/loading.rs`
**Confidence**: Medium

#### Description
When the daemon's reconciler encounters a package that fails to load (corrupt cdylib, missing trigger metadata, etc.), `handle_reconcile` logs `warn!("Package {} failed: {}", id, err)` once per failure (`commands/daemon.rs:106-108`) and the next reconcile tick re-attempts the same load, fails the same way, logs the same warn. There's no per-package failure counter, no exponential backoff, no quarantine after N failures. A single bad `.cloacina` file in a watch directory will produce a `warn!` every `poll_interval_ms` (default likely 1-2s) until manually removed. JSON log file fills up.

For the server path, the failure is contained to the single reconcile tick — the row stays in DB. But the same problem applies: every reconcile poll re-attempts. The reconciler's `enable_startup_reconciliation: false` flag in the daemon (`commands/daemon.rs:228`) suggests the team has thought about reconcile timing, but quarantine isn't implemented.

#### Evidence
- `crates/cloacinactl/src/commands/daemon.rs:106-108`:
```rust
if result.has_failures() {
    for (id, err) in &result.packages_failed {
        warn!("Package {} failed: {}", id, err);
    }
}
```
- The `ReconcileResult` struct does not carry a per-package `failure_count` or `last_failed_at`.
- No `package_failures_total{package, reason}` metric.

#### Suggested Resolution
Add an in-memory `HashMap<PackageId, PackageFailureState { count, last_failed_at, last_reason }>` on `RegistryReconciler`. Skip retry for `T-now < last_failed + backoff(count)`, log `info!` (not `warn`) on the skip, and emit a `cloacina_reconciler_package_failures_total{package, reason}` counter. Quarantine after 5 consecutive failures with a clear "package X is quarantined; remove and re-add to retry" log.

---

### OPS-12: Multi-tenant `tenant_id` never reaches the request span — log filtering for incident triage requires payload string matching

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:355-360`; `crates/cloacina-server/src/routes/auth.rs:175-194`
**Confidence**: High

#### Description
This is a sub-finding of OPS-03 worth its own line because the operability cost is concrete: when an operator's tenant `acme` reports a stuck workflow, the operator's only filter is `grep '"tenant_id":"acme"'` against JSON logs — but JSON logs only carry `tenant_id` in payload messages where some `info!("…tenant: {}", tenant_id)` happened to fire. Auth failures, scheduler-tick logs, registry reconciler logs, executor task lifecycle logs — none of them carry `tenant_id` because tenant_id is bound to the request, not the runner state, and the runner state is shared across tenants per the architectural mismatch in EVO-04 / Open Question 11.

#### Evidence
- `crates/cloacina-server/src/lib.rs:355-360` — the request span carries `request_id`, `method`, `path`. Nothing else.
- Auth middleware sets `AuthenticatedKey { tenant_id, ... }` but does not call `Span::current().record(...)`.
- The scheduler loop and executor are shared across tenants — they have no `tenant_id` to log even if they wanted to.

#### Suggested Resolution
(a) Span enrichment: declare `tenant_id = tracing::field::Empty, key_id = tracing::field::Empty, role = tracing::field::Empty` on the request span at creation, then `Span::current().record(...)` after auth. (b) For the scheduler/executor side (where there's no request context), structurally tag emitted log fields with `tenant_id` extracted from the `workflow_executions` row at scheduler-tick boundaries. The latter is a bigger lift and depends on resolving the multi-tenant execution scoping gap (EVO-04). (a) alone resolves the API-side filtering need.

---

### OPS-13: Server `/health` does not check DB connectivity — liveness-vs-readiness split exists but liveness is too permissive

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:521-555`
**Confidence**: High

#### Description
The split is correct in shape: `/health` is liveness (process alive, no DB check), `/ready` is readiness (DB pool + crashed-graphs check). But the published runbook (`docs/content/platform/how-to-guides/production-deployment.md:96-97`) says "Liveness: GET /health — returns 200 if the process is alive". A liveness probe that returns 200 without checking *any* dependency means a process stuck on an unkillable lock or starved I/O loop will never be replaced by an orchestrator — k8s / Nomad will keep failing readiness, never restarting. The conventional mitigation is to make liveness fail on "process can't make progress" (e.g., scheduler-tick hasn't fired in 5 minutes), not just "main thread alive". Today, `/health` is strictly an "I'm running" check.

#### Evidence
```rust
// crates/cloacina-server/src/lib.rs:521-523
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}
```
No state extraction, no last-tick timestamp check.

#### Suggested Resolution
Add a `SchedulerLastTick` shared state (an `AtomicI64` timestamp) updated on every scheduler tick. `/health` returns 503 if `now - last_tick > N seconds` (configurable, default 30s). The server's existing `MissedTickBehavior::Skip` cron means the timestamp will move at the configured interval; lack of movement signals real failure. Document the new contract in `production-deployment.md`.

---

### OPS-14: No correlation ID propagates through the cdylib FFI boundary — packaged-task logs are detached from runner logs

**Severity**: Minor
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:208-280` (the `execute_task` macro body); `crates/cloacina/src/registry/reconciler/loading.rs`
**Confidence**: High

#### Description
When a packaged cdylib's `execute_task` is called from the host, the host has a full tracing span context (`request_id`, `task_id`, `workflow_id`, etc.), but the cdylib does not — the FFI boundary does not propagate `tracing::Span` because each cdylib has its own tokio runtime and its own (uninitialized) tracing subscriber. Logs emitted from inside the user's task body via `tracing::info!` go to the *cdylib's* (default, uninitialized) subscriber, which is a no-op writer. `println!` from inside user task code reaches stdout, but tracing macros silently disappear.

The `cdylib_runtime` lazy-init in `lib.rs:213-225` constructs the runtime but never calls `tracing_subscriber::registry().init()` inside the cdylib. Even if it did, the host and cdylib have separate global subscribers, so spans don't share context.

#### Evidence
- `crates/cloacina-workflow-plugin/src/lib.rs:213-225` — runtime construction body. No `tracing_subscriber` setup.
- The host's `request_id` span field is a heap allocation owned by the host's subscriber registry; the cdylib has no access to it.
- No `task_execution_id` or `workflow_execution_id` is passed into `execute_task` as a wire-format field — the cdylib doesn't know its own task_id.

#### Suggested Resolution
This is a deep refactor. Lightweight first step: add `task_execution_id: Uuid` to the `TaskInvocation` wire-format struct (`crates/cloacina-workflow-plugin/src/types.rs`) so the cdylib can at least *log* the id even if it can't join the host's span. Document that user task code should rely on `tracing-subscriber` attached at cdylib init time and write JSON logs to a known path (or to stderr, which the host captures). Long-term: a tracing-context propagation envelope (W3C `traceparent`) passed across FFI.

---

### OPS-15: Best-effort persistence (`let _ = persist_*`) in CG paths silently drops crash-recovery state without metrics

**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/accumulator.rs:428, 515, 532, 636, 647, 692`; `crates/cloacina/src/computation_graph/reactor.rs:670-714`
**Confidence**: High

#### Description
The CG persistence layer follows a "best-effort" pattern: when the DB write fails, log and continue (per `cloacina-overview` §9 error-handling idioms — "many CG persistence calls are wrapped in `let _ =` and log on failure"). The justification is correctness: a CG fire that already produced its in-memory output shouldn't fail just because the checkpoint write didn't persist. But operationally, this means an operator has no way to tell that a reactor's state isn't actually being checkpointed. On crash recovery, the reactor will rehydrate from the *last successful* checkpoint, which could be hours behind reality if every checkpoint since has silently failed.

There's no `cloacina_reactor_persist_failures_total` counter, no log-level escalation on N consecutive failures, no health degradation. The accumulator's `AccumulatorHealth::SocketOnly` state (line 390) hints that "persistence failed, only socket forwarding works" is a known degradation mode, but the visibility surface ends at that single watch channel.

#### Evidence
- `crates/cloacina/src/computation_graph/accumulator.rs:702-709`:
```rust
fn set_health(ctx: &AccumulatorContext, health: AccumulatorHealth) {
    let _ = ctx.health_tx.send(health);
}
```
The `let _ =` is the operability-bypass pattern.
- The reactor's `persist_reactor_state` (`reactor.rs:670-714`) also returns silently on serde or DB error.

#### Suggested Resolution
Add metrics:
- `cloacina_reactor_persist_failures_total{reactor, kind=cache|dirty|seq_queue|checkpoint}`
- `cloacina_accumulator_persist_failures_total{accumulator, kind=boundary|buffer|health}`

Add a watchdog: if a reactor logs 5 consecutive persist failures, downgrade `ReactorHealth::Live` → `ReactorHealth::Degraded` and surface it via `/v1/health/graphs/{name}`. Operators can then alert on the health change.

---

### OPS-16: No structured `tenant_id` / `runner_id` correlation for execution events emitted to the database

**Severity**: Minor
**Location**: `crates/cloacina/src/dal/unified/execution_event.rs`; `crates/cloacina/src/execution_planner/scheduler_loop.rs:340-373`
**Confidence**: High

#### Description
Every state transition writes to `execution_events`. The table has columns sufficient to reconstruct a workflow's history (`workflow_execution_id`, `task_execution_id`, `event_type`, `created_at`), but no `runner_id` (which runner observed the transition), no `tenant_id` (so cross-tenant aggregations have to join `workflow_executions`), and no `request_id` (so an API-driven execution can't be traced from its HTTP request through to its terminal event). The `request_id` middleware on the server generates one, but it's not threaded into the execution-creation path.

This is fine for steady-state debugging but bad for incident forensics. "Why did this workflow fail?" — operator queries `execution_events`, sees a `TaskFailed` event, has no linkage back to which API key dispatched the workflow, which runner instance observed the failure, or which API request landed it.

#### Evidence
- `crates/cloacina/src/database/migrations/postgres/...execution_events...` tables: schema columns are `id, workflow_execution_id, task_execution_id, event_type, payload, created_at`.
- `scheduler_loop.rs:340-355` — emits counters but the DB event row carries no correlation id.

#### Suggested Resolution
Add `request_id TEXT NULL`, `runner_id UUID NULL`, `tenant_id TEXT NULL` columns on `execution_events`. Backfill from the request span at workflow-execution creation; set `runner_id` from the executor's `instance_id` at task-state-transition write time. Cost: a migration on each backend, plus 5-10 lines plumbing per emit site.

---

### OPS-17: `cloacinactl status` is a serial composite of three independent network calls; no aggregation or timeout

**Severity**: Minor
**Location**: `crates/cloacinactl/src/nouns/mod.rs:33-58`
**Confidence**: High

#### Description
The composite `cloacinactl status` does:
```rust
println!("=== daemon ==="); daemon::status::run(globals).await
println!("=== server ==="); server::status::run(globals).await
println!("=== compiler ==="); compiler::status::run(globals).await
```
Each call is a separate network round-trip with its own implicit timeout. If the server is unreachable, the user waits the full TCP/HTTP default (30s) before seeing the daemon and compiler results. Worse, the prints happen interleaved with the calls — the user sees `=== server ===`, then 30s of nothing, then `=== compiler ===`. There's no concurrent fan-out, no aggregate timeout, no JSON output mode for scripts, no exit code that reflects degraded state.

#### Evidence
```rust
// crates/cloacinactl/src/nouns/mod.rs:38-56
pub async fn top_level_status(globals: &GlobalOpts) -> Result<()> {
    println!("=== daemon ==="); /* await call */
    println!();
    println!("=== server ==="); /* await call */
    println!();
    println!("=== compiler ==="); /* await call */
    Ok(())
}
```

#### Suggested Resolution
`tokio::join!` the three status calls, emit results in a stable order after all complete, add `--json` output that returns `{daemon: {...}, server: {...}, compiler: {...}}` for monitoring scripts, exit non-zero if any of the three is unreachable. Wrap each call in a `tokio::time::timeout(10s, …)`.

---

### OPS-18: Server's request body limit is hardcoded at 100 MB; no per-route override or configurability

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:486`
**Confidence**: High

#### Description
`DefaultBodyLimit::max(100 * 1024 * 1024)` is the global request body cap. Workflow upload routes need this size; auth/key/tenant management routes do not. Anyone can post a 99 MB body to `/v1/auth/keys` and the server will buffer it before parsing fails — easy DoS shape. There's also no CLI flag to tune the limit; if a customer's `.cloacina` package legitimately exceeds 100 MB (large vendored deps), the operator has to recompile the server.

#### Evidence
- `crates/cloacina-server/src/lib.rs:486` — `.layer(DefaultBodyLimit::max(100 * 1024 * 1024))`.
- No `--max-body-size` flag; no per-route `RequestBodyLimitLayer::new(...)` override.

#### Suggested Resolution
Set the global default smaller (1 MB), apply `RequestBodyLimitLayer::new(100 * 1024 * 1024)` only on the workflow-upload route. Add `--max-package-size-mb` CLI flag.

---

### OPS-19: No rate limiter — per-tenant or global — despite project memory citing it as a soak gap

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:475-491` (router definition); whole `cloacina-server` crate
**Confidence**: High

#### Description
Project memory `project_soak_test_gaps.md` lists "rate limiter" as one of the five gaps revealed by the server soak. A grep of `cloacina-server/src/` for `RateLimit`, `tower::limit`, `GovernorLayer`, `ConcurrencyLimit` returns zero hits. There is no per-API-key rate limit, no per-tenant rate limit, no global concurrency cap on the upload endpoint. The 100 MB body limit + the small per-tenant pool means a single tenant can saturate its own pool by spamming uploads, and no other tenant is affected — but a single misbehaving CI bot can wedge that tenant's pool for the duration of the upload backlog.

WebSocket connections are similarly unlimited. A WS client can open thousands of `/v1/ws/accumulator/{name}` connections to one accumulator; each consumes server resources (mpsc channel slot, tokio task). The `EndpointRegistry` (`PERF-06`) takes a write lock per send, so high-volume WS pushes already serialize globally.

#### Evidence
- `grep "RateLimit\|GovernorLayer\|tower::limit"` in `cloacina-server`: zero hits.
- `crates/cloacina-server/Cargo.toml`: no `tower-governor`, no `tokio-stream::throttle` consumer.
- `routes/ws.rs` does not impose a max-connections-per-name limit.

#### Suggested Resolution
- Add `tower-governor` for HTTP rate limiting, scoped per API key (which is already extracted by middleware). Default 100 req/min/key.
- Add a per-accumulator connection limit (default 16) enforced at upgrade time in `accumulator_ws`.
- Add a `cloacina_rate_limited_total{route}` counter so operators can see when the limit fires.

**Cross-cutting note**: Security review should treat unbounded auth route + WS as a DoS surface. This is also called out in the soak-gap memory.

---

### OPS-20: Bootstrap admin key writes plaintext to `~/.cloacina/bootstrap-key` mode 0600 with no rotation guidance

**Severity**: Minor
**Location**: `crates/cloacina-server/src/lib.rs:603-655`
**Confidence**: High

#### Description
On first server start, if no API keys exist, `bootstrap_admin_key` generates a key, persists the **plaintext** to `~/.cloacina/bootstrap-key` with mode 0600, and logs `info!("Bootstrap admin key written to {} (mode 0600)", key_path.display())`. The runbook (`docs/content/platform/how-to-guides/production-deployment.md`) does not currently instruct the operator to rotate the bootstrap key after first use, delete the file, or revoke the bootstrap key after creating a real admin key. A long-running deployment retains the plaintext forever; an attacker who reads the file gets full admin access.

The `--bootstrap-key` CLI flag (`crates/cloacina-server/src/main.rs`) lets the operator provide their own key, which is better — but the file is still written.

#### Evidence
- `crates/cloacina-server/src/lib.rs:636-647`:
```rust
let key_path = home.join("bootstrap-key");
std::fs::write(&key_path, &plaintext)?;
#[cfg(unix)]
std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
```
- No `cloacinactl key revoke-bootstrap` verb.
- No log warning that "you should rotate this".

#### Suggested Resolution
- Print a `warn!` line: "Bootstrap key written to {}. Rotate after first admin login. Delete the file with `cloacinactl key delete-bootstrap` once you have a permanent admin key."
- Add a CLI verb `cloacinactl key delete-bootstrap` that revokes the bootstrap key in DB + deletes the file.
- Document rotation in the runbook.

**Cross-cutting note**: Security review territory.

---

### OPS-21: Daemon SIGHUP reload is documented but only re-reads watch dirs — config changes that affect the DB pool, intervals, or registry settings are silently ignored

**Severity**: Minor
**Location**: `crates/cloacinactl/src/commands/daemon.rs:339-365`
**Confidence**: High

#### Description
The daemon advertises SIGHUP-driven config reload (`info!("Received SIGHUP — reloading configuration...")`), but the implementation only reloads `CloacinaConfig` and diffs the watch directory list. Other fields of the config (`trigger_poll_interval_ms`, `cron_recovery_interval_s`, `cron_max_catchup`, `watcher_debounce_ms`, `shutdown_timeout_s`) are bound to the `DefaultRunner` at startup and not re-applied on SIGHUP. An operator who edits `~/.cloacina/config.toml` to bump `trigger_poll_interval_ms` and SIGHUPs will see the watch-dir diff log lines but no actual change in the trigger poll interval. The behavior is silent — there's no `warn!("Config field X changed but cannot be hot-reloaded")` — so the operator concludes the reload worked.

#### Evidence
- `crates/cloacinactl/src/commands/daemon.rs:340-365` — reload body.
- `DefaultRunner` has no `reload_config(&self, new: DefaultRunnerConfig)` method.

#### Suggested Resolution
On SIGHUP, diff the new config against the in-memory copy and `warn!` per field that "cannot be hot-reloaded; restart to apply". Or, plumb a subset of reloadable fields (poll intervals are easy — they're `Arc<AtomicU64>` candidates) into the running services. The current shape is misleading.

---

## Positive Patterns

1. **Credential-logging guard as a CI check** (`scripts/check_credential_logging.py`, run by `angreal lint:credential-logging`). The guard scans every Rust source file for `info!`/`debug!`/`println!` macros referencing sensitive identifiers (`database_url`, `db_url`, `connection_string`, `password`) without an explicit `// allow(credential-logging): <reason>` annotation. Combined with the `cloacina::logging::mask_db_url` helper used at every server/daemon/python startup log site, this is a real defense in depth. The exception-allowlist mechanism is also right (false-positive handling without disabling the check).

2. **`/metrics` validated by `promtool check metrics` in CI** via `angreal test:metrics-format`. The task boots a real server, scrapes `/metrics`, pipes it to `promtool`, and fails on exposition issues. T-0536 added this; few projects do this level of metric validation in CI. The bounded-reason label vocabulary (`task_error|timeout|validation_failed|infrastructure|task_not_found|claim_lost|unknown`) is enforced by a positive test in `thread_task_executor.rs:1034` per the Correctness review's positive patterns.

3. **SQL-derived `cloacina_active_workflows` gauge re-seeded every scheduler tick** (`scheduler_loop.rs:166-168`). This is the explicit T-0534 fix. Re-seeding from `SELECT count(*) FROM workflow_executions WHERE status IN ('Pending','Running')` per tick eliminates the gauge-leak class of bug entirely. The pattern just needs to extend to `cloacina_active_tasks` per OPS-01.

4. **Server `/ready` checks DB connectivity AND crashed-graph status with structured payload**. `crates/cloacina-server/src/lib.rs:526-555` returns 503 with a JSON body explaining which dependency is down. Operators get the right signal at the right level for k8s/Nomad readiness probes. The DB pool check is a real `get_postgres_connection` (not a config-only check).

5. **Graceful shutdown ordering on the server is correct**: SIGINT/SIGTERM → `shutdown_tx.send(true)` → graph scheduler `shutdown_all().await` (waits for accumulators/reactors to flush + persist) → runner `shutdown()` (30s timeout) → log. The race conditions you'd worry about ("shutdown the runner before the graph scheduler had a chance to checkpoint") are explicitly resolved by ordering. Same shape on the daemon, with the additional second-SIGINT force-exit option.

6. **Daemon health pulse + Unix socket** (`crates/cloacinactl/src/commands/health.rs`). For a single-host SQLite-backed self-hosted daemon, a Unix socket exposing structured JSON health (active_workflows, packages_loaded, last_reconciliation, uptime_seconds) is exactly the right shape — minimal-dep, no port to firewall, queryable from any local CLI. Pair with adding Prometheus metrics (OPS-05) and the daemon's observability story is complete.
