# Cloacina Architecture Review -- Consolidated Report

## Executive Summary

Cloacina is a well-engineered embedded workflow orchestration library with a clear architectural vision, strong trait-based extension points, and exemplary documentation patterns. The codebase demonstrates deliberate design choices -- DAG-based dependency management, transactional state persistence, content-based versioning, and clean builder patterns -- that make it genuinely useful for its target use case. The DAL accessor pattern, procedural macro API, graceful shutdown handling, and audit logging are standout strengths that should be preserved and extended.

However, three systemic issues undermine production readiness and long-term maintainability. First, an incomplete terminology migration from "pipeline" to "workflow" has created a naming collision that touches every consumer surface -- Rust, Python, HTTP, logs, and the database -- increasing incident diagnosis time and confusing new contributors. Second, the dual-backend DAL pattern (separate PostgreSQL and SQLite code paths joined only by a routing macro) doubles code volume, invites correctness divergences, and blocks the addition of new storage backends. Third, several security features are implemented but never activated: the WebSocket single-use ticket system exists as dead code while raw API keys are accepted in URL query parameters; package signature verification is disabled with no configuration path to enable it; and tenant isolation at the DAL layer is not enforced, meaning a compromised write-scoped API key can execute unsigned native code with access to all tenants' data. The overall assessment is that Cloacina is architecturally sound and feature-rich, but requires targeted integration work -- wiring existing security implementations, resolving state-ownership ambiguities, and consolidating the dual-backend pattern -- before it can be deployed with confidence in a multi-tenant production environment.

## Summary Table

| Lens | Critical | Major | Minor | Observations |
|------|----------|-------|-------|--------------|
| Legibility | 0 | 3 | 5 | 5 |
| Correctness | 1 (adjusted) | 3 | 3 | 4 |
| Evolvability | 0 | 4 | 3 | 3 |
| Performance | 0 | 3 | 3 | 3 |
| API Design | 0 | 5 | 5 | 4 |
| Operability | 1 | 3 | 3 | 5 |
| Security | 1 (adjusted) | 3 | 4 | 4 |
| **Cross-Cutting Adjustments** | **+2** | **+1** | **-1** | -- |
| **Totals** | **5** | **26** | **26** | **28** |

Note: Cross-cutting analysis elevated LEG-001+API-002+API-008 to Critical (combined), COR-001+COR-004 to Critical (combined), and SEC-002+SEC-006 to Critical (combined). COR-007 was elevated from Minor to Major. The "Totals" row reflects the adjusted severity counts, avoiding double-counting findings that were merged into cross-cutting clusters.

---

## Findings by Lens

---

### 1. Legibility

**Assessment**: The codebase is well-organized at the macro level with consistent documentation patterns, clean trait abstractions, and logical module decomposition. Two dominant sources of cognitive friction are the "pipeline" vs "workflow" naming collision and the dual-backend code duplication in the DAL. Module-level doc comments are exceptional, following the Diataxis framework with tutorials, how-to guides, and architecture diagrams.

#### LEG-001: "Pipeline" vs "Workflow" naming collision
- **Severity**: Major (elevated to Critical in cross-cutting analysis -- see CLF-01)
- **Location**: Pervasive; key files include `models/pipeline_execution.rs`, `dal/unified/pipeline_execution.rs`, `executor/pipeline_executor.rs`, `database/schema.rs`, `execution_planner/scheduler_loop.rs`
- **Confidence**: High
- **Description**: The public API uses "workflow" consistently, but the database layer, DAL, models, and internal scheduler use "pipeline" throughout. The database table is `pipeline_executions`, the model struct `WorkflowExecutionRecord` has fields named `pipeline_name` and `pipeline_version`, and error messages mix both terms. 539 occurrences of `pipeline_exec` vs 35 of `workflow_exec` across src.
- **Evidence**: `models/pipeline_execution.rs` line 29: `pub struct WorkflowExecutionRecord` with field `pub pipeline_name: String`; `executor/pipeline_executor.rs` line 104: `WorkflowExecutionError::ExecutionFailed` displays "Pipeline execution failed"; database table name `pipeline_executions`.
- **Suggested Resolution**: Align on "workflow." Rename the database table via migration, update model fields and error messages from `pipeline_*` to `workflow_*`. Alternatively, add a prominent comment in the DAL explaining the legacy term.

#### LEG-002: Postgres/SQLite code duplication via dispatch_backend macro
- **Severity**: Major
- **Location**: `dal/unified/pipeline_execution.rs` (1152 lines), `dal/unified/task_execution/claiming.rs` (829 lines), `execution_planner/mod.rs` lines 440-557, and 21 other files using `dispatch_backend!`
- **Confidence**: High
- **Description**: Nearly every DAL method is implemented twice with identical business logic differing only in connection acquisition. This doubles the code a reader must understand and creates maintenance risk where a bugfix applied to one backend may be missed in the other.
- **Evidence**: `dal/unified/pipeline_execution.rs` lines 60-120 vs lines 122-160+: identical transaction structure, only connection getter differs.
- **Suggested Resolution**: Extract common logic into a generic function parameterized by connection type, or use Diesel's `MultiConnection` to enable a single code path.

#### LEG-003: Multiple "Scheduler" concepts without disambiguation
- **Severity**: Major
- **Location**: `execution_planner/mod.rs` (TaskScheduler), `cron_trigger_scheduler.rs` (Scheduler), `computation_graph/scheduler.rs` (ReactiveScheduler), `runner/default_runner/mod.rs` line 87
- **Confidence**: High
- **Description**: Three distinct scheduler types with overlapping names. `DefaultRunner` has fields named `scheduler` (TaskScheduler) and `unified_scheduler` (Scheduler). The bare name `Scheduler` in the prelude gives no hint it handles cron/triggers.
- **Evidence**: `lib.rs` line 543: `pub use cron_trigger_scheduler::{Scheduler, SchedulerConfig}` -- bare name; `runner/default_runner/mod.rs` line 77/87: two scheduler fields with ambiguous names.
- **Suggested Resolution**: Rename `Scheduler` to `CronTriggerScheduler` or `ScheduleRunner`.

#### LEG-004: lib.rs re-export surface is excessively large
- **Severity**: Minor
- **Location**: `lib.rs` lines 530-578
- **Confidence**: High
- **Description**: Over 70 individual symbols re-exported at crate root, including implementation details like `global_task_registry()`, `register_task_constructor()`, `SlotToken`. Creates competing import paths with the prelude.
- **Suggested Resolution**: Move implementation-detail types behind their respective modules; reserve crate root for prelude and primary types.

#### LEG-005: "cloaca" vs "cloacina" naming for Python bindings
- **Severity**: Minor
- **Location**: `lib.rs` line 592
- **Confidence**: Medium
- **Description**: Python wheel named "cloaca" while Rust project is "cloacina." No in-code comment explains the difference.
- **Suggested Resolution**: Add a doc comment on `fn cloaca()` explaining the naming choice.

#### LEG-006: Duplicate debug log in build_task_context
- **Severity**: Minor
- **Location**: `executor/thread_task_executor.rs` lines 188-199
- **Confidence**: High
- **Description**: Same information logged twice at debug level with nearly identical messages. Appears to be a leftover from development.
- **Suggested Resolution**: Remove the second `tracing::debug!` call.

#### LEG-007: Execution_planner module name does not match its primary export
- **Severity**: Minor
- **Location**: `execution_planner/mod.rs`
- **Confidence**: Medium
- **Description**: Module named `execution_planner` but primary export is `TaskScheduler`. Module doc heading says "# Task Scheduler."
- **Suggested Resolution**: Rename module to `task_scheduler` or struct to `ExecutionPlanner`.

#### LEG-012: DefaultRunner struct has high field count and Arc<RwLock<Option<Arc<T>>>> nesting
- **Severity**: Minor
- **Location**: `runner/default_runner/mod.rs` lines 69-91
- **Confidence**: High
- **Description**: 10 fields, 5 following `Arc<RwLock<Option<Arc<T>>>>` triple wrapping. Cognitively expensive.
- **Suggested Resolution**: Extract optional services into a `ServiceRegistry` struct.

#### LEG-014: Error types display "Pipeline" in user-facing messages
- **Severity**: Minor (subsumed by LEG-001/CLF-01 for cross-cutting analysis)
- **Location**: `executor/pipeline_executor.rs` lines 97-120
- **Confidence**: High
- **Description**: `WorkflowExecutionError` variants display "Pipeline" in messages contradicting the type name.
- **Suggested Resolution**: Update error messages to use "Workflow."

**Notable Positive Patterns**:
- LEG-008: DAL accessor pattern (`dal.task_execution().mark_completed(id)`) is exceptionally legible
- LEG-009: Execution_planner module is cleanly decomposed into focused submodules
- LEG-010: Runtime struct provides clean registry isolation with clear two-mode design
- LEG-011: `var.rs` is a model of concise, self-documenting design
- LEG-013: cloacinactl CLI is cleanly structured with Clap-derived command hierarchy

---

### 2. Correctness

**Assessment**: Strong correctness fundamentals -- transactional state transitions, clear error hierarchies, well-structured retry logic, comprehensive unit tests for core data structures. The most significant risks are the double state-update path between dispatcher and executor, silent swallowing of dependency-loading failures, and unsafe Send/Sync declarations on FFI/Python wrappers.

#### COR-001: Double State Update on Task Completion
- **Severity**: Major (elevated to Critical in cross-cutting analysis -- see CLF-03)
- **Location**: `executor/thread_task_executor.rs:906-920`, `dispatcher/default.rs:92-103`
- **Confidence**: High
- **Description**: When a task completes, both `ThreadTaskExecutor::complete_task_transaction()` and `DefaultDispatcher::handle_result()` call `mark_completed()`, producing duplicate execution events, misleading timestamps, and extra DB transactions. For failures, the dispatcher could overwrite a retry status set by the executor.
- **Evidence**: Both paths call `mark_completed` for the same `task_execution_id`.
- **Suggested Resolution**: Remove state-transition calls from `DefaultDispatcher::handle_result()` for Completed and Failed statuses, since the executor already handles these.

#### COR-002: Silent Swallowing of Dependency Context Loading Failures
- **Severity**: Major
- **Location**: `executor/thread_task_executor.rs:248-293`
- **Confidence**: High
- **Description**: `build_task_context` uses `if let Ok(...)` guards to silently swallow database errors and JSON parse failures when loading dependency contexts. Tasks run with empty or partial context rather than failing with a clear error. Only logged at debug level.
- **Evidence**: `else` branch only logs at `debug` level; function returns `Ok(context)` with partial data.
- **Suggested Resolution**: Return an error when dependency context loading fails for non-root tasks. Elevate logging to `warn` or `error`.

#### COR-003: Unsafe Send/Sync on Python Wrappers Without Compile-Time Enforcement
- **Severity**: Major
- **Location**: `python/task.rs:129-130`, `python/trigger.rs:163-164`, `python/computation_graph.rs:491-492`, `python/bindings/trigger.rs:130-131`
- **Confidence**: Medium
- **Description**: Four structs holding `PyObject` fields are marked `unsafe impl Send/Sync` relying on convention that all access goes through `Python::with_gil()`. No compile-time or concurrent stress testing enforcement.
- **Evidence**: Safety comments document convention-based invariants. No focused concurrency tests exist.
- **Suggested Resolution**: Wrap PyObject in a `GilProtected<PyObject>` newtype enforcing GIL acquisition at the type level. Add concurrent stress tests.

#### COR-004: Pipeline Completion Check Races With Executor Writes
- **Severity**: Major (elevated to Critical in cross-cutting analysis -- see CLF-03)
- **Location**: `execution_planner/scheduler_loop.rs:249-259`
- **Confidence**: Medium
- **Description**: The scheduler calls `check_pipeline_completion` and `update_pipeline_final_context` concurrently with executor writes. A task completing during the context scan may be missed, producing incorrect final context.
- **Evidence**: `check_pipeline_completion` and `update_pipeline_final_context` are separate queries not in a single transaction.
- **Suggested Resolution**: Run both within a single database transaction, or have the executor trigger pipeline completion when it completes the last task.

#### COR-005: Heartbeat ClaimLost Does Not Cancel Running Task
- **Severity**: Minor
- **Location**: `executor/thread_task_executor.rs:767-793`
- **Confidence**: High
- **Description**: When heartbeat detects `ClaimLost`, it breaks the heartbeat loop but the task continues running, potentially causing duplicate execution by two runners.
- **Suggested Resolution**: Provide a cancellation token that the heartbeat can trigger. Check the token before saving results.

#### COR-006: Consecutive Error Counter Can Never Reset Under Sustained Errors
- **Severity**: Minor
- **Location**: `execution_planner/scheduler_loop.rs:132-170`
- **Confidence**: High
- **Description**: `consecutive_errors` counter is a `u32` that increments without bound. Rate-limited logging at modulo 10 means hundreds of errors can pass silently.
- **Suggested Resolution**: Use `saturating_add` or cap the counter. Emit periodic warnings by time interval rather than error count.

#### COR-007: Context Merge Strategy Differs Between Executor and ContextManager
- **Severity**: Minor (elevated to Major in cross-cutting analysis -- see CLF-02)
- **Location**: `executor/thread_task_executor.rs:317-352` vs `execution_planner/context_manager.rs:147-198`
- **Confidence**: High
- **Description**: Executor uses smart merging (array concatenation, recursive object merge); ContextManager uses simple overwrite. Same workflow produces different merged contexts depending on which path evaluates them.
- **Suggested Resolution**: Extract merge strategy into a shared utility function. Adopt executor's smart merge in ContextManager.

**Notable Positive Patterns / Observations**:
- COR-008: Stale claim sweeper counts releases even when some fail (minor reporting issue)
- COR-009: PostgreSQL distributor cannot be created via factory function (incomplete abstraction)
- COR-010: `TriggerRule::All` with empty conditions vacuously returns true (mathematically correct, potentially surprising)
- COR-011: `clone()` on ThreadTaskExecutor snapshots atomic counters (metrics are per-clone, not aggregate)

---

### 3. Evolvability

**Assessment**: Moderately evolvable. Clean trait boundaries for executors, dispatchers, and registries enable pluggable backends. However, the monolithic core crate, process-global static registries, and dual-backend DAL duplication impose high change costs for persistence modifications and test infrastructure.

#### EVO-001: Monolithic core crate conflates many concerns
- **Severity**: Major
- **Location**: `lib.rs` lines 490-578, `Cargo.toml`
- **Confidence**: High
- **Description**: The `cloacina` crate is simultaneously a persistence library, execution engine, packaging/security system, Python binding module, and computation graph runtime. 43 direct dependencies. 21 public modules. Builds as both `lib` and `cdylib`.
- **Suggested Resolution**: Split into focused crates: `cloacina-dal`, `cloacina-engine`, `cloacina-python`, `cloacina-security`. Facade crate for backward compatibility.

#### EVO-002: Process-global static registries impede test isolation
- **Severity**: Major
- **Location**: `task.rs:637`, `workflow/registry.rs:36`, `trigger/registry.rs:36`, `computation_graph/global_registry.rs`, plus 5 more
- **Confidence**: High
- **Description**: 9+ process-global `Lazy<Mutex/RwLock<...>>` statics. 161 uses of `#[serial]` across 24 test files. `Runtime` struct covers tasks/workflows/triggers but not computation graphs, stream backends, or Python registries.
- **Suggested Resolution**: Extend `Runtime` to encompass all registry types. Make `#[ctor]` registration opt-in.

#### EVO-003: Dual-backend DAL pattern doubles persistence code
- **Severity**: Major
- **Location**: `dal/unified/` (all 15+ sub-modules), `database/connection/backend.rs:265`
- **Confidence**: High
- **Description**: Every DAL entity module has paired `_postgres` and `_sqlite` methods with identical logic. ~60-120 method pairs. Adding a third backend requires 45-120 new implementations.
- **Suggested Resolution**: Introduce `async fn with_connection<F, R>(&self, f: F)` or use Diesel's `MultiConnection`.

#### EVO-004: DefaultRunner is a god object orchestrating 8+ services
- **Severity**: Major
- **Location**: `runner/default_runner/mod.rs:69-91`, `runner/default_runner/services.rs`
- **Confidence**: High
- **Description**: 11 fields including 6 `Arc<RwLock<Option<Arc<...>>>>`. Manages background task lifecycles, shutdown coordination, and exposes methods spanning execution, scheduling, registry, and reactive scheduling.
- **Suggested Resolution**: Extract service lifecycle into a `ServiceManager` with `Box<dyn BackgroundService>`.

#### EVO-005: No trait abstraction for the DAL aggregate
- **Severity**: Minor
- **Location**: `dal/unified/mod.rs:94-231`
- **Confidence**: Medium
- **Description**: `DAL` is a concrete type with no trait. Tests require real database fixtures. No mock DAL possible.
- **Suggested Resolution**: Define a `trait DataAccessLayer` with methods returning trait objects for each entity.

#### EVO-006: Test architecture requires database infrastructure
- **Severity**: Minor
- **Location**: `tests/fixtures.rs`, `tests/integration/`
- **Confidence**: High
- **Description**: Integration tests require PostgreSQL. 161 `#[serial]` occurrences prevent parallel execution. `cloacina-testing` covers only pure task logic.
- **Suggested Resolution**: Invest in DAL traits (EVO-005) for in-memory test doubles. Consider per-test schemas for parallelism.

#### EVO-007: Feature flag combinatorics create maintenance burden
- **Severity**: Minor
- **Location**: `database/connection/mod.rs` lines 377-449, `Cargo.toml` features
- **Confidence**: Medium
- **Description**: 4 feature flag combinations require separate `#[cfg]` blocks. `run_migrations()` has 4 blocks. CI must test 3+ combinations.
- **Suggested Resolution**: Consolidate with enum-dispatch or `DatabaseBackend` trait.

**Notable Positive Patterns / Observations**:
- EVO-008: Computation graph system is parallel but partially integrated (separate registries, schedulers, health)
- EVO-009: Python bindings are tightly coupled to internals (14 files mirroring internal modules)
- EVO-010: Extension points are well-designed where they exist -- `TaskExecutor`, `Dispatcher`, `WorkflowRegistry` traits are clean and minimal

---

### 4. Performance

**Assessment**: Generally appropriate for the embedded orchestration workload. Hot paths are well-optimized (batch queries, `FOR UPDATE SKIP LOCKED`, semaphore concurrency). Significant concerns are an N+1 query pattern in pipeline completion, unbounded retry-ready scans, and a `usize::MAX` default for cron catchup that can cause execution storms.

#### PERF-001: N+1 query pattern in pipeline final context resolution
- **Severity**: Major
- **Location**: `execution_planner/scheduler_loop.rs`, `update_pipeline_final_context()` lines 359-416
- **Confidence**: High
- **Description**: On pipeline completion, iterates all tasks calling `get_by_pipeline_and_task()` per task. N queries for N tasks.
- **Evidence**: Per-task database lookup inside a loop; data could be batch-loaded.
- **Suggested Resolution**: Add `get_metadata_batch_by_pipeline()` query. Filter and find latest context in memory.

#### PERF-002: Unbounded `get_ready_for_retry` scans full table
- **Severity**: Major
- **Location**: `dal/unified/task_execution/claiming.rs` lines 777-828
- **Confidence**: High
- **Description**: Query loads ALL tasks with `status = 'Ready'` and past `retry_at`. No `LIMIT`. Called every 100ms.
- **Evidence**: `.load(conn)` with no limit; no composite index on `(status, retry_at)`.
- **Suggested Resolution**: Add `LIMIT` matching available executor capacity. Add composite index on `(status, retry_at) WHERE status = 'Ready'`.

#### PERF-004: Default `cron_max_catchup_executions` is `usize::MAX`
- **Severity**: Major
- **Location**: `runner/default_runner/config.rs` line 271
- **Confidence**: High
- **Description**: After scheduler downtime, system attempts every missed cron invocation without limit. 1-day outage with 1-minute cron = 1,440 backlogged executions.
- **Evidence**: `cron_max_catchup_executions: usize::MAX` overrides the `SchedulerConfig` default of 100.
- **Suggested Resolution**: Change default to a bounded value (10-100). Align with `SchedulerConfig` default.

#### PERF-003: Trigger condition evaluation issues individual queries per condition
- **Severity**: Minor
- **Location**: `execution_planner/state_manager.rs`, `evaluate_condition()` lines 245-321
- **Confidence**: Medium
- **Description**: Each `TaskSuccess`/`TaskFailed`/`TaskSkipped` condition makes an individual `get_task_status()` call. Only manifests with complex trigger rules.
- **Suggested Resolution**: Pre-collect task names, batch-fetch statuses with `get_task_statuses_batch()`.

#### PERF-005: SQLite connection pool hardcoded to size 1
- **Severity**: Minor
- **Location**: `database/connection/mod.rs` lines 239, 293
- **Confidence**: High
- **Description**: Ignores caller's `max_size` parameter. With WAL mode, 2-4 connections could allow concurrent reads.
- **Suggested Resolution**: Document the override. Consider allowing small pool (2-4) with WAL mode.

#### PERF-006: Duplicate context loading in StateManager and ContextManager
- **Severity**: Minor
- **Location**: `execution_planner/state_manager.rs:96-145`, `execution_planner/context_manager.rs:47-144`
- **Confidence**: Medium
- **Description**: Both independently fetch pipeline execution record and workflow from runtime for the same evaluation.
- **Suggested Resolution**: Pass already-fetched pipeline and workflow through to context manager.

**Notable Positive Patterns / Observations**:
- PERF-007: Execution events table grows without automatic retention (manual cleanup via `cloacinactl admin cleanup-events`)
- PERF-008: Dispatcher executes tasks synchronously in dispatch loop (adequate at current scale)
- PERF-009: Context merge uses O(n*m) array deduplication (adequate for typical small arrays)

---

### 5. API Design

**Assessment**: Well-structured interfaces within each layer (Rust, Python, HTTP, CLI) with consistent builder patterns and error formats. Significant cross-layer inconsistencies: "pipeline" terminology leaks into Python config; Python runner has stub methods; HTTP routes differ from documentation; `DefaultRunnerConfigBuilder.build()` panics instead of returning Result.

#### API-001: Python runner exposes non-functional `start()` and `stop()` methods
- **Severity**: Major
- **Location**: `python/bindings/runner.rs` lines 1411-1428
- **Confidence**: High
- **Description**: Public methods that unconditionally raise `ValueError`. Users discover they are stubs only at runtime. Working `shutdown()` method exists but must be discovered by trial and error.
- **Suggested Resolution**: Remove `start()` and `stop()` entirely until they work.

#### API-002: "pipeline" terminology exposed in Python config API
- **Severity**: Major (elevated to Critical in cross-cutting analysis -- see CLF-01)
- **Location**: `python/bindings/context.rs` lines 38, 55, 80-81, 144-145, 215-217, 283-284
- **Confidence**: High
- **Description**: Python `DefaultRunnerConfig` exposes `pipeline_timeout_seconds` as constructor parameter, getter, and setter while everything else uses "workflow."
- **Suggested Resolution**: Rename to `workflow_timeout_seconds` in Python bindings.

#### API-003: `DefaultRunnerConfigBuilder.build()` panics instead of returning `Result`
- **Severity**: Major
- **Location**: `runner/default_runner/config.rs` lines 461-474
- **Confidence**: High
- **Description**: Uses `assert!` macros for validation, causing unrecoverable panics on invalid configuration. Three conditions trigger panics.
- **Suggested Resolution**: Change `build()` to return `Result<DefaultRunnerConfig, ConfigError>`.

#### API-004: HTTP API route structure differs from documented paths
- **Severity**: Major
- **Location**: `cloacinactl/src/commands/serve.rs` lines 298-396
- **Confidence**: High
- **Description**: Documented routes lack tenant scoping. Actual routes are `/v1/tenants/{tenant_id}/...`. Documented `POST /accumulators` endpoint does not exist.
- **Suggested Resolution**: Update documentation to reflect actual tenant-scoped routes.

#### API-005: `list_executions` returns only active executions, not all
- **Severity**: Major
- **Location**: `cloacinactl/src/server/executions.rs` lines 96-135
- **Confidence**: High
- **Description**: Endpoint named "list_executions" calls `get_active_executions()`, returning only non-terminal executions. No pagination, no status filtering, no historical access.
- **Suggested Resolution**: Accept query parameters for status filtering and pagination, or rename endpoint to `/active-executions`.

#### API-006: `DefaultRunnerConfig` has 28 fields with no logical grouping
- **Severity**: Minor
- **Location**: `runner/default_runner/config.rs` lines 58-90
- **Confidence**: High
- **Description**: Flat structure with 28 setter methods spanning concurrency, database, cron, triggers, registry, and claiming.
- **Suggested Resolution**: Group into sub-configs: `CronConfig`, `TriggerConfig`, `RegistryConfig`, `ClaimingConfig`.

#### API-007: `Context` uses different method semantics in Rust vs Python
- **Severity**: Minor
- **Location**: `cloacina-workflow/src/context.rs` vs `python/context.rs`
- **Confidence**: High
- **Description**: Python adds `set()` (insert-or-update) that does not exist in Rust. Different exception types for analogous failures.
- **Suggested Resolution**: Add `set()` or `upsert()` to Rust `Context`.

#### API-008: Error types use wrong terminology in user-facing messages
- **Severity**: Minor (subsumed by CLF-01 for cross-cutting analysis)
- **Location**: `executor/pipeline_executor.rs` lines 104, 107; `error.rs` lines 210, 233, 288
- **Confidence**: High
- **Description**: Five error variants display "Pipeline" in messages from types named with "Workflow."
- **Suggested Resolution**: Update all error message strings to use "Workflow."

#### API-009: Python `DefaultRunner` creation panics on failure instead of raising
- **Severity**: Minor
- **Location**: `python/bindings/runner.rs` lines 318-322, 660-663
- **Confidence**: High
- **Description**: `.expect("Failed to create DefaultRunner")` inside background thread. Database errors manifest as confusing channel-disconnect errors in Python.
- **Suggested Resolution**: Propagate errors through channel back to Python; raise descriptive `PyRuntimeError`.

#### API-010: `get_workflow` performs full list-and-filter instead of direct lookup
- **Severity**: Minor
- **Location**: `cloacinactl/src/server/workflows.rs` lines 157-195
- **Confidence**: High
- **Description**: Single-workflow lookup loads all workflows then does `.find()` in memory. O(n) instead of O(1).
- **Suggested Resolution**: Add `get_workflow_by_name()` to `WorkflowRegistry` trait.

#### API-011: Python and Rust config surface parity gap
- **Severity**: Minor
- **Location**: `python/bindings/context.rs` vs `runner/default_runner/config.rs`
- **Confidence**: High
- **Description**: Python exposes 14 of 28 Rust config fields. Missing: trigger scheduling, registry, claiming, routing configs.
- **Suggested Resolution**: Expose remaining fields or document intentional omissions.

#### API-012: Two separate builders for the same system
- **Severity**: Minor
- **Location**: `runner/default_runner/config.rs` lines 254-474 and 509-722
- **Confidence**: High
- **Description**: `DefaultRunnerBuilder` and `DefaultRunnerConfigBuilder` overlap; `routing_config` appears in both. Setting both causes silent override.
- **Suggested Resolution**: Remove `routing_config()` from `DefaultRunnerBuilder`, leaving it solely on config builder.

**Notable Positive Patterns / Observations**:
- API-013: DAL accessor pattern is exemplary API design
- API-014: `Dispatcher` and `TaskExecutor` extension traits are clean and well-documented with example implementations
- API-015: HTTP API error format is consistent and machine-readable with `error`/`code` fields
- API-016: Macro API is ergonomic with compile-time validation

---

### 6. Operability

**Assessment**: The `cloacinactl serve` mode is reasonably production-ready with health/ready endpoints, Prometheus metrics endpoint, request-ID tracing, structured JSON logs, and graceful shutdown. The `cloacinactl daemon` mode has no health check surface. Metrics are mostly phantom (5 of 7 described but never recorded). No distributed tracing through the execution hot path.

#### OPS-001: Daemon mode has no health check endpoint
- **Severity**: Critical
- **Location**: `cloacinactl/src/commands/daemon.rs`
- **Confidence**: High
- **Description**: No HTTP listener, no Unix socket, no PID-based health file. Container orchestrators cannot determine daemon health, only process existence.
- **Evidence**: Daemon's `run()` enters `tokio::select!` with no health reporting mechanism.
- **Suggested Resolution**: Add minimal health reporting (lightweight HTTP endpoint, periodic health file, or Unix domain socket).

#### OPS-002: Metrics described but never recorded
- **Severity**: Major
- **Location**: `cloacinactl/src/commands/serve.rs` lines 128-150, plus scheduler and executor files
- **Confidence**: High
- **Description**: Seven Prometheus metrics described at startup; only two counters (`cloacina_pipelines_total`, `cloacina_tasks_total`) are actually incremented. Five phantom metrics (API request count, pipeline/task duration histograms, active pipeline/task gauges) always show zero.
- **Suggested Resolution**: Either remove phantom metric descriptions or implement recording at lifecycle boundaries.

#### OPS-003: No configuration validation at startup
- **Severity**: Major
- **Location**: `runner/default_runner/config.rs` lines 259-293, `cloacinactl/src/commands/config.rs` lines 90-119
- **Confidence**: High
- **Description**: Invalid configurations are silently accepted. `max_concurrent_tasks: 0` creates zero-permit semaphore; `scheduler_poll_interval: 0ms` creates busy-loop; TOML typos produce no warning.
- **Suggested Resolution**: Add `validate()` method checking bounds. Use `#[serde(deny_unknown_fields)]` for TOML config.

#### OPS-004: No distributed tracing through the execution hot path
- **Severity**: Major
- **Location**: `execution_planner/scheduler_loop.rs`, `executor/thread_task_executor.rs`, `runner/default_runner/services.rs`
- **Confidence**: High
- **Description**: Only 2 `info_span!` calls in entire core library (both in service startup). Individual workflow executions, task dispatches, and task executions are not traced. Cannot answer "why did workflow X take 45 seconds?" using trace data.
- **Suggested Resolution**: Add `#[tracing::instrument]` or explicit spans at key lifecycle boundaries: `schedule_workflow_execution`, per-task dispatch, `ThreadTaskExecutor::execute`, `complete_pipeline`.

#### OPS-005: Docker image has no HEALTHCHECK or STOPSIGNAL
- **Severity**: Minor
- **Location**: `docker/Dockerfile`
- **Confidence**: High
- **Description**: No `HEALTHCHECK` or `STOPSIGNAL` directive. Docker-compose production config has health check only for PostgreSQL, not cloacina.
- **Suggested Resolution**: Add `STOPSIGNAL SIGTERM` and `HEALTHCHECK` using `curl` or `wget` against `/health`.

#### OPS-006: Request-ID does not propagate to background scheduler/executor
- **Severity**: Minor
- **Location**: `cloacinactl/src/commands/serve.rs` lines 271-292, `runner/default_runner/services.rs`
- **Confidence**: High
- **Description**: HTTP request-ID span does not propagate to scheduler/executor background tasks. Cannot correlate HTTP request with resulting execution logs.
- **Suggested Resolution**: Store request/correlation ID in pipeline execution metadata or initial context.

#### OPS-007: Database URL logging inconsistencies
- **Severity**: Minor
- **Location**: `cloacinactl/src/commands/serve.rs` line 124, `logging.rs` lines 211-220
- **Confidence**: High
- **Description**: Server masks database URL; daemon does not (though SQLite URLs typically have no credentials). `mask_db_url` uses heuristic that does not handle all URL formats.
- **Suggested Resolution**: Apply masking consistently. Use `url` crate for robust credential detection.

#### OPS-008: No runbook documentation for common operational tasks
- **Severity**: Minor
- **Location**: Project-wide absence
- **Confidence**: High
- **Description**: No guidance for draining nodes, cleaning up old data, rotating API keys, recovering from split-brain, or debugging stuck workflows.
- **Suggested Resolution**: Create operations guide covering monitoring setup, failure scenarios, data retention, key rotation, troubleshooting.

**Notable Positive Patterns / Observations**:
- OPS-009: SIGHUP configuration reload is a strong operational feature (live config reload without restart)
- OPS-010: Graceful shutdown with timeout and force-exit is well-implemented (correct ordering, configurable timeout)
- OPS-011: Bootstrap key written with proper permissions (0600, never logged)
- OPS-012: Server TLS warning is operationally appropriate
- OPS-013: Error responses include machine-readable error codes

---

### 7. Security

**Assessment**: Deliberately security-conscious design with Ed25519 package signing, AES-256-GCM key encryption, SHA-256 API key hashing, and schema-based tenant isolation. However, multiple security features are implemented but not activated: WebSocket tickets exist as dead code, signature verification is disabled with no config to enable it, and tenant isolation is not enforced at the DAL layer.

#### SEC-001: WebSocket endpoints accept long-lived API keys in URL query parameters
- **Severity**: Major
- **Location**: `cloacinactl/src/server/ws.rs` lines 41-63, `server/keys.rs` lines 239-253
- **Confidence**: High
- **Description**: Single-use ticket mechanism exists but is never consumed. WebSocket handlers accept raw API keys via `token` query parameter. URLs logged by proxies, browsers, and monitoring tools leak long-lived credentials.
- **Evidence**: `WsTicketStore::consume()` is never called in `ws.rs`. `extract_ws_token` validates raw API key hash, not ticket.
- **Suggested Resolution**: Change `extract_ws_token` to consume tickets. Reject raw API keys in query parameters.

#### SEC-002: Package signature verification disabled by default in server mode
- **Severity**: Major (elevated to Critical in cross-cutting analysis -- see CLF-05)
- **Location**: `cloacinactl/src/commands/serve.rs` line 181, `security/verification.rs` lines 36-44
- **Confidence**: High
- **Description**: `SecurityConfig::default()` sets `require_signatures: false`. No config path to enable it. Upload handler has TODO comment and blanket-rejects when enabled. No mode where packages are both accepted AND verified.
- **Suggested Resolution**: Add `--require-signatures` CLI flag. Implement actual verification at upload. Default to `true` for `serve` mode.

#### SEC-003: API key cache creates revocation delay window
- **Severity**: Major
- **Location**: `cloacinactl/src/server/auth.rs` lines 57-117
- **Confidence**: High
- **Description**: 30-second TTL cache. Revocation clears only the local instance's cache. Multi-instance deployments accept revoked keys for up to 30 seconds on other instances.
- **Suggested Resolution**: Reduce TTL to 5 seconds, add database-polling cache-bust, or document the revocation delay.

#### ~~SEC-004~~: Rate limiting — intentionally removed
- **Severity**: N/A (withdrawn)
- Rate limiting was evaluated and intentionally removed because it degraded normal throughput. The `tower_governor` dependency and `TOO_MANY_REQUESTS` error variant are vestigial code that should be cleaned up. API keys are 256-bit random (brute-force impractical). This is a conscious design decision.

#### SEC-005: Server runs without TLS by default
- **Severity**: Major
- **Location**: `cloacinactl/src/commands/serve.rs` line 126, `main.rs` line 62
- **Confidence**: High
- **Description**: Default bind `0.0.0.0:8080` exposes server on all interfaces without TLS. API keys, tokens, and tenant passwords transmitted in cleartext.
- **Suggested Resolution**: Default to `127.0.0.1:8080`. Add optional native TLS. Refuse `0.0.0.0` without TLS or explicit `--allow-plaintext`.

#### SEC-006: Tenant data access not enforced at the DAL layer
- **Severity**: Major (elevated to Critical in cross-cutting analysis -- see CLF-05)
- **Location**: `cloacinactl/src/server/executions.rs` lines 96-134, `server/workflows.rs` lines 112-154
- **Confidence**: High
- **Description**: Tenant access check performed in handlers, but DAL queries do not filter by tenant. Single `DefaultRunner` shared across all tenant-scoped routes. Tenant-scoped key sees global data.
- **Evidence**: `dal.workflow_execution().get_active_executions()` has no tenant filter. Single runner at `serve.rs` line 157.
- **Suggested Resolution**: Create per-tenant runners, add tenant filtering to DAL, or use `SET search_path` per request.

#### SEC-007: Unsafe FFI Send/Sync implementations without memory safety audit
- **Severity**: Minor
- **Location**: `computation_graph/packaging_bridge.rs:51-54`, `registry/loader/task_registrar/dynamic_task.rs:41-44`, `python/task.rs:129-130`
- **Confidence**: Medium
- **Description**: Multiple types implement `unsafe impl Send/Sync` vouching for thread-safety of FFI-loaded native code that may contain thread-unsafe global state.
- **Suggested Resolution**: Document thread-safety requirements for packaged workflows. Consider per-plugin thread isolation.

#### SEC-008: WsTicketStore does not bound ticket count or expire old tickets
- **Severity**: Minor
- **Location**: `cloacinactl/src/server/auth.rs` lines 269-308
- **Confidence**: High
- **Description**: Unbounded `HashMap`. Expired tickets only removed when consumed (never, since consumption is dead code). Repeated calls can grow memory without limit.
- **Suggested Resolution**: Use `LruCache` with fixed capacity. Add periodic eviction.

#### SEC-009: Bootstrap key written to disk without cleanup mechanism
- **Severity**: Minor
- **Location**: `cloacinactl/src/commands/serve.rs` lines 498-550
- **Confidence**: High
- **Description**: Bootstrap admin key persists on disk indefinitely. No rotation, deletion after retrieval, or "file still exists" warning.
- **Suggested Resolution**: Print to stderr instead, or add rotation command and startup warning.

#### SEC-010: CLOACINA_VAR_* environment variables may contain secrets without protection
- **Severity**: Minor
- **Location**: `var.rs`
- **Confidence**: Medium
- **Description**: No distinction between sensitive and non-sensitive variables. Debug logging could expose secrets. `VarNotFound` errors reveal expected secret names.
- **Suggested Resolution**: Add `CLOACINA_SECRET_*` prefix convention with masking in logs and error messages.

#### SEC-011: Metrics endpoint exposed without authentication
- **Severity**: Minor
- **Location**: `cloacinactl/src/commands/serve.rs` lines 399-402
- **Confidence**: High
- **Description**: `/metrics` in unauthenticated route group. Exposes operational details (execution counts, error patterns, timing).
- **Suggested Resolution**: Move behind authentication or expose on separate internal port.

**Notable Positive Patterns / Observations**:
- SEC-012: Database URL masking in logs is properly implemented
- SEC-013: Tenant SQL uses thorough identifier validation (alphanumeric + underscore, reserved name checks)
- SEC-014: Security audit logging is comprehensive and SIEM-compatible (14 audit functions, structured fields)
- SEC-015: Cryptographic implementation uses standard, modern algorithms (Ed25519, AES-256-GCM, SHA-256)

---

## Cross-Cutting Concerns

### Root Causes

**RC-1: Incomplete terminology migration.** The codebase renamed "pipeline" to "workflow" at the API layer but did not propagate to the database schema, internal models, error messages, or Python bindings. Root cause of LEG-001, LEG-014, API-002, API-008. Affects OPS-003.

**RC-2: No backend abstraction in the DAL.** PostgreSQL and SQLite treated as completely separate code paths joined only by a routing macro. No trait, generic, or shared implementation body. Root cause of LEG-002, EVO-003, COR-007. Contributes to EVO-007, PERF-006.

**RC-3: Monolithic core crate with global mutable state.** Single crate with 9+ global statics, 21 public modules, 43 dependencies. Root cause of EVO-001, EVO-002, EVO-004, LEG-004, LEG-003. Contributes to OPS-004, SEC-006.

**RC-4: Security features implemented but not activated.** Ticket system, signature verification, and tenant isolation exist as code but are not wired into the default runtime path. Root cause of SEC-001. Contributes to SEC-002, SEC-006. (Note: rate limiting was intentionally removed due to throughput impact — the vestigial `tower_governor` dependency and dead `TOO_MANY_REQUESTS` error variant should be cleaned up.)

### Severity Adjustments

| Finding(s) | Original | Adjusted | Rationale |
|-------------|----------|----------|-----------|
| LEG-001 + API-002 + API-008 | Major each | **Critical** (combined) | Affects every consumer surface and every operational scenario. Combined impact on mean-time-to-diagnosis exceeds any single-lens assessment. |
| COR-001 + COR-004 | Major each | **Critical** (combined) | Double state updates + completion races corrupt audit trails and produce incorrect final context. No metrics to detect (OPS-002), making them invisible in production. |
| SEC-002 + SEC-006 | Major each | **Critical** (combined) | Unsigned code execution + no tenant DAL isolation = any write-scoped key can execute arbitrary code with access to all tenants' data. |
| COR-007 | Minor | **Major** | Context merge divergence is a symptom of dual-backend pattern (RC-2). Structurally likely to recur and grow. |

### Systemic Patterns

**SP-1: Implementation exists but integration is missing.** WebSocket tickets, rate limiting, metrics, config validation, signature verification, Python `start()`/`stop()` -- all built at the component level, not wired into the runtime path.

**SP-2: Positive patterns that should be extended.** DAL accessor pattern, extension traits, `var.rs` module design, audit logging, graceful shutdown -- exemplary patterns that should serve as templates for future work.

**SP-3: Computation graph system is a parallel universe.** Separate scheduler, global registry, packaging bridge, health state machines. Two maintenance surfaces for similar problems. Not necessarily wrong, but increases maintenance burden.

**SP-4: Test architecture is structurally limited.** Global state requires `#[serial]`. No mock DAL. Database fixtures use shared singletons. Critical paths (COR-001, COR-004, COR-005) cannot be tested without structural changes.
