# Cross-Cutting Analysis

## Summary

Three root causes explain the majority of findings across all seven review lenses: (1) an incomplete terminology migration from "pipeline" to "workflow" that damages legibility, API consistency, and operability simultaneously; (2) a dual-backend persistence architecture that doubles code volume, blocks evolvability, and creates correctness risk through unsynchronized duplicate implementations; and (3) a monolithic core crate acting as a god object, which concentrates global mutable state, impedes test isolation, and makes it structurally difficult to enforce security boundaries or add observability instrumentation. Addressing these three leverage points would resolve or mitigate roughly 60% of all findings across the review.

## Cross-Lens Findings

### CLF-01: "Pipeline" vs "Workflow" naming collision (Legibility + API Design + Operability)
**Lenses**: LEG-001, LEG-014, API-002, API-008, OPS-003
**Relationship**: Shared root cause -- incomplete terminology migration

This is the most pervasive single issue in the codebase. It manifests differently in each lens:
- **Legibility** (LEG-001, LEG-014): Internal code mixes terms, creating confusion about whether "pipeline" and "workflow" are different concepts. 539 occurrences of `pipeline_exec` vs 35 of `workflow_exec`.
- **API Design** (API-002): Python users see `pipeline_timeout_seconds` in the config API while everything else says "workflow." API-008: Error messages displayed to all consumers say "Pipeline execution failed" from a type named `WorkflowExecutionError`.
- **Operability** (OPS-003): Invalid config values using legacy terminology are silently accepted because validation does not exist. Operators reading logs see "Pipeline" errors alongside "Workflow" status messages.

The combined impact is greater than any single lens suggests. A user debugging a failed workflow must mentally translate between "pipeline" in database tables, error messages, and internal logs, and "workflow" in the API, documentation, and CLI. This is not just a cosmetic issue -- it increases mean time to diagnosis during incidents.

**Severity adjustment**: Individually rated Major in each lens. The combined cross-cutting impact warrants **Critical** severity, as it affects every consumer surface (Rust, Python, HTTP, logs, database) and every operational scenario (debugging, monitoring, incident response).

### CLF-02: Dual-backend DAL duplication (Legibility + Evolvability + Correctness + Performance)
**Lenses**: LEG-002, EVO-003, EVO-007, COR-007, PERF-006
**Relationship**: Shared root cause -- no backend abstraction in the persistence layer

The `dispatch_backend!` macro and paired `_postgres`/`_sqlite` methods create cascading problems:
- **Legibility** (LEG-002): Every DAL method must be read twice to verify behavior. 1152-line files where half is near-identical duplication.
- **Evolvability** (EVO-003, EVO-007): Adding a new entity requires 6+ files with dual implementations. Adding a third database backend (EVO-003) would require 45-120 new method implementations. Feature flag combinatorics (EVO-007) create 4 separate `#[cfg]` blocks in migration code.
- **Correctness** (COR-007): The two backends have silently diverged in context merge behavior -- `ThreadTaskExecutor` uses smart merging (array concatenation, recursive object merge), while `ContextManager` uses simple overwrite. This means trigger rule evaluation sees different merged data than the task itself.
- **Performance** (PERF-006): `StateManager` and `ContextManager` independently fetch the same pipeline and workflow records because the dual-backend structure discourages sharing state across DAL boundaries.

**Severity adjustment**: The correctness divergence (COR-007) is currently rated Minor because it only affects trigger-rule evaluation. In combination with the duplication pattern that makes such divergences structurally likely to recur, the systemic risk warrants **Major** severity for the dual-backend pattern as a whole.

### CLF-03: Double state update path (Correctness + Performance + Operability)
**Lenses**: COR-001, COR-004, PERF-001, OPS-002
**Relationship**: Cause/effect chain -- unclear ownership of state transitions

The executor and dispatcher both write task completion status, creating:
- **Correctness** (COR-001): Duplicate `mark_completed` calls produce duplicate execution events and misleading timestamps. For failures, the dispatcher could overwrite a retry status set by the executor. COR-004: The scheduler can mark a pipeline complete while the executor is still writing the last task's context.
- **Performance** (PERF-001): The N+1 query pattern in `update_pipeline_final_context` compounds the race -- loading metadata per-task during a window where the executor may still be writing means stale reads.
- **Operability** (OPS-002): The phantom metrics (gauges and histograms that are described but never recorded) are partly explained by the ambiguous ownership -- it is unclear whether the executor, dispatcher, or scheduler should record duration histograms, because all three touch the same state transitions.

**Severity adjustment**: COR-001 and COR-004 are both Major. Their combination with the operability gap (no metrics to detect double-writes or race conditions) makes this a **Critical** operational risk in production, as duplicate events corrupt audit trails and completion races can produce incorrect final context.

### CLF-04: Unsafe Send/Sync on FFI/Python wrappers (Correctness + Security + Evolvability)
**Lenses**: COR-003, SEC-007, EVO-009
**Relationship**: Shared root cause -- Python/FFI bindings tightly coupled to core engine

Four struct types use `unsafe impl Send/Sync` with convention-based safety arguments:
- **Correctness** (COR-003): The safety invariant ("ALL access goes through with_gil()") is enforced by code review, not by the type system. No concurrent stress tests validate this.
- **Security** (SEC-007): FFI-loaded plugins (`LoadedWorkflowPlugin`, `LoadedGraphPlugin`) are vouched as thread-safe, but user-provided native code may contain thread-unsafe global state.
- **Evolvability** (EVO-009): Python bindings live inside the core crate and directly reference internal types. Any refactor that adds PyObject access to a method that currently does not touch it could silently break the safety invariant.

**Severity adjustment**: Individually rated Major (COR-003) and Minor (SEC-007, EVO-009). The combination does not change the severity -- the risk is real but mitigated by the small number of unsafe impls (4 structs) and the current discipline of GIL usage. However, the evolvability dimension means the risk grows over time as the codebase evolves.

### CLF-05: Security defaults favor developer convenience over production safety (Security + Operability + API Design)
**Lenses**: SEC-002, SEC-005, OPS-001, OPS-003, API-003
**Relationship**: Shared root cause -- defaults optimized for development, not deployment

Multiple security-relevant defaults create a "secure by effort, not by default" posture:
- **Security** (SEC-002): Package signature verification disabled by default, with no config path to enable it. SEC-005: Server binds to 0.0.0.0 without TLS.
- **Operability** (OPS-001): Daemon has no health check surface. OPS-003: No configuration validation catches dangerous values.
- **API Design** (API-003): Config builder panics instead of returning errors, meaning a zero-value config crashes the application rather than producing a diagnostic.

The pattern is consistent: every security/safety feature requires explicit opt-in, and the defaults are permissive. An operator who deploys with defaults gets: no TLS, no signature verification, no health checks for daemon mode, and a configuration that silently accepts dangerous values. (Note: rate limiting was evaluated and intentionally removed due to throughput impact — this is a conscious decision, not a gap.)

**Severity adjustment**: SEC-002 (signature verification disabled) combined with SEC-006 (tenant data not isolated at DAL layer) is particularly dangerous -- an authenticated user with write access can upload unsigned native code that executes in the server process with access to all tenants' data. This combination warrants **Critical** severity.

### CLF-06: WebSocket security implementation gap (Security + API Design)
**Lenses**: SEC-001, SEC-008, API-004
**Relationship**: Cause/effect -- ticket system implemented but not connected

The WebSocket security story has a complete implementation that is never activated:
- **Security** (SEC-001): The single-use ticket mechanism exists (`WsTicketStore`, `POST /auth/ws-ticket`) but WebSocket handlers accept raw API keys in URL query parameters instead. SEC-008: The ticket store is unbounded and has no cleanup, but since tickets are never consumed, this is both a memory leak and dead code.
- **API Design** (API-004): The documented API paths differ from actual routes, and the WebSocket authentication model is undocumented because the intended model (tickets) is not the actual model (raw keys in query params).

**Severity adjustment**: SEC-001 remains Major. The dead-code nature of the ticket system means the security intent exists but is not realized -- this is a more tractable fix than designing from scratch.

## Root Causes

### RC-1: Incomplete terminology migration

The codebase underwent a conceptual rename from "pipeline" to "workflow" at the API layer but did not propagate it to the database schema, internal models, error messages, or Python bindings. This single incomplete migration is the root cause of LEG-001, LEG-014, API-002, API-008, and contributes to OPS-003 and the general legibility overhead throughout the codebase.

**Leverage**: A database migration renaming `pipeline_executions` to `workflow_executions` (with an alias view for backward compatibility) plus a find-and-replace of `pipeline_` to `workflow_` in model fields and error messages would resolve 5 findings across 3 lenses. This is high-leverage, moderate-effort work.

### RC-2: No backend abstraction in the DAL

The persistence layer treats PostgreSQL and SQLite as two completely separate code paths joined only at the routing macro level. There is no trait, generic parameter, or shared implementation body that captures their common logic. Every entity requires dual implementation, every feature change requires dual modification, and divergences between the two paths are structurally invisible.

**Leverage**: Introducing a `DalConnection` trait or using Diesel's `MultiConnection` to collapse paired methods into single implementations would resolve LEG-002, EVO-003, COR-007, and reduce the surface area for EVO-007 and PERF-006. This is high-leverage, high-effort work (touching ~15 DAL modules), but the codebase is already structured to support it -- the business logic inside each method pair is identical.

### RC-3: Monolithic core crate with global mutable state

The `cloacina` crate is simultaneously a persistence library, execution engine, packaging system, Python binding module, and computation graph runtime. It holds 9+ process-global static registries, exports 70+ symbols at the crate root, and builds as both `lib` and `cdylib`. This monolith is the root cause of:
- EVO-001 (monolithic crate), EVO-002 (global statics require `#[serial]`), EVO-004 (DefaultRunner as god object)
- LEG-004 (excessively large re-export surface), LEG-003 (multiple "Scheduler" concepts without disambiguation)
- The structural difficulty of adding tracing instrumentation (OPS-004) or enforcing security boundaries (SEC-006)

**Leverage**: Splitting into `cloacina-dal`, `cloacina-engine`, `cloacina-python`, and `cloacina-security` crates would enforce boundary discipline, reduce compile times, enable independent testing, and make the god-object pattern structurally impossible. This is very high-leverage but also very high-effort -- it is a multi-sprint architectural project. A pragmatic first step is extending the `Runtime` struct to cover all registries (not just tasks/workflows/triggers), which would resolve EVO-002 without crate splitting.

### RC-4: Security features implemented but not activated

The codebase contains implementations for package signature verification, WebSocket single-use tickets, and tenant schema isolation -- but none are wired into the default runtime path. The pattern suggests a development process where security features are built in isolation and then not integrated into the main flow. (Note: rate limiting was previously in this category but was intentionally removed after evaluation showed it degraded normal throughput.)

**Leverage**: Wiring existing implementations (ticket consumption in WebSocket handlers, per-tenant DAL connections) would resolve SEC-001 and SEC-006 with relatively low effort since the code already exists.

## Tensions

### T-1: Developer convenience vs production safety (conscious, undertensioned)

The system defaults to permissive configuration (no TLS, no signatures, `usize::MAX` catchup executions) to minimize friction for developers getting started. This is a valid tradeoff -- embedded libraries should be easy to adopt. However, the tension is undertensioned: there is no "production mode" flag, no deployment checklist, and no runtime warnings beyond the TLS message. The gap between "works in development" and "safe in production" requires the operator to discover and enable each safety feature independently.

**Recommendation**: Add a `--production` flag or `CLOACINA_ENV=production` environment variable that enables secure defaults (require TLS or explicit opt-out, enable signature verification, set reasonable catchup limits, validate configuration). This makes the tradeoff explicit rather than implicit.

### T-2: Dual-backend support vs code maintainability (conscious, costly)

Supporting both PostgreSQL and SQLite serves a legitimate purpose: SQLite for development and testing, PostgreSQL for production. However, the implementation choice (full code duplication rather than abstraction) imposes a high ongoing cost. Every DAL change is twice the work, correctness divergences are invisible, and a third backend is practically impossible.

**Recommendation**: The tradeoff itself is sound, but the implementation approach should change. Diesel's `MultiConnection` or a custom `DalConnection` trait would preserve the benefit (both backends work) while eliminating the cost (duplicate code).

### T-3: Macro ergonomics vs debuggability (conscious, acceptable)

The procedural macros (`#[task]`, `#[workflow]`, `#[computation_graph]`) provide an ergonomic API with compile-time validation (cycle detection, missing dependencies). The cost is opacity -- macro-generated code is harder to debug, `#[ctor]` auto-registration creates hidden initialization order dependencies, and the global task registry is implicitly populated.

**Recommendation**: This tradeoff is acceptable for the project's goals. The `Runtime` struct partially addresses the registry problem. The macro API is well-documented and the compile-time validation catches the most common errors. No change recommended.

### T-4: Embedded library simplicity vs observability depth (unconscious)

The architecture provides minimal tracing instrumentation (2 spans in the entire core library) and records only 2 of 7 declared metrics. This is partly because an embedded library does not control the application's observability stack. However, the `cloacinactl serve` mode IS a standalone service, and it has the same observability gaps. The tension between "lightweight embedded library" and "observable production service" appears unresolved rather than consciously traded off.

**Recommendation**: Instrument the core library with spans gated behind a feature flag (e.g., `tracing` feature). This lets embedded users opt out of overhead while giving `cloacinactl serve` users full observability. The phantom metrics should either be removed (reducing false confidence) or implemented (providing actual signal).

### T-5: Schema-based tenant isolation vs server architecture (unconscious)

Multi-tenancy is designed around PostgreSQL schema isolation -- each tenant gets its own schema with separate tables. However, the server creates a single `DefaultRunner` with a single database connection pool pointed at one schema. Tenant-scoped HTTP endpoints check authorization but then query the single shared schema. The intended isolation model (schemas) and the actual server architecture (single runner) are in tension, and SEC-006 is the result.

**Recommendation**: Either create per-tenant runners (resource-heavy) or use `SET search_path` at the connection level before each tenant-scoped request (requires connection-level state management). This tension must be resolved before multi-tenant deployments are safe.

## Systemic Patterns

### SP-1: Implementation exists but integration is missing

A recurring pattern across the codebase: components are built in isolation with correct internal logic, but the wiring between components is incomplete or missing.

- WebSocket ticket system: built, never consumed (SEC-001)
- Metrics: 7 described, 2 recorded (OPS-002)
- Config validation: `assert!` in builder but no validation on TOML load (OPS-003, API-003)
- Signature verification: verification pipeline exists, upload handler has TODO (SEC-002)
- Python `start()`/`stop()`: methods defined, raise at runtime (API-001)

This suggests a development cadence where individual features are shipped to "done at the unit level" but the integration step is deferred or forgotten. A pre-release checklist that includes "all declared interfaces are connected" would catch this pattern.

### SP-2: Positive patterns that should be extended

Several well-designed patterns exist in parts of the codebase that should be applied more broadly:

- **DAL accessor pattern** (LEG-008, API-013): `dal.task_execution().mark_completed(id)` is exemplary. The same pattern should be applied to create a trait-based DAL (EVO-005) to enable testing without databases.
- **Extension traits** (EVO-010): `TaskExecutor`, `Dispatcher`, `WorkflowRegistry` are clean trait boundaries. The same discipline should be applied to the DAL and to background service management.
- **`var.rs` module** (LEG-011): Concise, self-documenting, thoroughly tested. Other small utility modules should follow this template.
- **Audit logging** (SEC-014): Structured, SIEM-compatible, comprehensive for the security domain. Should be extended to HTTP-layer auth events and operational events.
- **Graceful shutdown** (OPS-010): Well-implemented with timeout, force-exit, and correct ordering. Should be documented in an operations runbook.

### SP-3: Computation graph system is a parallel universe

The computation graph system (v0.5.0) operates as a largely independent subsystem with its own scheduler (`ReactiveScheduler`), global registry, packaging bridge, health state machines, and WebSocket endpoints. It shares little infrastructure with the workflow system beyond the `DefaultRunner` and the reconciler.

This creates two maintenance surfaces for similar problems:
- Two global registries (only workflows/tasks/triggers are covered by `Runtime`; CG is not)
- Two shutdown coordination paths
- Two health reporting mechanisms
- Two sets of test fixtures

This is not necessarily wrong -- the two systems have genuinely different execution models (DAG vs reactive). But the parallel evolution increases the maintenance burden and creates opportunities for inconsistency.

### SP-4: Test architecture is structurally limited

161 uses of `#[serial]` across 24 test files reflect the global mutable state problem (RC-3). Integration tests require a running PostgreSQL instance. The `cloacina-testing` crate provides database-free testing but only covers pure task logic. The most critical paths (scheduler-executor interaction, concurrent claiming, pipeline completion races) have no test coverage, and the test architecture makes such tests difficult to write because:
- Global state requires serialization
- No mock DAL exists (concrete `DAL` type, no trait)
- Database fixtures use shared singletons

This means the correctness findings (COR-001 double state update, COR-004 completion race, COR-005 heartbeat ClaimLost) are not just missing tests -- the test infrastructure would need structural changes to support them.

## Severity Adjustments

| Finding | Original Severity | Adjusted Severity | Rationale |
|---------|------------------|-------------------|-----------|
| LEG-001 + API-002 + API-008 | Major (each) | **Critical** (combined) | Affects every consumer surface and every operational scenario. Combined impact on mean-time-to-diagnosis during incidents exceeds any single-lens assessment. |
| COR-001 + COR-004 | Major (each) | **Critical** (combined) | Double state updates and completion races can corrupt audit trails and produce incorrect final context. No metrics exist to detect these conditions (OPS-002), making them invisible in production. |
| SEC-002 + SEC-006 | Major (each) | **Critical** (combined) | Unsigned code execution + no tenant isolation at DAL layer = any write-scoped key can execute arbitrary code with access to all tenants' data. |
| COR-007 | Minor | **Major** | Context merge divergence between executor and scheduler is a symptom of the dual-backend pattern (RC-2). The divergence is structurally likely to recur and grow after future changes. |
| SEC-001 | Major | Major (no change) | Severity remains Major. The existence of dead-code ticket system means the fix is tractable (wiring, not design), but the current exposure (API keys in URLs) is genuinely dangerous. |
| PERF-004 + OPS-003 | Major (each) | Major (no change) | The `usize::MAX` default for catchup executions appears in both performance and operability reviews. These are two views of the same finding, not two separate issues. Severity is correctly Major in both lenses. |
| OPS-001 | Critical | Critical (no change) | Daemon with no health check surface is correctly Critical for any containerized deployment. |
