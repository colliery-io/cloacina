# Architecture Review Report

## Executive Summary

Cloacina is a well-architected Rust-based workflow orchestration platform with strong foundations in its core engine, an exceptionally documented codebase, and a genuinely production-grade computation graph subsystem. The macro-based task/workflow API achieves low-ceremony ergonomics across both Rust and Python, the DAL accessor pattern is clean and discoverable, and the daemon lifecycle management (SIGHUP reload, graceful shutdown, force-exit) reflects operational maturity. The crate decomposition separating authoring types from runtime concerns is a deliberate, well-executed design choice that supports the plugin compilation story.

However, the review identified 3 Critical, 14 Major, 18 Minor findings, and 19 Observations across 8 review lenses. The most urgent concerns cluster in three areas: (1) **server-mode security** -- the multi-tenant authorization model is structurally incomplete, with any authenticated user able to mint admin keys (SEC-01), cross-tenant data leakage via unfiltered DAL queries (SEC-02), and unsigned package uploads enabling arbitrary code execution (SEC-03); (2) **correctness in the execution engine** -- pipelines are always marked "Completed" regardless of task failure (COR-01), and task completion across context-save and status-update is non-atomic (COR-02); and (3) **observability** -- no real metrics, no distributed tracing, and no request correlation exist, making the correctness and security issues effectively invisible in production. A clear maturity gradient runs through the system: the computation graph subsystem, daemon mode, and core executor are mature, while the HTTP API, server authorization, and multi-tenant isolation are at an early stage. Multiple instances of good patterns (shutdown channels, circuit breakers, credential masking, API versioning, auth checks) being applied in some subsystems but not others suggest incremental development without a final consistency pass.

## Summary Table

| Lens | Critical | Major | Minor | Observations |
|------|----------|-------|-------|--------------|
| Legibility (LEG) | 0 | 2 | 5 | 7 |
| Correctness (COR) | 1 | 3 | 3 | 4 |
| Evolvability (EVO) | 0 | 3 | 3 | 4 |
| Performance (PERF) | 0 | 0 | 3 | 8 |
| API Design (API) | 0 | 3 | 5 | 5 |
| Operability (OPS) | 0 | 4 | 4 | 5 |
| Security (SEC) | 3 | 5 | 4 | 5 |
| Cross-Cutting (CLF) | -- | -- | -- | 5 clusters |
| **Total** | **4** | **20** | **27** | **43** |

*Note: Cross-cutting clusters group existing findings; they do not add new counts. The total reflects unique findings.*

## Findings by Lens

### Legibility

**Assessment**: The codebase is well above typical Rust project standards for documentation, with thorough module-level rustdoc, ASCII architecture diagrams, and a well-curated prelude. The primary legibility concerns are terminology inconsistency and naming collisions between similarly-purposed modules.

| ID | Title | Severity |
|----|-------|----------|
| LEG-01 | "Pipeline" vs "Workflow" terminology drift | Major |
| LEG-02 | Scheduler module naming collision (scheduler.rs vs task_scheduler/) | Major |
| LEG-03 | Three backend-dispatch macros with overlapping purpose | Minor |
| LEG-04 | Duplicate and legacy error variants in ValidationError | Minor |
| LEG-05 | Lossy error conversion -- ContextError::Database mapped to KeyNotFound | Minor |
| LEG-06 | lib.rs re-exports duplicate what prelude already provides | Minor |
| LEG-09 | DefaultRunner struct has many Arc<RwLock<Option<Arc<...>>>> fields | Minor |
| LEG-11 | Task scheduler code duplication between PostgreSQL and SQLite branches | Minor |
| LEG-07 | Excellent module-level documentation throughout | Positive |
| LEG-08 | DAL accessor pattern is clear and consistent | Positive |
| LEG-10 | Computation graph subsystem is well-isolated and named | Positive |
| LEG-12 | cloacinactl main.rs is a model of CLI legibility | Positive |
| LEG-13 | Workflow vs Graph conceptual overlap (two graph modules) | Observation |
| LEG-14 | "task_name" field stores full namespace strings | Observation |

### Correctness

**Assessment**: Core execution paths are well-protected -- DAL state transitions are transactional, outbox claiming uses `FOR UPDATE SKIP LOCKED` (Postgres) and `IMMEDIATE` transactions (SQLite), and DAG cycle detection is correctly implemented. The gaps are in pipeline-level status determination, transaction atomicity at the task completion boundary, and a missing shutdown mechanism on one scheduler component.

| ID | Title | Severity |
|----|-------|----------|
| COR-01 | Pipeline marked "Completed" even when tasks failed | Critical |
| COR-02 | `complete_task_transaction` is not actually atomic | Major |
| COR-03 | `SchedulerLoop::run()` has no shutdown mechanism | Major |
| COR-04 | Error conversion maps Database/ConnectionPool errors to KeyNotFound | Major |
| COR-05 | `WorkflowGraph::add_task` silently overwrites duplicate task IDs | Minor |
| COR-06 | `find_parallel_groups` sorts by group size, not depth level | Minor |
| COR-07 | Stale claim sweeper log over-reports released claims | Minor |
| COR-08 | `PipelineStatus::from_str` silently defaults invalid strings to Failed | Observation |
| COR-09 | Sequential reactor queue persistence timing gap | Observation |
| COR-10 | `TriggerRule::All` with empty conditions evaluates to true (vacuous truth) | Observation |
| COR-11 | Context merge silently swallows dependency load errors | Observation |

### Evolvability

**Assessment**: The architecture has well-placed trait-based extension points (Dispatcher/TaskExecutor, RegistryStorage, StreamBackend) and the crate decomposition supports the plugin use case well. The main evolvability concerns are the monolithic core crate, global mutable registries that prevent isolation, and structural code duplication from the dual-backend DAL strategy.

| ID | Title | Severity |
|----|-------|----------|
| EVO-01 | Monolithic core crate (67K lines, dual crate-type) | Major |
| EVO-02 | Global mutable registries impede isolation (160 #[serial] tests) | Major |
| EVO-03 | DAL code duplication for dual-backend support | Major |
| EVO-04 | Error type fragmentation between crates | Minor |
| EVO-05 | Python bindings runner is a parallel implementation (2,888 lines) | Minor |
| EVO-06 | No trait abstraction for the DAL itself | Minor |
| EVO-07 | Tight coupling between scheduler layers | Observation |
| EVO-08 | Feature flag complexity in database layer | Observation |
| EVO-09 | Crate dependency on fidius 0.0.5 (pre-1.0) | Observation |
| EVO-10 | Test architecture supports refactoring moderately well | Observation |

### Performance

**Assessment**: The system demonstrates thoughtful performance design on its critical paths: batch database queries in the scheduler loop, `FOR UPDATE SKIP LOCKED` for contention-free claiming, semaphore-based concurrency limiting, and channel-based backpressure in the computation graph pipeline. All findings are Minor or Observation -- there are no performance-critical issues. The system is correctly optimized for its expected workloads.

| ID | Title | Severity |
|----|-------|----------|
| PERF-01 | Reactor persists state on every graph execution | Minor (downgraded to Observation by cross-cutting) |
| PERF-02 | Trigger condition evaluation issues per-condition database queries | Minor |
| PERF-03 | Pipeline completion context lookup is per-task | Minor |
| PERF-04 | wait_for_completion polls with 500ms sleep | Observation |
| PERF-05 | SQLite pool size hardcoded to 1 | Observation |
| PERF-06 | Reactor RwLock contention between receiver and executor | Observation |
| PERF-07 | Sequential reactor mode drains queue under single lock per item | Observation |
| PERF-08 | Auth key cache uses tokio::sync::Mutex | Observation |
| PERF-09 | Batch execution event inserts are per-task in a loop | Observation |
| PERF-10 | Computation graph snapshot clones entire cache | Observation |
| PERF-11 | Dependency checking reconstructs workflow from global registry on every check | Observation |

### API Design

**Assessment**: The macro-based task/workflow definition achieves genuinely low-ceremony ergonomics, the Python bindings faithfully mirror Rust concepts with idiomatic patterns, and the CLI is immediately understandable. The primary issues are the Pipeline/Workflow terminology split leaking through all surfaces, the REST API lacking a consistent error format and API versioning, and several configuration footguns where invalid values are silently accepted.

| ID | Title | Severity |
|----|-------|----------|
| API-01 | Pipeline/Workflow terminology split leaks through all consumer surfaces | Major |
| API-02 | REST API has no consistent error response format | Major |
| API-03 | REST API routes lack version prefix for core endpoints | Major |
| API-04 | Configuration accepts freeform strings where enums would prevent misuse | Minor |
| API-05 | get_workflow handler lists all workflows and filters in memory | Minor |
| API-06 | Prelude exports Pipeline terminology alongside Workflow types | Minor |
| API-07 | Python config uses `_seconds` and `_ms` suffixes inconsistently | Minor |
| API-08 | WorkflowBuilder context manager __exit__ drops description/tags | Minor |
| API-13 | REST API success responses lack a consistent envelope | Minor |
| API-16 | list_executions returns only active executions despite being named "list" | Minor |
| API-09 | Macro system provides excellent consumer ergonomics | Positive |
| API-10 | Python bindings faithfully mirror Rust API with idiomatic patterns | Positive |
| API-11 | CLI is well-structured and self-documenting | Positive |
| API-12 | DefaultRunnerConfig builder provides granular control with safe defaults | Positive |
| API-14 | WebSocket auth supports dual token sources -- good design | Positive |
| API-15 | DAL accessor pattern provides excellent API discoverability | Positive |

### Operability

**Assessment**: Operability posture is mixed. The daemon has excellent lifecycle management, the computation graph supervisor is production-grade, and security audit logging follows SIEM conventions. However, the system has no real metrics, no distributed tracing, no request correlation, and the server's shutdown path is incomplete. The Python bindings leak database credentials in logs.

| ID | Title | Severity |
|----|-------|----------|
| OPS-01 | /metrics endpoint is a static placeholder -- no real metrics | Major |
| OPS-02 | No distributed tracing or request correlation | Major |
| OPS-03 | Python bindings log database URLs with credentials | Major |
| OPS-04 | SchedulerLoop::run() has no shutdown mechanism (operability perspective) | Major |
| OPS-05 | No circuit breaker for sustained database outages in scheduler loop | Minor |
| OPS-06 | No production Dockerfile or Kubernetes manifests | Minor |
| OPS-07 | Configuration is not validated at startup | Minor |
| OPS-08 | Server graceful shutdown does not drain the runner | Minor |
| OPS-09 | Daemon log rotation has no size limit | Minor |
| OPS-10 | SIGHUP config reload in daemon is well-designed | Positive |
| OPS-11 | Security audit logging follows SIEM conventions | Positive |
| OPS-12 | Database URL masking in serve.rs is a good practice | Positive |
| OPS-13 | Reactive scheduler supervision with circuit breaker is production-grade | Positive |
| OPS-14 | Graceful daemon shutdown with force-exit escape hatch | Positive |

### Security

**Assessment**: Cryptographic foundations are solid (Ed25519, AES-256-GCM, proper key management), SQL injection prevention is thorough in the data layer, and the API key design is well-structured. However, the server authorization model has critical gaps: any authenticated user can create admin keys, tenant data isolation is not enforced at the query layer, and signature verification for uploaded packages is off by default. The system has no TLS support, no rate limiting, and multiple credential exposure surfaces.

| ID | Title | Severity |
|----|-------|----------|
| SEC-01 | create_key endpoint has no authorization check -- any user can mint admin keys | Critical |
| SEC-02 | Tenant data isolation not enforced at database query layer (IDOR) | Critical |
| SEC-03 | Package signature verification off by default, not enforced on API upload | Critical |
| SEC-04 | list_tenants endpoint has no authorization check | Major |
| SEC-05 | WebSocket auth token accepted via query parameter -- exposed in logs | Major |
| SEC-06 | No TLS configuration -- all traffic in cleartext | Major |
| SEC-07 | No rate limiting on any endpoint | Major |
| SEC-08 | Tenant credentials returned in HTTP response body | Major |
| SEC-09 | FFI plugin loading executes arbitrary native code with no sandboxing | Major |
| SEC-10 | Revoked key remains valid for up to 30s due to cache TTL | Minor |
| SEC-11 | Bootstrap key written to disk in plaintext | Minor |
| SEC-12 | API key hash uses SHA-256 without salt (practical risk: low) | Minor (downgraded to Observation by cross-cutting) |
| SEC-13 | No request body size limit on multipart upload endpoint | Minor |
| SEC-14 | No dependency vulnerability auditing in CI pipeline | Minor |
| SEC-15 | Database URL with credentials exposed via CLI argument | Observation |
| SEC-16 | Security heuristic scan of packages is trivially bypassable | Observation |
| SEC-17 | Password escaping for tenant creation is necessary but fragile | Observation |

## Cross-Cutting Concerns

### Root Causes

Four root causes explain the majority of findings:

1. **RC-01: Global Mutable State as Architecture** (drives EVO-01, EVO-02, EVO-06, EVO-10, PERF-11). Process-global static registries populated via `#[ctor]` are the most consequential architectural choice. They prevent parallel testing (160 `#[serial]` annotations), prevent crate decomposition, and preclude multiple isolated workflow environments in the same process.

2. **RC-02: Multi-Tenant Server as Single-Tenant with Decorative Tenant IDs** (drives SEC-01, SEC-02, SEC-04, API-16, OPS-08). The HTTP API presents multi-tenant URLs but the implementation uses a single shared database connection without tenant filtering. This is the root of the critical security cluster.

3. **RC-03: Dual-Backend Strategy Produces Structural Code Duplication** (drives EVO-03, LEG-03, LEG-11, EVO-08). The runtime backend selection via `dispatch_backend!` produces near-identical method pairs in every DAL module. An inherent cost of the product requirement, manageable for two backends but would be prohibitive for three.

4. **RC-04: Python Bindings as Parallel Implementation** (drives EVO-05, OPS-03, API-07, API-08). The `PyDefaultRunner` (2,888 lines) reimplements coordination logic rather than wrapping the Rust API, causing drift and credential logging gaps.

### Severity Adjustments from Cross-Cutting Analysis

**Upgraded:**
- LEG-01 (Pipeline/Workflow naming): Major to Major+ -- naming confusion makes COR-01 harder to detect and the API harder to consume across four lenses
- OPS-01 (No metrics): Major to Major+ -- amplifies every other finding; COR-01 (silent pipeline "success") is undetectable without metrics
- SEC-02 (Tenant isolation): Critical confirmed -- combined with SEC-01 and SEC-04, creates a full privilege escalation chain

**Downgraded:**
- PERF-01 (Reactor per-execution persist): Minor to Observation -- reasonable crash-recovery tradeoff; only relevant at throughputs unlikely in current deployment
- SEC-12 (SHA-256 without salt): Minor to Observation -- 256-bit input space makes precomputation infeasible

### Systemic Patterns

1. **Maturity Gradient**: Computation graph and daemon mode are production-grade; core executor is solid with gaps; REST API and server authorization are at an early stage. The system was built outward from the core engine, with newer subsystems (CG) getting more polish than connecting surfaces (HTTP API).

2. **Inconsistent Application of Good Patterns**: Shutdown channels, circuit breakers, credential masking, API versioning, and auth checks are each implemented in one subsystem but missing from an adjacent one. This reflects incremental development without a consistency pass.

3. **Documentation Quality Exceeds Implementation Completeness**: Exceptional documentation (module docs, ASCII diagrams, specifications, tutorials) but several documented features are partially implemented (multi-tenancy, metrics, security assessment), creating a risk of false confidence.

4. **Computation Graph as Design Template**: This subsystem consistently appears as a positive example across all lenses (legibility, operability, API design, performance, evolvability) and should serve as the reference architecture when hardening older subsystems.
