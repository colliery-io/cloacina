# Cloacina Architecture Review -- Recommendations

---

## Immediate Actions (address before further development)

---

## REC-001: Wire existing WebSocket ticket system and enforce tenant DAL isolation
**Addresses**: SEC-001, SEC-006, SEC-008, CLF-05, CLF-06
**Severity of addressed findings**: Critical (SEC-002+SEC-006 combined), Major (SEC-001)
**Effort**: Days (3-5 days)

### What to do
1. Change `extract_ws_token` in `ws.rs` to call `state.ws_tickets.consume(&token)` when the token comes from a query parameter. Reject raw API keys in URL query parameters entirely.
2. Add capacity limit and periodic eviction to `WsTicketStore` (replace `HashMap` with `LruCache`, cap at 1024).
3. Implement per-request schema scoping for tenant-scoped endpoints: before each handler's DAL call, execute `SET search_path TO <tenant_schema>` on the connection, or create per-tenant DAL instances.

### Why it matters
These are the highest-severity security findings in the review. The combination of SEC-002 (unsigned code execution) and SEC-006 (no tenant isolation at DAL) means a compromised write-scoped API key can execute arbitrary native code with access to all tenants' data. The WebSocket ticket system already exists -- it just needs to be connected. Tenant isolation is the architectural promise of the multi-tenancy system; without DAL enforcement, the schema isolation is not realized.

### Suggested approach
- Start with SEC-006 (tenant isolation): modify the server's request pipeline to resolve the tenant schema from the URL path parameter, then set the connection's `search_path` before any DAL operations. The `Database::try_new_with_schema()` method already supports schema-scoped connections; the issue is that the server creates one runner for all tenants.
- For SEC-001: the `WsTicketStore::consume()` method already exists and works; replace the `validate_token` call in `extract_ws_token` with ticket consumption. Add a fallback that accepts Bearer tokens in the `Authorization` header (not query params) for non-browser clients.
- For SEC-008: swap the `HashMap` for a bounded LRU cache. Add a background task or lazy eviction on `issue()` that removes expired entries.

### Dependencies
None. All required code exists; this is integration work.

---

## REC-002: Resolve the double state-update path between dispatcher and executor
**Addresses**: COR-001, COR-004, CLF-03
**Severity of addressed findings**: Critical (combined COR-001+COR-004)
**Effort**: Days (2-3 days)

### What to do
1. Remove the `mark_completed()` and `mark_failed()` calls from `DefaultDispatcher::handle_result()`. The executor already persists state transitions in `complete_task_transaction()`.
2. Make `check_pipeline_completion` and `update_pipeline_final_context` run within a single database transaction.
3. Add a test that verifies exactly one `TaskCompleted` execution event is emitted per task completion.

### Why it matters
The double state-update produces duplicate execution events in the audit trail, misleading timestamps, and creates a race condition where the dispatcher could overwrite a retry status set by the executor. The pipeline completion race (COR-004) can produce incorrect final context. Neither condition is detectable with current metrics (OPS-002), making them invisible in production.

### Suggested approach
- In `DefaultDispatcher::handle_result()`, change the `Completed` and `Failed` branches to only log the result (using the existing `tracing::info!` calls) without calling DAL state-transition methods. The dispatcher's role becomes pure routing and result logging.
- For COR-004: wrap the `check_pipeline_completion` query and the `update_pipeline_final_context` writes in a single transaction. Use `SELECT ... FOR UPDATE` on the pipeline execution record to prevent concurrent completion.
- Add an integration test: execute a simple 2-task workflow, then query `execution_events` and assert exactly 2 completion events (one per task), not 4.

### Dependencies
None.

---

## REC-003: Enable package signature verification and harden server defaults
**Addresses**: SEC-002, SEC-005
**Severity of addressed findings**: Critical (SEC-002 in combination), Major (SEC-005)
**Effort**: Days (2-3 days)

### What to do
1. Add `--require-signatures` CLI flag and `security.require_signatures` config file key for the server.
2. Wire the existing signature verification pipeline into the `upload_workflow` handler (replacing the TODO comment).
3. Change the default bind address from `0.0.0.0:8080` to `127.0.0.1:8080`.
4. Add `--allow-plaintext` flag required when binding to non-loopback addresses without TLS.
5. Remove the vestigial `tower_governor` dependency from `Cargo.toml` and the dead `TOO_MANY_REQUESTS` error variant (rate limiting was intentionally removed due to throughput impact).

### Why it matters
Currently, there is no mode where packages are both accepted AND verified. The default bind on all interfaces without TLS exposes credentials in transit. The dead rate-limiting code creates false impressions about the security posture.

### Suggested approach
- For SEC-002: the verification pipeline in `security/verification.rs` already validates signatures against trusted keys. In `upload_workflow`, after receiving the package bytes, call `verify_package_signature()` before accepting the upload. The `SecurityConfig` already has `require_signatures` -- just expose it via CLI and config.
- For SEC-005: change the `default_value` in the Clap `#[arg]` attribute from `"0.0.0.0:8080"` to `"127.0.0.1:8080"`. Add a startup check: if the bind address is non-loopback and no TLS cert is configured and `--allow-plaintext` is not set, refuse to start with a clear error message.

### Dependencies
None.

---

## REC-004: Add daemon health check surface
**Addresses**: OPS-001
**Severity of addressed findings**: Critical
**Effort**: Days (1-2 days)

### What to do
Add a health reporting mechanism to `cloacinactl daemon` mode. Recommended approach: a lightweight HTTP endpoint on a configurable port serving `/health`.

### Why it matters
The daemon has zero health observability. Container orchestrators (Docker, Kubernetes) cannot determine if the daemon is alive, healthy, or functioning. They can only check process existence, which does not detect deadlocked schedulers, database connectivity loss, or reconciliation failures.

### Suggested approach
- Add a `--health-port` CLI option (default: 9090) to the daemon command.
- Spawn a minimal axum server (5 lines of routing: `/health` returning 200 with a JSON body including `last_reconciliation_time`, `active_workflow_count`, and `uptime`).
- The health endpoint should check database connectivity (a simple `SELECT 1` query) and report the scheduler's last successful tick timestamp.
- Add `HEALTHCHECK` to the Dockerfile: `CMD curl -sf http://localhost:9090/health || exit 1`.
- Add health check to `docker-compose.production.yml` for the cloacina service.

### Dependencies
None.

---

## REC-005: Fix silent dependency context loading failures
**Addresses**: COR-002
**Severity of addressed findings**: Major
**Effort**: Hours (2-4 hours)

### What to do
Change `build_task_context` in `ThreadTaskExecutor` to return an error when dependency context loading fails for non-root tasks, rather than silently continuing with empty or partial context.

### Why it matters
A task that depends on upstream data currently executes with empty context when the database is temporarily unavailable or data is corrupt. This produces incorrect results or confusing errors unrelated to the actual cause. The failure is only logged at `debug` level, making it nearly invisible in production.

### Suggested approach
- Replace `if let Ok(dep_metadata_with_contexts) = ...` with proper error propagation using `?` or `match` with explicit error handling.
- For root tasks (no dependencies), the current lenient behavior is acceptable.
- For non-root tasks, return `Err(ExecutorError::ContextLoadFailed { task_name, cause })`.
- Elevate the current `debug!` log to `error!` for the failure case.
- Consider adding a config flag `strict_context_loading: bool` (default `true`) for backward compatibility if needed.

### Dependencies
None.

---

## REC-006: Add configuration validation
**Addresses**: OPS-003, API-003, PERF-004
**Severity of addressed findings**: Major (all three)
**Effort**: Days (1-2 days)

### What to do
1. Add a `validate()` method to `DefaultRunnerConfig` with bounds checking.
2. Change `DefaultRunnerConfigBuilder::build()` to return `Result<DefaultRunnerConfig, ConfigError>`.
3. Change `cron_max_catchup_executions` default from `usize::MAX` to `100`.
4. Add `#[serde(deny_unknown_fields)]` to the TOML config struct.

### Why it matters
Invalid configurations are silently accepted and produce surprising runtime behavior: zero-permit semaphores deadlock execution, zero-interval polls create busy-loops, and `usize::MAX` catchup executions create storms after downtime. Config builder panics are unrecoverable, potentially crashing applications. TOML typos are silently ignored.

### Suggested approach
- Define validation rules: `max_concurrent_tasks > 0`, `scheduler_poll_interval >= 10ms`, `stale_claim_threshold > heartbeat_interval`, `cron_max_catchup_executions <= 1000`, `db_pool_size > 0`, `task_timeout > 0`.
- Replace the three `assert!` calls in `build()` with validation returning `Err(ConfigError::InvalidValue { field, value, constraint })`.
- For the TOML config, `#[serde(deny_unknown_fields)]` catches typos at parse time. Log a warning listing all unknown fields.
- Align `DefaultRunnerConfig` default for `cron_max_catchup_executions` with `SchedulerConfig` default of 100.

### Dependencies
None.

---

## Short-Term Actions (next sprint/cycle)

---

## REC-007: Complete the "pipeline" to "workflow" terminology migration
**Addresses**: LEG-001, LEG-014, API-002, API-008, CLF-01
**Severity of addressed findings**: Critical (combined cross-cutting)
**Effort**: Weeks (1-2 weeks, including migration testing)

### What to do
Rename all "pipeline" references to "workflow" throughout the codebase: database tables, model fields, error messages, Python config properties, test names, and internal variables.

### Why it matters
This is the most pervasive single issue in the codebase, affecting every consumer surface. It increases mean time to diagnosis during incidents, confuses new contributors, and creates the impression that "pipeline" and "workflow" are different concepts. The cross-cutting analysis elevated this to Critical severity.

### Suggested approach
Phase 1 (low-risk, high-impact):
- Rename Python config property from `pipeline_timeout_seconds` to `workflow_timeout_seconds` (API-002).
- Update all error message strings from "Pipeline" to "Workflow" (API-008, LEG-014).
- Rename `pipeline_executor.rs` to `workflow_executor.rs`.
- Rename internal variables: `pipeline_execution_id` to `workflow_execution_id`, etc.

Phase 2 (requires migration):
- Create database migration renaming `pipeline_executions` to `workflow_executions` and `pipeline_name`/`pipeline_version` columns to `workflow_name`/`workflow_version`.
- Create a view aliasing the old table name for backward compatibility during rollout.
- Update the Diesel schema file and all DAL code.

Phase 3 (cleanup):
- Rename test functions from `test_pipeline_*` to `test_workflow_*`.
- Update any remaining internal references.
- Search for `pipeline` case-insensitively to catch any stragglers.

### Dependencies
REC-002 (double state update fix) should be completed first, as it touches the same executor/dispatcher code paths. Doing the rename concurrently would create merge conflicts.

---

## REC-008: Implement the five phantom Prometheus metrics
**Addresses**: OPS-002, OPS-004
**Severity of addressed findings**: Major (OPS-002, OPS-004)
**Effort**: Days (2-3 days)

### What to do
1. Implement recording for all five described-but-never-recorded metrics.
2. Add `#[tracing::instrument]` spans at key lifecycle boundaries for distributed tracing.

### Why it matters
Operators see metric names and descriptions on `/metrics` suggesting comprehensive monitoring, but values are always zero. The four golden signals are mostly uncovered: no latency histograms, no saturation gauges, no error-rate metrics. Individual workflow executions cannot be traced through the system.

### Suggested approach
For metrics:
- `cloacina_api_requests_total`: Add request-counting middleware to the axum router (tower layer that increments counter with `method`, `path`, `status` labels).
- `cloacina_pipeline_duration_seconds`: Record `Instant::now()` in `schedule_workflow_execution`, emit `histogram!().record(elapsed)` in `complete_pipeline`.
- `cloacina_task_duration_seconds`: Record start time in `ThreadTaskExecutor::execute`, emit on completion.
- `cloacina_active_pipelines`: `gauge!().increment(1)` in `schedule_workflow_execution`, `gauge!().decrement(1)` in `complete_pipeline`.
- `cloacina_active_tasks`: Same pattern in executor `execute`/`complete_task_transaction`.

For tracing:
- Add `#[tracing::instrument(skip(self), fields(pipeline_execution_id, workflow_name))]` to `schedule_workflow_execution`.
- Add per-task spans in `dispatch_ready_tasks` with `task_execution_id`, `task_name`.
- Add spans in `ThreadTaskExecutor::execute` with `task_execution_id`, `task_name`, `attempt`.
- Add span in `complete_pipeline` with `pipeline_execution_id`, `status`.

### Dependencies
REC-002 (clarify state ownership) should be done first, as it determines which component owns each lifecycle boundary and therefore which component should record each metric.

---

## REC-009: Remove stub Python methods and fix Python runner error propagation
**Addresses**: API-001, API-009
**Severity of addressed findings**: Major (API-001), Minor (API-009)
**Effort**: Hours (3-4 hours)

### What to do
1. Remove `start()` and `stop()` from `PyDefaultRunner`. The runner auto-starts on construction and has a working `shutdown()` method.
2. Replace `.expect("Failed to create DefaultRunner")` with proper error propagation in `PyDefaultRunner::new()` and `with_config()`.

### Why it matters
`start()` and `stop()` invite users to call them, then fail at runtime. This is worse than absent methods -- it wastes debugging time. The `.expect()` on runner creation turns database errors into panics that manifest as confusing channel-disconnect errors in Python.

### Suggested approach
- Simply delete the `start()` and `stop()` method definitions from the `#[pymethods]` impl block.
- For error propagation: use the existing `oneshot` channel pattern. In the background thread, match on the `DefaultRunner::new()` result and send the error through the channel. On the main thread, convert the error to `PyRuntimeError` or `PyConnectionError`.

### Dependencies
None.

---

## REC-010: Add heartbeat-driven cancellation for claim-lost tasks
**Addresses**: COR-005
**Severity of addressed findings**: Minor
**Effort**: Days (1-2 days)

### What to do
Provide a cancellation mechanism that the heartbeat task can trigger when it detects `ClaimLost`, preventing the running task from saving results that belong to another runner.

### Why it matters
When a heartbeat detects ClaimLost, the task continues running to completion and attempts to save context, potentially overwriting results from the runner that legitimately claimed the task. This can cause duplicate execution and data corruption.

### Suggested approach
- Create a `CancellationToken` (from `tokio_util::sync`) and pass it into `ThreadTaskExecutor::execute`.
- When the heartbeat detects `ClaimLost`, trigger the cancellation token.
- Before `complete_task_transaction`, check the cancellation token. If cancelled, skip saving and log a warning.
- Optionally, pass the token into the task function via `Context` metadata so tasks can cooperatively check cancellation.

### Dependencies
None.

---

## Structural Improvements (larger efforts to schedule)

---

## REC-011: Unify the dual-backend DAL into a single code path
**Addresses**: LEG-002, EVO-003, EVO-007, COR-007, PERF-006, CLF-02
**Severity of addressed findings**: Major (LEG-002, EVO-003), Major (COR-007 adjusted), Minor (EVO-007, PERF-006)
**Effort**: Weeks (2-4 weeks)

### What to do
Introduce a backend abstraction that collapses paired `_postgres`/`_sqlite` methods into single implementations. This eliminates the code duplication, the correctness divergence between merge strategies, and the structural barrier to adding new storage backends.

### Why it matters
The dual-backend pattern is the second most impactful root cause (RC-2) in the review. Every persistence change must be implemented twice. The context merge divergence (COR-007) is a direct consequence -- the two paths have already silently diverged. Feature flag combinatorics (EVO-007) create 4 separate `#[cfg]` blocks. Adding a third backend would require 45-120 new method implementations.

### Suggested approach
**Phase 1: Backend-agnostic connection abstraction**
- Add an `async fn with_connection<F, R>(&self, f: F) -> Result<R, DalError>` method to `Database` that abstracts connection acquisition. Both `get_postgres_connection()` and `get_sqlite_connection()` are routed through this single method.
- Alternatively, investigate Diesel's `MultiConnection` feature, which provides enum-based dispatch across backends with a single query definition.

**Phase 2: Collapse DAL method pairs**
- Starting with the simplest entity (e.g., `context.rs`), refactor `create_postgres` and `create_sqlite` into a single `create` method using the new abstraction. Verify tests pass.
- Repeat for each entity module, progressing from simplest to most complex.
- Extract the shared context merge logic (from `thread_task_executor.rs`) into a `context::merge` utility. Apply it in both `ThreadTaskExecutor` and `ContextManager` (fixes COR-007).

**Phase 3: Simplify the macro and feature flags**
- Replace `dispatch_backend!` with direct calls to unified methods.
- Consolidate `#[cfg]` blocks in `Database::run_migrations()` into a single match on `BackendType`.

### Dependencies
REC-007 (terminology migration) should be done before or during Phase 2, as both touch DAL files extensively. Doing them concurrently avoids double-editing.

---

## REC-012: Introduce a DAL trait for testability
**Addresses**: EVO-005, EVO-006
**Severity of addressed findings**: Minor (both)
**Effort**: Weeks (1-2 weeks)

### What to do
Define a `trait DataAccessLayer` with methods returning trait objects for each entity type. This enables in-memory mock implementations for unit testing and reduces dependence on database infrastructure in CI.

### Why it matters
Currently, 161 uses of `#[serial]` across 24 test files prevent parallel test execution. Integration tests require a running PostgreSQL instance. The most critical correctness paths (COR-001 double state update, COR-004 completion race) have no test coverage partly because the test architecture makes such tests difficult to write.

### Suggested approach
- Define `trait DataAccessLayer` with methods like `fn task_execution(&self) -> &dyn TaskExecutionOps`.
- Define sub-traits for each entity: `trait TaskExecutionOps`, `trait WorkflowExecutionOps`, etc.
- Create an `InMemoryDAL` implementation using `HashMap` and `Mutex` for testing.
- Refactor `ThreadTaskExecutor`, `DefaultDispatcher`, and `TaskScheduler` to accept `Arc<dyn DataAccessLayer>` instead of `DAL`.
- Write tests for COR-001 and COR-004 using the in-memory DAL.

### Dependencies
REC-011 (DAL unification) should be done first, as it stabilizes the DAL interface. Defining traits against a unified interface is simpler than defining them against the dual-backend pattern.

---

## REC-013: Extract DefaultRunner services into a ServiceManager
**Addresses**: EVO-004, LEG-012
**Severity of addressed findings**: Major (EVO-004), Minor (LEG-012)
**Effort**: Days (3-5 days)

### What to do
Replace the 6 `Arc<RwLock<Option<Arc<...>>>>` fields on `DefaultRunner` with a single `ServiceManager` that holds a dynamic collection of `Box<dyn BackgroundService>`.

### Why it matters
`DefaultRunner` is a god object that orchestrates 8+ services. Adding any new service requires modifying the runner, its builder, service startup, and shutdown. The `Arc<RwLock<Option<Arc<T>>>>` triple-nesting is cognitively expensive and fragile.

### Suggested approach
- Define `trait BackgroundService: Send + Sync` with `async fn start(&self)`, `async fn shutdown(&self)`, and `fn name(&self) -> &str`.
- Implement for each service: `TaskSchedulerService`, `CronRecoveryService`, `RegistryReconcilerService`, `ReactiveSchedulerService`.
- `ServiceManager` holds `Vec<Box<dyn BackgroundService>>` and provides `start_all()`, `shutdown_all()`.
- `DefaultRunner` delegates lifecycle management to `ServiceManager`, reducing its field count to ~5 (runtime, database, config, scheduler, service_manager).

### Dependencies
None. Can be done independently.

---

## REC-014: Extend Runtime to cover all global registries
**Addresses**: EVO-002, EVO-008
**Severity of addressed findings**: Major (EVO-002), Observation (EVO-008)
**Effort**: Days (2-3 days)

### What to do
Extend the `Runtime` struct to encompass computation graph registries, stream backend registries, and Python registries. Make `#[ctor]` auto-registration opt-in.

### Why it matters
The `Runtime` struct only covers tasks, workflows, and triggers. Computation graphs, stream backends, and Python registries remain global, requiring `#[serial]` for all tests that touch these systems. This is a pragmatic first step toward the larger goal of eliminating process-global mutable state (RC-3) without requiring the full crate-splitting effort.

### Suggested approach
- Add computation graph and stream backend registry fields to `Runtime`.
- Update `ReactiveScheduler` and related code to use `Runtime` rather than global statics.
- Make `#[ctor]` registration on tasks/workflows register to a thread-local or lazy-initialized staging area, with `Runtime::from_global()` consuming staged registrations. This enables opt-out for tests.

### Dependencies
None.

---

## Architectural Recommendations

### AR-1: Adopt a "production mode" configuration preset

Beyond individual findings, the system needs a clear distinction between development and production configurations. Add a `CLOACINA_ENV=production` environment variable or `--production` flag that enables:
- TLS required (or explicit `--allow-plaintext` override)
- Package signature verification enabled

- `cron_max_catchup_executions` capped at 100
- Configuration validation strict (reject invalid values)
- Health endpoints enabled on daemon

This makes the developer-convenience vs production-safety tradeoff (Tension T-1 from cross-cutting analysis) explicit and addressable.

### AR-2: Plan for crate decomposition as a medium-term goal

The monolithic `cloacina` crate (RC-3) is the root cause of multiple findings but is too expensive to split immediately. However, the following preparatory work can be done incrementally:
1. REC-014 (extend Runtime) removes the global state problem
2. REC-011 (DAL unification) creates a clean persistence interface
3. REC-012 (DAL trait) enables testing without the full crate
4. REC-013 (ServiceManager) decouples lifecycle management

After these are complete, splitting into `cloacina-dal`, `cloacina-engine`, `cloacina-python`, and `cloacina-security` becomes a mechanical refactor rather than an architectural project.

### AR-3: Adopt an integration checklist for new features

Systemic pattern SP-1 ("implementation exists but integration is missing") suggests adding a pre-release checklist:
- All declared interfaces are connected (no dead-code ticket stores, no phantom metrics)
- All `todo!()` and `TODO` comments are resolved or tracked
- All declared dependencies are imported and used
- All error variants have at least one code path that produces them
- All configuration keys have at least one test exercising non-default values

### AR-4: Invest in concurrent correctness testing

The test architecture (SP-4) structurally prevents testing the most critical paths. After REC-012 (DAL trait) enables in-memory testing, prioritize tests for:
- Double state update (COR-001): verify exactly one completion event per task
- Completion race (COR-004): verify correct final context when tasks complete concurrently
- Heartbeat claim-lost (COR-005): verify task cancellation propagates
- Concurrent claiming: verify `FOR UPDATE SKIP LOCKED` prevents duplicate execution

---

## Summary Roadmap

```
Week 1: Immediate security and correctness fixes (unblocks safe deployment)
  |
  +-- REC-001: Wire WebSocket tickets + tenant DAL isolation [Critical]
  |     (unblocks safe multi-tenant deployment)
  |
  +-- REC-002: Fix double state-update path [Critical]
  |     (unblocks REC-008 metrics, clarifies ownership)
  |
  +-- REC-003: Enable signature verification + harden defaults [Critical]
  |     (unblocks safe package uploads)
  |
  +-- REC-004: Add daemon health check [Critical]
  |     (unblocks containerized daemon deployment)
  |
  +-- REC-005: Fix silent context loading failures [Major]
  |
  +-- REC-006: Add configuration validation [Major]

Week 2-3: Short-term quality improvements (unblocks observability)
  |
  +-- REC-007: Complete pipeline -> workflow migration [Critical combined]
  |     (depends on REC-002 being done first to avoid conflicts)
  |
  +-- REC-008: Implement phantom metrics + add tracing spans [Major]
  |     (depends on REC-002 clarifying state ownership)
  |
  +-- REC-009: Remove Python stubs + fix error propagation [Major]
  |
  +-- REC-010: Add heartbeat cancellation [Minor]

Week 4+: Structural improvements (unblocks long-term evolvability)
  |
  +-- REC-013: Extract ServiceManager from DefaultRunner [Major]
  |     (no dependencies, can start any time)
  |
  +-- REC-014: Extend Runtime to all registries [Major]
  |     (no dependencies, can start any time)
  |
  +-- REC-011: Unify dual-backend DAL [Major, multi-week]
  |     (do alongside or after REC-007 to avoid double-editing)
  |
  +-- REC-012: DAL trait for testability [Minor]
        (depends on REC-011 stabilizing the DAL interface)
```

**Key sequencing rationale:**
- REC-001/002/003/004 are independent and can be parallelized in Week 1
- REC-007 depends on REC-002 (both touch executor/dispatcher; doing REC-002 first avoids conflicts)
- REC-008 depends on REC-002 (state ownership determines which component records each metric)
- REC-011 should be done alongside or after REC-007 (both touch DAL files; concurrent edits create merge conflicts)
- REC-012 depends on REC-011 (defining traits against a unified interface is simpler)
- REC-013 and REC-014 are independent and can start any time
