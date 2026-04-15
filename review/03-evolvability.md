# Evolvability Review

## Summary

Cloacina's architecture is **moderately evolvable** with clear strengths in its trait-based extension points and layered module separation, but meaningful weaknesses in global mutable state, dual-backend code duplication, and a monolithic core crate that conflates many concerns. Adding new executor backends or storage implementations is well-supported by traits; adding new entity types or changing the persistence model is expensive due to widespread DAL duplication. The `Runtime` abstraction partially tames the global registry problem but does not eliminate it.

## Architecture Assessment

**Strengths:**
- Clean trait boundaries for `Dispatcher`, `TaskExecutor`, `WorkflowRegistry`, `RegistryStorage`, and `WorkDistributor` enable pluggable backends without changing callers.
- The `Runtime` struct provides scoped isolation of task/workflow/trigger registries, supporting parallel tests and multi-tenant server operation.
- The `DefaultRunnerBuilder` pattern with `DefaultRunnerConfig` centralizes configuration with ~30 knobs, all with sensible defaults and individual overrides.
- The `cloacina-workflow` crate successfully separates lightweight authoring types (Task trait, Context, errors, retry) from heavy runtime dependencies (database, executor, scheduler).
- Feature flags (`postgres`, `sqlite`, `kafka`, `macros`) enable building only what is needed.

**Weaknesses:**
- The core `cloacina` crate is a single monolith containing ~25 public modules spanning persistence, execution, scheduling, security, Python bindings, packaging, and computation graphs. This makes it difficult to evolve one concern without understanding many others.
- Process-global static registries (7+ `Lazy<...>` statics) create hidden coupling between tests, require `#[serial]` (161 occurrences across 24 files), and leak state between unrelated components.
- The dual-backend pattern (`dispatch_backend!` macro + separate `_postgres`/`_sqlite` methods in every DAL module) means every persistence change must be implemented twice with no shared abstraction.
- `DefaultRunner` accumulates optional subsystems via `Arc<RwLock<Option<Arc<...>>>>` fields, making it a god object that orchestrates 8+ independent services.

## Change Cost Analysis

### Change 1: Add a new executor backend (e.g., Kubernetes)

**Cost: Low.** The `TaskExecutor` trait at `crates/cloacina/src/dispatcher/traits.rs` is well-defined with 4 methods (`execute`, `has_capacity`, `metrics`, `name`). Registration uses `dispatcher.register_executor("k8s", Arc::new(k8s_executor))`. The `RoutingConfig` system with glob-based rules already supports routing different tasks to different executors. No changes to scheduler, DAL, or runner needed.

**Blast radius:** New crate + 1 line of registration in DefaultRunnerBuilder. Minimal.

### Change 2: Add a new database-backed entity (e.g., "Audit Log" table)

**Cost: High.** Requires:
1. New Diesel migration in *both* `migrations/postgres/` and `migrations/sqlite/`
2. New model struct in `crates/cloacina/src/models/`
3. New schema entry in `crates/cloacina/src/database/schema.rs`
4. New DAL module in `crates/cloacina/src/dal/unified/` with separate `_postgres` and `_sqlite` async methods, each duplicating connection acquisition and query logic
5. New accessor method on `DAL` struct
6. If exposed via HTTP: new handler in `crates/cloacinactl/src/server/`

**Blast radius:** 6+ files across 3 directories. The dual-backend pattern roughly doubles the persistence code for every entity.

### Change 3: Replace the database backend (e.g., add MySQL)

**Cost: Very High.** The `dispatch_backend!` macro and every DAL module method are structured as binary branches (Postgres or SQLite). Adding a third backend would require:
1. Extending `BackendType` enum and `AnyPool` enum with a new variant
2. Updating `dispatch_backend!` macro to support 3 branches (or N branches)
3. Adding a third implementation method to *every* DAL module (~15 entity modules, each with 3-8 methods = 45-120 new method implementations)
4. New migration directory and migration embedding
5. New Diesel schema if MySQL types differ

The architecture assumes exactly two backends. There is no trait-based DAL abstraction that would allow adding a backend without modifying existing code.

## Findings

## EVO-001: Monolithic core crate conflates many concerns
**Severity**: Major
**Location**: `crates/cloacina/src/lib.rs` (lines 490-578), `crates/cloacina/Cargo.toml`
**Confidence**: High

### Description
The `cloacina` crate is simultaneously a persistence library (DAL, models, database, migrations), an execution engine (scheduler, dispatcher, executor), a packaging/security system (packaging, crypto, security, registry), a Python binding module (python/*), a computation graph runtime (computation_graph/*), and a public API surface. It has 43 direct dependencies in Cargo.toml.

### Evidence
`lib.rs` declares 21 public modules and re-exports ~70 symbols at the crate root. The crate builds as both `lib` and `cdylib` (for Python). Every change to any subsystem recompiles the entire crate. The Python module (`#[pymodule] fn cloaca`) is defined directly in `lib.rs` alongside Rust-only exports.

### Suggested Resolution
Consider splitting into focused crates: `cloacina-dal` (persistence), `cloacina-engine` (scheduler/dispatcher/executor), `cloacina-python` (PyO3 bindings), and `cloacina-security` (crypto, signing, keys). The existing `cloacina-workflow` and `cloacina-computation-graph` crates show this pattern is already understood. A facade `cloacina` crate could re-export for backward compatibility.

---

## EVO-002: Process-global static registries impede test isolation and parallel evolution
**Severity**: Major
**Location**: `crates/cloacina/src/task.rs:637`, `crates/cloacina/src/workflow/registry.rs:36`, `crates/cloacina/src/trigger/registry.rs:36`, `crates/cloacina/src/computation_graph/global_registry.rs`, `crates/cloacina/src/computation_graph/stream_backend.rs:138`, `crates/cloacina/src/python/computation_graph.rs:62-463`
**Confidence**: High

### Description
At least 9 process-global `Lazy<Mutex<...>>` or `Lazy<RwLock<...>>` statics hold mutable registries for tasks, workflows, triggers, computation graphs, stream backends, Python nodes, and Python graph executors. The `#[ctor]` macro auto-registers tasks at process startup into these globals.

### Evidence
161 uses of `#[serial]` across 24 test files demonstrate that tests cannot run in parallel due to shared global state. The `Runtime` struct (introduced to mitigate this) only covers tasks, workflows, and triggers -- not computation graphs, stream backends, or Python registries. `Runtime::from_global()` still delegates to the global registries, meaning dynamically loaded packages mutate globals that all runtimes see.

### Suggested Resolution
Extend the `Runtime` struct to encompass all registry types (computation graphs, stream backends). Consider making `#[ctor]` registration opt-in rather than default, allowing explicit runtime-scoped registration. For Python bindings, use per-runtime state rather than module-level statics.

---

## EVO-003: Dual-backend DAL pattern doubles persistence code with no shared abstraction
**Severity**: Major
**Location**: `crates/cloacina/src/dal/unified/` (all 15+ sub-modules), `crates/cloacina/src/database/connection/backend.rs:265` (`dispatch_backend!` macro)
**Confidence**: High

### Description
Every DAL entity module (context, task_execution, pipeline_execution, schedule, etc.) contains paired `_postgres` and `_sqlite` methods with identical logic differing only in connection acquisition and minor SQL dialect differences. The `dispatch_backend!` macro routes between exactly two branches at runtime. Adding a third backend or modifying shared logic requires editing every entity module.

### Evidence
In `crates/cloacina/src/dal/unified/context.rs`, the `create` method dispatches to `create_postgres` and `create_sqlite` which share identical serialization logic but differ in connection acquisition. This pattern repeats across ~15 entity modules with 3-8 methods each, yielding roughly 60-120 method pairs. The `Database::run_migrations()` method in `connection/mod.rs` has 4 separate `#[cfg]` blocks for the 4 combinations of features.

### Suggested Resolution
Introduce an `async fn with_connection<F, R>(&self, f: F) -> Result<R, Error>` method on `Database` that abstracts connection acquisition. Consider Diesel's `MultiConnection` feature more aggressively, or define a `DalConnection` trait that both backends implement. This would collapse paired methods into single implementations.

---

## EVO-004: DefaultRunner is a god object orchestrating 8+ services
**Severity**: Major
**Location**: `crates/cloacina/src/runner/default_runner/mod.rs:69-91`, `crates/cloacina/src/runner/default_runner/services.rs`
**Confidence**: High

### Description
`DefaultRunner` holds 11 fields including 6 `Arc<RwLock<Option<Arc<...>>>>` for optional subsystems (cron recovery, workflow registry, registry reconciler, unified scheduler, reactive scheduler). It manages background task lifecycles, shutdown coordination, and exposes methods spanning workflow execution, cron scheduling, registry management, and reactive scheduling.

### Evidence
The struct definition at `mod.rs:69-91` shows the accumulation. `services.rs` contains `start_background_services()` which conditionally spawns 5 different background tasks based on config flags. The `shutdown()` method sequentially awaits 5 optional join handles. The `Clone` implementation clones all 11 `Arc` fields. Adding any new service requires modifying DefaultRunner, its builder, its service startup, and its shutdown.

### Suggested Resolution
Extract service lifecycle management into a `ServiceManager` that holds a dynamic collection of `Box<dyn BackgroundService>`. Each service (scheduler, executor, cron recovery, reconciler, reactive scheduler) would implement a `BackgroundService` trait with `start()`, `shutdown()`, and `name()` methods. DefaultRunner delegates to the ServiceManager.

---

## EVO-005: No trait abstraction for the DAL aggregate
**Severity**: Minor
**Location**: `crates/cloacina/src/dal/unified/mod.rs:94-231`
**Confidence**: Medium

### Description
The `DAL` struct is a concrete type that directly constructs concrete DAL sub-types (ContextDAL, TaskExecutionDAL, etc.). There is no `trait DAL` or `trait Repository` that would allow substituting a mock or alternative storage implementation at the DAL aggregate level.

### Evidence
Components like `ThreadTaskExecutor` and `DefaultDispatcher` receive the concrete `DAL` type directly. Tests must use real database fixtures (`TestFixture`) with actual PostgreSQL/SQLite connections. The `cloacina-testing` crate provides a `TestRunner` that avoids the database entirely, but it does so by reimplementing execution logic rather than by injecting a mock DAL.

### Suggested Resolution
Define a `trait DataAccessLayer` with methods returning trait objects for each entity (context, task_execution, etc.). This would enable in-memory mock implementations for unit testing and alternative storage backends without database dependencies.

---

## EVO-006: Test architecture requires database infrastructure for most integration tests
**Severity**: Minor
**Location**: `crates/cloacina/tests/fixtures.rs`, `crates/cloacina/tests/integration/`
**Confidence**: High

### Description
The integration test suite requires a running PostgreSQL instance and uses `TestFixture` with real database connections. The `backend_test!` macro enables running tests against all enabled backends but still requires database infrastructure. The `#[serial]` requirement (161 occurrences) means integration tests cannot run in parallel, increasing CI time.

### Evidence
`fixtures.rs` hardcodes `postgres://cloacina:cloacina@localhost:5432/cloacina` as the default. The `get_or_init_postgres_fixture()` function creates a shared singleton fixture. Tests use `guard.reset_database().await` to truncate all tables between tests. The `cloacina-testing` crate provides database-free testing but only for pure task logic, not for scheduler, executor, or DAL behavior.

### Suggested Resolution
Invest in the `cloacina-testing` crate to cover more of the execution pipeline without databases. Consider extracting DAL traits (see EVO-005) to enable in-memory test doubles. For tests that genuinely need databases, the schema-isolation approach is good -- consider generating unique schemas per test to enable parallel execution.

---

## EVO-007: Feature flag combinatorics create maintenance burden
**Severity**: Minor
**Location**: `crates/cloacina/src/database/connection/mod.rs` (lines 377-449), `crates/cloacina/Cargo.toml` features section
**Confidence**: Medium

### Description
The crate supports 4 feature flag combinations for database backends: `postgres+sqlite` (default), `postgres`-only, `sqlite`-only, and (implicitly) neither. Each combination requires separate `#[cfg]` blocks in connection management, migration running, pool creation, and DAL dispatch. The `run_migrations()` method alone has 4 separate `#[cfg]` blocks with duplicated logic.

### Evidence
`Database::try_new_with_schema()` in `connection/mod.rs` has 3 separate `#[cfg]` blocks for pool creation. `Database::run_migrations()` has 4 `#[cfg]` blocks. The `dispatch_backend!` macro handles 3 of 4 combinations. This means that CI must test at least 3 feature combinations to ensure correctness, and any new database code must be written 2-4 times.

### Suggested Resolution
Consider consolidating the feature-gated code paths using an enum-dispatch pattern or a `DatabaseBackend` trait that both backends implement. This would replace `#[cfg]` blocks with a single code path using dynamic dispatch, trading a negligible runtime cost for significantly reduced maintenance burden.

---

## EVO-008: Computation graph system is parallel but partially integrated
**Severity**: Observation
**Location**: `crates/cloacina/src/computation_graph/`, `crates/cloacina-computation-graph/`
**Confidence**: Medium

### Description
The computation graph system exists as both a separate crate (`cloacina-computation-graph` for macro-generated runtime types) and an embedded module within the core crate (`computation_graph/` with scheduler, registry, accumulator, reactor). It has its own global registry, its own scheduler (`ReactiveScheduler`), its own packaging bridge, and its own health state machines. Integration with the workflow system happens through the reconciler and the DefaultRunner, but the two systems share little infrastructure.

### Evidence
`DefaultRunner` holds a separate `reactive_scheduler` field alongside the workflow `scheduler`. The reconciler routes packages to either the workflow or computation graph path based on `has_computation_graph()` metadata. The computation graph global registry is separate from the workflow/task/trigger registries and is not covered by the `Runtime` abstraction. WebSocket endpoints in cloacinactl are specific to computation graphs.

### Suggested Resolution
This is not necessarily a problem -- the two systems may genuinely have different lifecycles. However, if they continue to evolve in parallel, consider: (1) bringing the CG global registry under the `Runtime` umbrella, (2) extracting common patterns (health state machines, shutdown coordination) into shared utilities, and (3) ensuring the `BackgroundService` pattern (if adopted from EVO-004) covers both workflow and CG services uniformly.

---

## EVO-009: Python bindings are tightly coupled to internals
**Severity**: Observation
**Location**: `crates/cloacina/src/python/`, `crates/cloacina/src/lib.rs:592-654`
**Confidence**: Medium

### Description
Python bindings are defined within the core `cloacina` crate and directly reference internal types (DAL, Database, TaskRegistry, Runtime). The `#[pymodule]` entry point in `lib.rs` registers 20+ Python classes and functions. Any change to internal Rust types that are wrapped by Python bindings requires coordinated updates to the Python wrapper code.

### Evidence
`lib.rs` lines 592-654 show the `cloaca` pymodule registering types from `python::context`, `python::task`, `python::trigger`, `python::workflow`, `python::computation_graph`, and `python::bindings::*`. The `python/` directory contains 14 files mirroring internal modules. The `PyDefaultRunner` in `python/bindings/runner.rs` wraps the Rust `DefaultRunner` and re-exposes its methods.

### Suggested Resolution
Consider moving Python bindings to a separate `cloacina-python` crate that depends on the public API surface of `cloacina`. This would enforce a clean boundary -- Python bindings could only use public APIs, reducing coupling to internal implementation details. It would also allow the core crate to build without PyO3 dependencies when Python support is not needed, and avoid the dual `lib`/`cdylib` crate-type on the core crate.

---

## EVO-010: Extension points are well-designed where they exist
**Severity**: Observation (positive)
**Location**: `crates/cloacina/src/dispatcher/traits.rs`, `crates/cloacina/src/registry/traits.rs`
**Confidence**: High

### Description
The trait-based extension points are well-designed and documented. The `TaskExecutor` trait enables custom execution backends (Kubernetes, serverless). The `Dispatcher` trait enables custom routing. The `WorkflowRegistry` and `RegistryStorage` traits enable custom storage backends. The `WorkDistributor` trait enables custom work notification mechanisms (PostgreSQL LISTEN/NOTIFY vs. polling). The `AccumulatorFactory` trait enables custom computation graph sources.

### Evidence
`dispatcher/traits.rs` includes doc examples showing a `KubernetesExecutor` implementation. The `DefaultDispatcher` uses glob-based routing rules that can be configured at runtime. The `WorkflowRegistryImpl` is generic over `S: RegistryStorage`, allowing filesystem, database, or custom storage. These traits have clean, minimal interfaces (3-5 methods each).

### Suggested Resolution
No changes needed. This is a model for how other parts of the system should evolve. The same trait-based pattern should be applied to the DAL (EVO-005) and background service management (EVO-004).
