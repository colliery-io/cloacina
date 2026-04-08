# Cross-Cutting Analysis

## Summary

Three architectural root causes explain the majority of findings across all lenses: (1) the Pipeline/Workflow naming schism permeates every interface surface and compounds confusion from legibility through API design to operability, (2) the global mutable registry pattern drives test serialization, prevents multi-tenant isolation, and makes the monolithic core crate resistant to decomposition, and (3) the server's authorization and tenant isolation are structurally incomplete, creating a cluster of critical security findings that also undermine the multi-tenant operability story. The codebase shows a clear maturity gradient -- the computation graph subsystem and daemon lifecycle management are production-grade, while the HTTP API layer and multi-tenant server mode have significant gaps across security, API design, and operability.

## Cross-Lens Findings

### CLF-01: Pipeline/Workflow Terminology Drift (LEG-01, API-01, API-06, COR-01)
**Lenses affected**: Legibility, API Design, Correctness, Operability
**Relationship**: Shared root cause -- a naming decision made early that was never reconciled as the system grew.

LEG-01 identifies the naming confusion. API-01 shows it leaking through every consumer surface (Rust types, REST responses, Python bindings). API-06 notes the prelude exports both vocabularies side-by-side. COR-01 -- the most severe correctness finding -- is entangled with this: `PipelineExecution` is always marked "Completed" regardless of task failure status, and the confusing naming makes it harder to spot that the status logic is wrong because the reader must first untangle whether "Pipeline" and "Workflow" completion mean different things (they do not).

From an operability perspective, log messages mix the two terms, meaning an operator searching logs for "workflow failed" will not find the relevant "pipeline completed" entry. The combined impact across four lenses elevates this from a legibility nuisance to a systemic issue that affects debugging, correctness, and API consumer trust.

**Severity assessment**: The individual legibility finding is Major. Combined with COR-01 (Critical) and the API surface confusion, the naming drift should be treated as a high-priority cleanup that unblocks fixing the completion status bug.

### CLF-02: Error Handling -- Lossy Conversions and Silent Swallowing (LEG-05, COR-04, COR-11, EVO-04)
**Lenses affected**: Legibility, Correctness, Evolvability
**Relationship**: Shared root cause -- the error type hierarchy was designed for the authoring crate (`cloacina-workflow`) and then force-fitted to the runtime crate's richer error landscape.

LEG-05 and COR-04 identify the same `ContextError::Database -> KeyNotFound` lossy conversion. COR-11 shows the parallel pattern: dependency context load failures are silently swallowed with `if let Ok(...)`. EVO-04 describes the broader fragmentation (14-variant `ValidationError`, manual `From` conversions across 3 crates). These are all symptoms of a single architectural gap: `cloacina-workflow` defines the error types that cross the plugin FFI boundary, but it lacks variants for infrastructure errors (database, connection pool, I/O). The runtime crate must shoehorn these into existing variants or drop them entirely.

The combined impact is that infrastructure failures become invisible or misclassified, making production debugging harder (operability) and enabling tasks to proceed with incomplete data (correctness).

**Severity assessment**: Individually Minor. Collectively, this cluster justifies adding infrastructure error variants to `cloacina-workflow::ContextError` -- a small cross-crate change that resolves findings in three lenses simultaneously.

### CLF-03: SchedulerLoop Shutdown Gap (COR-03, OPS-04, OPS-08)
**Lenses affected**: Correctness, Operability
**Relationship**: Same finding observed through different lenses.

COR-03 flags the missing shutdown channel as a correctness issue (infinite loop, no clean termination). OPS-04 restates it from the operator's perspective (interrupted database operations, no shutdown logging). OPS-08 adds that the server's graceful shutdown path does not call `runner.shutdown()` at all, compounding the problem: even if `SchedulerLoop` had a shutdown channel, the server never signals it. The daemon correctly calls `runner.shutdown()`, showing the pattern is established but incompletely applied.

**Severity assessment**: The combined finding is Major for the server deployment and should be treated as a single work item: add shutdown channel to `SchedulerLoop` AND call `runner.shutdown()` in the server's shutdown sequence.

### CLF-04: Credential Exposure Across Multiple Surfaces (OPS-03, SEC-06, SEC-08, SEC-15)
**Lenses affected**: Security, Operability
**Relationship**: Multiple manifestations of insufficient credential hygiene.

OPS-03: Python bindings log full database URLs with passwords. SEC-06: No TLS, so credentials traverse the network in cleartext. SEC-08: Tenant passwords returned in HTTP response bodies. SEC-15: Database URL visible via CLI arguments and `ps`. The server's `mask_db_url()` function (OPS-12, positive) shows the team is aware of the problem but has applied the fix inconsistently. The Python bindings bypass it entirely, and the network layer has no encryption.

These findings reinforce each other: even if you fix the Python logging, the lack of TLS means the credentials are still exposed in transit. The combined exposure surface is: logs (Python), network (no TLS), HTTP responses (tenant creation), and process listing (CLI args).

**Severity assessment**: The cluster collectively represents a Critical credential hygiene gap for the server deployment mode. The individual findings range from Minor to Major, but the combined attack surface -- especially with SEC-01 (any user can mint admin keys) -- creates a privilege escalation chain: intercept credentials from any of these surfaces, create an admin key, and gain full system access.

### CLF-05: Observability Gaps Compound Debugging Difficulty (OPS-01, OPS-02, API-02, COR-07, COR-01)
**Lenses affected**: Operability, API Design, Correctness
**Relationship**: Absence of observability makes correctness and API issues harder to detect and diagnose.

OPS-01 (no metrics) and OPS-02 (no distributed tracing) mean operators have no dashboards and no request correlation. API-02 (no consistent error format, no request IDs) means API consumers cannot trace failures either. COR-01 (pipeline always "Completed") means the system actively misreports status. COR-07 (stale claim sweeper over-reports) means operational logs are inaccurate.

In combination: a pipeline fails, but the status says "Completed" (COR-01). The operator checks metrics -- none exist (OPS-01). They search logs -- no request ID correlates the HTTP request to the pipeline execution (OPS-02). The API error response has no machine-readable code to act on (API-02). The stale claim count in logs is wrong (COR-07). Every layer that should help diagnose the problem is either missing or actively misleading.

**Severity assessment**: The observability gap amplifies the impact of every other finding. OPS-01 and OPS-02 should be treated as Major infrastructure prerequisites rather than independent line items.

## Root Causes

### RC-01: Global Mutable State as Architecture
**Findings explained**: EVO-02, EVO-01, EVO-10, EVO-06, PERF-11

The decision to use process-global static registries (`Lazy<Arc<RwLock<HashMap>>>` populated via `#[ctor]`) is the single most consequential architectural choice in the codebase. It explains:

- **EVO-02**: Tests require `#[serial]` (160 instances) because registries are process-global.
- **EVO-01**: The monolithic core crate cannot be decomposed because modules share global state. Extracting the executor into a separate crate would require making the global task registry a cross-crate dependency.
- **EVO-10**: Test architecture is limited -- no mock-based testing because the DAL is concrete and tied to global state.
- **EVO-06**: The DAL has no trait abstraction partly because the registries it interacts with are global singletons.
- **PERF-11**: The state manager reconstructs workflows from the global registry on every scheduler tick because there is no per-pipeline cached workflow instance.

**Leverage**: Introducing a scoped `Runtime` or `Engine` struct that owns registry instances would unblock parallel testing, enable crate decomposition, and create a natural home for per-pipeline caching. This is a high-cost change but has the highest leverage of any single improvement.

### RC-02: Multi-Tenant Server Implemented as Single-Tenant with Decorative Tenant IDs
**Findings explained**: SEC-02, SEC-01, SEC-04, API-16, OPS-08

The HTTP API presents multi-tenant URLs (`/tenants/{id}/...`) but the implementation is single-tenant:

- **SEC-02**: DAL queries do not filter by tenant. The `tenant_id` path parameter is passed through to responses but never to queries.
- **SEC-01**: Key creation has no authorization -- any authenticated user can mint admin keys, because the authorization model was never fully implemented.
- **SEC-04**: `list_tenants` has no authorization check -- same pattern of incomplete authorization.
- **API-16**: `list_executions` returns all active executions globally, not per-tenant.
- **OPS-08**: Server shutdown does not drain the runner because the server was not designed with the same lifecycle rigor as the daemon.

The architectural intent (PostgreSQL schema-based isolation, per-tenant `search_path`) is documented in ADR-1, but the implementation gap between intent and reality is the largest source of security findings. The server mode appears to be at an earlier maturity stage than the daemon mode.

**Leverage**: Implementing schema-path switching in a middleware layer (set `search_path` per-request based on authenticated tenant) would resolve SEC-02 and API-16 in one change. Completing the authorization checks (SEC-01, SEC-04) is a separate but related task.

### RC-03: Dual-Backend Strategy Produces Structural Code Duplication
**Findings explained**: EVO-03, LEG-03, LEG-11, EVO-08

The runtime database backend selection (ADR-1) requires `dispatch_backend!` macro usage at 132 call sites, three overlapping dispatch macros (LEG-03), visually identical method pairs in every DAL module (LEG-11), and a three-way `#[cfg]` feature flag matrix (EVO-08). Each finding individually is Minor, but together they represent a significant maintenance burden:

- Every schema change requires parallel modification in two backends.
- Every new DAL method requires a `_postgres` and `_sqlite` variant.
- The three dispatch macros (`dispatch_backend!`, `backend_dispatch!`, `connection_match!`) suggest the team has iterated on the pattern without converging.

**Leverage**: This is an inherent cost of the dual-backend strategy, which is a genuine product requirement (embedded SQLite vs. server PostgreSQL). The cost is manageable for two backends but would be prohibitive for three (EVO-03 change cost analysis). Consolidating to a single dispatch macro (LEG-03's recommendation) and adding explanatory comments (LEG-11's recommendation) are low-cost mitigations. A deeper fix would require Diesel generic connection abstractions, which may not be feasible with the current Diesel version.

### RC-04: Python Bindings as Parallel Implementation
**Findings explained**: EVO-05, OPS-03, API-07, API-08

The Python `PyDefaultRunner` (2,888 lines) is not a thin wrapper but a parallel implementation with its own Tokio runtime, message-passing protocol, and coordination logic. This produces:

- **EVO-05**: Every new `DefaultRunner` method requires a parallel implementation in 4 places (message variant, handler, pymethods, error mapping).
- **OPS-03**: The Python bindings log credentials because they have their own logging paths that bypass the server's `mask_db_url()` protection.
- **API-07**: Time unit inconsistency (`_ms` vs `_seconds`) because the Python config surface was designed independently.
- **API-08**: Context manager `__exit__` creates a fresh workflow that drops description/tags -- a bug that exists because the Python builder reimplements workflow construction rather than delegating.

**Leverage**: Extracting the Tokio runtime bridge into a reusable module (`runtime_bridge.rs`) and making the Python bindings thin wrappers over the Rust API (via `pyo3-asyncio` or similar) would reduce the maintenance surface and prevent the divergence that causes OPS-03 and API-08.

## Tensions

### T-01: Performance vs. Crash Recovery (Appropriate Tradeoff)
**PERF-01** (reactor persists state on every execution) vs. **COR-09** (sequential reactor queue has a crash-loss window).

The system errs on the side of durability by persisting after every graph execution, accepting the I/O overhead. The correctness review notes that even with this aggressive persistence, a crash between `pop_front()` and the next persist can lose one boundary. This is a conscious and appropriate tradeoff for a workflow system -- durability is more important than throughput, and the comment in the code acknowledges the gap. No change needed; the tradeoff is sound.

### T-02: Security vs. Usability -- Signature Verification Default (Inappropriate Tradeoff)
**SEC-03** (signatures off by default) vs. the stated goal of enabling easy development setup.

The `SecurityConfig::development()` constructor correctly disables signatures for local development. But the server-mode default also has signatures disabled, which is inappropriate for a network-accessible service that loads arbitrary native code. The development convenience should not apply to production deployment. The resolution is to have `cloacinactl serve` default to `require_signatures: true` while keeping the library/daemon defaults permissive.

### T-03: Evolvability vs. Simplicity -- DAL Trait Abstraction (Conscious Non-Decision)
**EVO-06** (no DAL trait) argues for a trait-based DAL to enable mocking and decorator patterns. However, introducing traits for 15+ DAL sub-modules with 100+ methods would add significant complexity to a system that already has a dual-backend dispatch layer. The current concrete DAL is simpler to navigate and refactor (no trait coherence issues, no generic lifetime puzzles).

The tension is real: mock-based testing vs. API surface complexity. Given that the integration test infrastructure works (with `#[serial]`), the current tradeoff is reasonable for the team's size and velocity. A partial approach -- trait-abstracting only the most-tested DAL modules (task execution, pipeline execution) -- would capture most of the testing benefit with lower cost.

### T-04: Legibility vs. Architectural Necessity -- dispatch_backend! Macro (Appropriate Tradeoff)
**LEG-03** and **LEG-11** note that the backend dispatch pattern produces visual duplication and cognitive load. But the duplication is a necessary consequence of Diesel's type system, which requires separate connection types for PostgreSQL and SQLite. The macro encapsulates the branching correctly; the alternative (trait-based generic DAL) would require Diesel features that may not exist. The tradeoff is appropriate, but the ancillary macros (`backend_dispatch!`, `connection_match!`) should be cleaned up as LEG-03 recommends.

## Systemic Patterns

### SP-01: Maturity Gradient Between Subsystems
The codebase shows a clear maturity gradient:

| Subsystem | Maturity | Evidence |
|-----------|----------|----------|
| Computation graph | Production-grade | Circuit breaker, backoff, health endpoints, state persistence, supervision loop (OPS-13) |
| Daemon lifecycle | Production-grade | SIGHUP reload, graceful shutdown, force-exit, configurable timeout (OPS-10, OPS-14) |
| Core executor/scheduler | Solid with gaps | Batch queries, outbox pattern, but COR-01 (completion status), COR-02 (non-atomic completion) |
| Macro/builder API | Well-designed | Low ceremony, compile-time validation, good Python parity (API-09, API-10) |
| REST API | Early stage | No versioning, no error envelope, no pagination, decorative tenant IDs (API-02, API-03, SEC-02) |
| Server authorization | Incomplete | Missing checks on 2 of 6 endpoints, no per-tenant query filtering (SEC-01, SEC-02, SEC-04) |
| Observability | Placeholder | Static metrics endpoint, no tracing, no correlation IDs (OPS-01, OPS-02) |

The pattern suggests the team built outward from the core engine (which is mature) and the newest subsystem (computation graphs, which is well-designed) while the HTTP API layer connecting them to external consumers received less hardening attention. The daemon, which came earlier, got more operational polish than the server, which was added later.

### SP-02: Inconsistent Application of Good Patterns
Several findings result from a good pattern being applied in one place but not another:

- `mask_db_url()` exists in `serve.rs` but is not used in Python bindings (OPS-03)
- Shutdown channels exist in `StaleClaimSweeper` and `UnifiedScheduler` but not in `SchedulerLoop` (COR-03/OPS-04)
- Circuit breaker exists in `ReactiveScheduler` but not in the scheduler loop (OPS-05 vs OPS-13)
- Authorization checks exist on `create_tenant`, `remove_tenant`, `revoke_key` but not on `create_key`, `list_tenants` (SEC-01, SEC-04)
- Batch DB queries exist for dependency checking but not for trigger condition evaluation (PERF-02)
- API versioning (`/v1/`) exists for CG endpoints but not for core CRUD endpoints (API-03)

This is not a knowledge gap -- the team clearly knows these patterns (they implemented them). It is a coverage gap, likely caused by features being added incrementally without a final consistency pass. A checklist-based review ("does this new component have: shutdown channel? error backoff? auth check? versioned route? masked credentials?") would catch these.

### SP-03: Documentation Quality Exceeds Implementation Completeness
The codebase has exceptional documentation: module-level rustdoc with ASCII diagrams, specification references, tutorials, ADRs, and a well-curated prelude (LEG-07, LEG-08, LEG-10, LEG-12). But several documented features are partially implemented:

- Multi-tenancy is documented and has API routes but lacks query-level isolation (SEC-02)
- Metrics endpoint exists and returns Prometheus format but contains only a static placeholder (OPS-01)
- Security assessment scans packages but provides no real security (SEC-16)
- The `#[non_exhaustive]` on config structs documents future extensibility but `build()` performs no validation (OPS-07)

This creates a risk of false confidence: an evaluator reading the docs (or even the API surface) would conclude the system is more complete than it is. The documentation should note which features are stubs or partial implementations.

### SP-04: Computation Graph Subsystem as Design Template
The computation graph subsystem consistently appears as a positive example across multiple lenses:

- **Legibility** (LEG-10): Well-isolated vocabulary, 1:1 file-to-concept mapping
- **Operability** (OPS-13): Production-grade supervision with circuit breaker and backoff
- **API Design** (API-09): Consistent macro-based definition pattern
- **Performance** (PERF-06): Thoughtful lock contention analysis with bounded channels
- **Evolvability** (EVO-03, Change 1): Clear extension points for new accumulator types

This subsystem was likely designed and built more recently (evidenced by its `/v1/` API prefix and its specification references to CLOACI-S-0004/S-0005). It should serve as the reference architecture when hardening the older subsystems -- particularly for the scheduler loop (add circuit breaker per OPS-05/OPS-13 pattern) and the HTTP API (add versioned routes per the CG pattern).

## Severity Adjustments

### Upgraded

| Finding | Original Severity | Adjusted Severity | Rationale |
|---------|-------------------|-------------------|-----------|
| LEG-01 (Pipeline/Workflow naming) | Major | Major+ | Combined with COR-01, API-01, API-06 -- the naming confusion makes the completion-status bug harder to detect and the API harder to consume. Treat as prerequisite for COR-01 fix. |
| OPS-01 (No metrics) | Major | Major+ | Amplifies every other finding. Without metrics, COR-01 (silent pipeline "success") is undetectable in production. Without latency metrics, PERF findings cannot be validated. |
| SEC-02 (Tenant data isolation) | Critical | Critical (confirmed) | Combined with SEC-01 and SEC-04, creates a full privilege escalation chain: authenticate with any key, enumerate tenants, access any tenant's data, mint admin keys. |

### Downgraded

| Finding | Original Severity | Adjusted Severity | Rationale |
|---------|-------------------|-------------------|-----------|
| PERF-01 (Reactor per-execution persist) | Minor | Observation | The persistence is best-effort and async. Combined with COR-09 analysis, the current approach is a reasonable crash-recovery tradeoff. Only matters at throughputs the system is unlikely to see in its current deployment model. |
| LEG-03 (Three dispatch macros) | Minor | Minor (confirmed, but low priority) | Root cause RC-03 explains this as an inherent cost. The unused macros should be cleaned up, but this is housekeeping, not architectural. |
| SEC-12 (SHA-256 without salt) | Minor | Observation | The 256-bit input space makes precomputation infeasible. This is a theoretical concern with no practical attack scenario. |

### Priority Ordering (Recommended Fix Sequence)

1. **SEC-01 + SEC-02 + SEC-04**: Authorization and tenant isolation -- critical security cluster
2. **SEC-03**: Make signature verification mandatory in server mode
3. **COR-01**: Fix pipeline completion status determination
4. **COR-03 + OPS-04 + OPS-08**: SchedulerLoop shutdown + server graceful shutdown
5. **OPS-03**: Remove credential logging from Python bindings
6. **COR-02**: Make task completion atomic
7. **OPS-01 + OPS-02**: Metrics and tracing infrastructure
8. **API-02 + API-03**: REST API error format and versioning
9. **LEG-01 / API-01**: Pipeline-to-Workflow rename (large but high-leverage)
10. **EVO-02**: Scoped registries (highest architectural leverage, highest cost)
