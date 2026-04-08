# Evolvability Review

## Summary

Cloacina has a well-layered architecture with good trait-based abstractions at most major boundaries (Dispatcher/TaskExecutor, WorkflowRegistry/RegistryStorage, StreamBackend, WorkDistributor). The internal crate decomposition -- separating `cloacina-workflow` (authoring types) from `cloacina` (runtime) and `cloacina-computation-graph` (CG core types) -- is a genuine strength that enables lighter-weight compilation for plugin authors. However, the core `cloacina` crate is a monolith carrying 67,000 lines of Rust across 20+ top-level modules, and its pervasive use of global mutable registries creates hidden coupling that raises the cost of change for anything touching task/workflow/trigger/CG registration. The dual-backend (Postgres/SQLite) DAL strategy, while architecturally sound in concept, produces significant code duplication via backend-specific method pairs in every DAL module, making schema evolution expensive.

## Architecture Assessment

### Strengths

**1. Trait-based extension points are well-placed.** The `Dispatcher`/`TaskExecutor` boundary (`dispatcher/traits.rs`) is a clean seam. Adding a new executor backend (Kubernetes, serverless) requires implementing one 4-method trait and registering it -- the scheduler and dispatcher are decoupled. The `RegistryStorage` trait similarly isolates binary storage backends. The `StreamBackend` trait enables pluggable stream brokers. These are the right abstractions in the right places.

**2. The crate decomposition supports the packaging use case.** `cloacina-workflow` carries only authoring types (Context, Task, TaskNamespace, RetryPolicy, error types) with zero database or runtime dependencies. This means packaged workflow cdylibs can be compiled lean. `cloacina-computation-graph` does the same for CG plugins. This is a deliberate, well-executed design choice that pays dividends for the plugin compilation story.

**3. Feature flags enable genuine compile-time backend selection.** The `postgres`, `sqlite`, and `kafka` feature flags gate real code and dependencies. A SQLite-only build avoids pulling in PostgreSQL drivers entirely. This is appropriate structural coupling.

**4. The DAL sub-object pattern (`dal.context()`, `dal.task_execution()`, etc.) is clean.** Each entity gets its own DAL struct with scoped methods. This means adding a new entity type (e.g., a new table) is additive -- you create a new DAL sub-module and a new accessor method on `DAL`, without modifying existing entity DALs.

**5. Builder pattern on DefaultRunner.** The `DefaultRunnerBuilder` supports incremental configuration without breaking existing callers. Adding a new configuration knob is a one-method addition.

### Weaknesses

**1. The core crate is a monolith.** 67,000 lines in `crates/cloacina/src/` across 20+ top-level modules (dal, database, executor, scheduler, task_scheduler, runner, registry, packaging, security, crypto, computation_graph, python, dispatcher, trigger, workflow, context, graph, models, ...). This produces coupling through shared access to internal types and makes it impossible to use, say, the executor without also compiling the entire registry, security, and Python binding system. The `cdylib` crate type in the library Cargo.toml means every library consumer also builds the PyO3 module, which is likely undesirable for pure-Rust consumers.

**2. Global mutable registries are a significant coupling mechanism.** There are at least 5 distinct global static registries (task, workflow, trigger, computation graph, stream backend) plus 4 additional Python-specific globals. These use `Lazy<Arc<RwLock<HashMap<...>>>>` and are populated at program startup via `#[ctor]`. This creates:
- Hidden cross-test interference (tests must use `#[serial]` -- 160 occurrences found)
- Inability to run multiple isolated workflow environments in the same process
- Difficulty reasoning about what state exists at any given point

**3. Dual-backend DAL requires writing every query twice.** Each DAL method (e.g., `TaskOutboxDAL::create`) has both a `create_postgres` and `create_sqlite` implementation gated by `#[cfg(feature = "...")]`. This is not a trait-based dispatch -- it is structural code duplication. Every schema change requires parallel modification in both branches, both migration sets, and the unified model types.

## Change Cost Analysis

### Change 1: Adding a New Accumulator Type

**Scenario:** Add a "windowed" accumulator that aggregates events over a time window.

**What changes:**
1. Define the trait/struct in `computation_graph/accumulator.rs` (already ~1,500 lines, follows established patterns for `BatchAccumulator`, `PollingAccumulator`)
2. Add a new proc-macro in `cloacina-macros/src/computation_graph/` (e.g., `#[windowed_accumulator]`)
3. Re-export from `computation_graph/mod.rs` and `lib.rs`
4. Add the decorator in `python/computation_graph.rs` for Python bindings
5. Add the new accumulator declaration type in `cloacina-workflow-plugin/src/types.rs` (FFI boundary)
6. Update the reconciler/loader in `registry/reconciler/loading.rs` to handle the new type in packages

**Cost assessment:** Moderate. The accumulator hierarchy is well-structured with clear patterns to follow. The main friction points are: (a) the Python bindings in `python/computation_graph.rs` are a separate, parallel implementation that must be manually synchronized, (b) the FFI types in `cloacina-workflow-plugin` must be updated for packaged CGs, and (c) the macro crate requires proc-macro expertise. The core accumulator addition itself is straightforward.

### Change 2: Adding a New Database Backend (e.g., MySQL)

**Scenario:** Support MySQL as a third runtime database backend.

**What changes:**
1. Add `mysql` feature flag in Cargo.toml
2. Extend `BackendType` enum with `MySQL` variant
3. Extend `AnyPool` enum with `MySQL(MysqlPool)` variant
4. Extend `AnyConnection` enum with `MySQL(MysqlConnection)`
5. Update `dispatch_backend!` macro to handle three branches
6. Update `connection_match!` macro to handle three branches
7. Add a third migration set in `database/migrations/mysql/`
8. Add `_mysql` method variants to **every** DAL module (15+ modules, each with multiple methods)
9. Update `Database::run_migrations` with a third `#[cfg]` branch
10. Add new universal type conversions for MySQL's type system
11. Add MySQL-specific `WorkDistributor` implementation
12. Update `create_work_distributor` factory

**Cost assessment:** Very high. The current dual-backend approach uses structural duplication rather than trait-based polymorphism in the DAL layer. Each DAL method has parallel `_postgres` and `_sqlite` implementations. A third backend multiplies this by 1.5x. The `dispatch_backend!` and `connection_match!` macros would need three-way variants. The migration system is already duplicated (postgres/ and sqlite/ directories). This is the most expensive kind of change because it touches nearly every file in the `dal/unified/` subtree. The architecture made a reasonable trade-off for two backends but does not scale to three.

### Change 3: Adding a New API Endpoint to cloacinactl serve

**Scenario:** Add `POST /tenants/{id}/workflows/{name}/pause` to pause a workflow.

**What changes:**
1. Add a handler function in `cloacinactl/src/server/workflows.rs`
2. Add the route in the axum router setup (likely in `cloacinactl/src/commands/serve.rs`)
3. The handler uses `cloacina::dal::DAL` and `cloacina::Database` from shared state
4. Possibly add a new DAL method if the operation needs a new query

**Cost assessment:** Low. The server module in `cloacinactl` has clean, file-per-resource organization (workflows.rs, tenants.rs, keys.rs, etc.). Adding a new handler is additive. The axum framework makes route additions straightforward. The coupling is appropriate -- the server naturally depends on the core library's DAL.

## Findings

### EVO-01: Monolithic Core Crate (Major)

**Location:** `crates/cloacina/` -- 67,000 lines, 20+ top-level modules, dual crate-type `["lib", "cdylib"]`

**Description:** The core `cloacina` crate contains the complete runtime: database/DAL, executor, scheduler, task_scheduler, registry, packaging, security, crypto, computation graphs, Python bindings, dispatcher, triggers, and workflows. This is not a library with optional components -- it is a monolith where everything depends on everything through `crate::` imports. The `cdylib` crate-type forces PyO3 compilation even for pure-Rust consumers.

**Impact:** Cannot adopt individual subsystems independently. Cannot separate the Python wheel build from the Rust library build without feature gymnastics. Compile times are artificially coupled. Internal refactoring requires understanding the full dependency web.

**Recommendation:** Consider extracting the Python bindings into a separate crate (`cloacina-python`) that depends on `cloacina`. Consider whether the executor/scheduler could be separated from the registry/packaging system. At minimum, separate `lib` and `cdylib` into distinct crates to decouple Rust and Python builds.

---

### EVO-02: Global Mutable Registries Impede Isolation (Major)

**Location:** `task.rs:637`, `workflow/registry.rs:36`, `trigger/registry.rs:36`, `cloacina-computation-graph/src/lib.rs:289`, `computation_graph/stream_backend.rs:138`; plus Python-specific globals in `python/computation_graph.rs`

**Description:** At least 5 process-global static registries hold constructors for tasks, workflows, triggers, computation graphs, and stream backends. These are populated via `#[ctor]` at program startup and read/written via `Arc<RwLock<HashMap>>`. There is no concept of a scoped or namespaced registry instance -- there is exactly one global registry per type per process.

**Impact:**
- Tests cannot run in parallel (160 `#[serial]` annotations across the test suite)
- Cannot run multiple independent workflow environments in the same process
- Global state makes it difficult to reason about initialization ordering
- The `#[ctor]`-based registration is invisible to the developer -- tasks appear in the registry "by magic", which creates debugging difficulties when registration fails silently

**Recommendation:** Introduce a `Runtime` or `Engine` struct that owns registry instances. Tasks/workflows can still be discovered via `#[ctor]` into a global staging area, but the actual operational registry should be per-instance, populated at `DefaultRunner` construction time. This would enable parallel test execution and multi-tenant embedding.

---

### EVO-03: DAL Code Duplication for Dual-Backend Support (Major)

**Location:** `dal/unified/` -- all 15+ sub-modules (task_outbox.rs, pipeline_execution.rs, task_execution/, context.rs, schedule/, checkpoint.rs, etc.)

**Description:** Each DAL method is implemented as a pair: `method_postgres()` and `method_sqlite()`, dispatched via a `dispatch_backend!` macro. These are not shared through a trait -- they are structurally duplicated code blocks with nearly identical logic, differing only in connection acquisition and occasional type conversion. A representative example: `TaskOutboxDAL::create` calls either `create_postgres` or `create_sqlite`, each doing the same insert but through different pool types.

**Impact:** Every schema change requires modification in two places. The code is roughly doubled. The two branches can drift subtly (e.g., different error handling, different transaction boundaries). Adding a third backend would require tripling every method. The `backend_dispatch!`, `connection_match!`, and `dispatch_backend!` macros manage the `#[cfg]` complexity but do not eliminate the duplication.

**Recommendation:** Consider whether Diesel's `MultiConnection` can be used more directly to write a single query that dispatches at the connection level rather than the method level. Alternatively, extract the common logic into a generic helper that is parameterized over the pool/connection type. The universal type system (`DbUuid`, `DbTimestamp`, etc.) already exists -- the DAL should be able to use it to write queries once.

---

### EVO-04: Error Type Fragmentation Between Crates (Minor)

**Location:** `cloacina-workflow/src/error.rs` defines `TaskError`, `ContextError`, `CheckpointError`. `cloacina/src/error.rs` re-exports these and adds `ExecutorError`, `ValidationError`, `WorkflowError`, `RegistrationError`, `SubgraphError`. `cloacina/src/registry/error.rs` adds `RegistryError`, `LoaderError`, `StorageError`. `cloacina/src/computation_graph/` has `GraphError`, `AccumulatorError`. `cloacina/src/dispatcher/types.rs` has `DispatchError`.

**Description:** The error hierarchy spans at least 3 crates and 4 files, with manual `From` conversions between types (e.g., `ContextError -> TaskError` uses a lossy mapping that converts `Database` errors to `KeyNotFound` with a string prefix). The `ValidationError` enum contains 14 variants spanning dependency validation, execution failures, scheduling, trigger rules, recovery, database connections, and context errors -- a clear multi-concern enum.

**Impact:** Adding a new error scenario requires understanding which error type it belongs in, which `From` conversions exist, and whether the conversion is lossy. The `ContextError::Database -> TaskError::ContextError { task_id: "unknown" }` conversion (line 354-378 of error.rs) loses the actual error provenance. `ValidationError` is used as a catch-all in contexts where a more specific error type would be appropriate.

**Recommendation:** Split `ValidationError` into domain-specific error types. Fix the lossy `From<ContextError> for TaskError` conversion to preserve database error information rather than mapping to `KeyNotFound`. Consider a unified error type with `#[from]` chains that preserve context.

---

### EVO-05: Python Bindings Runner Is a Parallel Implementation (Minor)

**Location:** `python/bindings/runner.rs` (2,888 lines -- largest file in the crate)

**Description:** The Python `PyDefaultRunner` is not a thin wrapper around `DefaultRunner`. It spawns its own Tokio runtime on a separate thread, maintains its own channel-based message passing protocol (`RuntimeMessage` enum with 20+ variants), and reimplements the coordination logic. Each `DefaultRunner` method that should be exposed to Python requires: (a) a new `RuntimeMessage` variant, (b) a new handler in the runtime loop, (c) a new `#[pymethods]` method on `PyDefaultRunner`, and (d) Python-specific error mapping.

**Impact:** Adding a new capability to `DefaultRunner` (e.g., a new method) requires parallel changes in the Python bindings. The message-passing architecture means that error contexts are lost across the channel boundary. The 2,888-line file is the largest in the crate and difficult to navigate.

**Recommendation:** Consider whether `pyo3-asyncio` or a similar library could bridge the Rust async runtime to Python more generically, reducing the message-passing boilerplate. At minimum, consider splitting `runner.rs` into `runtime_bridge.rs` (the Tokio thread + channel machinery) and `runner_methods.rs` (the individual method implementations).

---

### EVO-06: No Trait Abstraction for the DAL Itself (Minor)

**Location:** `dal/unified/mod.rs`

**Description:** The `DAL` struct is a concrete type, not a trait. All consumers take `DAL` by value or reference. The `DefaultRunner`, `Scheduler`, `TaskScheduler`, `PipelineExecutor`, and `DefaultDispatcher` all depend directly on the concrete `DAL` struct. This means:
- The DAL cannot be mocked in unit tests without a real database
- There is no seam for introducing caching, metrics, or audit logging between the caller and the database
- Alternative storage backends (e.g., an in-memory DAL for testing) cannot be substituted

**Impact:** Integration tests require real databases (Postgres or SQLite). The test fixture (`fixtures.rs`) goes to significant lengths to manage database setup/teardown, schema isolation, and pool management. A trait-based DAL would allow unit tests to use a mock, eliminating database dependencies for logic-level testing.

**Recommendation:** Extract a `DalOperations` trait (or a set of per-entity traits) that `DAL` implements. Scheduler, executor, and runner should depend on the trait, not the concrete struct. This enables mock-based unit tests and decorating patterns (caching, metrics).

---

### EVO-07: Tight Coupling Between Scheduler Layers (Observation)

**Location:** `scheduler.rs` (unified scheduler), `task_scheduler/` (lower-level scheduler loop), `runner/default_runner/services.rs` (background service management)

**Description:** There are three scheduling-related components: the `Scheduler` (unified cron + trigger scheduler), the `TaskScheduler` (lower-level scheduling loop with state management), and the runner's service management layer. The `DefaultRunner` holds references to all three. The `Scheduler` calls into the `PipelineExecutor` directly. The `TaskScheduler` manages the outbox pattern independently.

**Impact:** Modifying the scheduling strategy requires understanding the interaction between three distinct components. The boundaries between "what decides when to run" (Scheduler), "what manages task state" (TaskScheduler), and "what coordinates everything" (DefaultRunner) are not always clear from the code.

**Recommendation:** Document the responsibility boundaries between these three layers explicitly. Consider whether `TaskScheduler` and `Scheduler` could be unified or whether their boundaries could be clarified through a shared trait.

---

### EVO-08: Feature Flag Complexity in Database Layer (Observation)

**Location:** `database/connection/mod.rs`, `database/mod.rs`, `dal/unified/mod.rs`

**Description:** The database and DAL modules use three-way `#[cfg]` patterns extensively:
- `#[cfg(all(feature = "postgres", feature = "sqlite"))]` -- both backends
- `#[cfg(all(feature = "postgres", not(feature = "sqlite")))]` -- postgres only
- `#[cfg(all(feature = "sqlite", not(feature = "postgres")))]` -- sqlite only

This pattern appears in `Database::try_new_with_schema`, `Database::run_migrations`, `create_work_distributor`, and throughout the DAL. The `run_migrations` function at the module level only exists when exactly one backend is enabled. Legacy type aliases (`DbConnection`, `DbPool`) exist only in single-backend mode.

**Impact:** The feature flag combinations create a matrix of compilation modes that is difficult to test exhaustively. Code that compiles with `postgres+sqlite` may not compile with `sqlite` alone due to missing type definitions. The legacy aliases suggest incomplete migration from single-backend to multi-backend.

**Recommendation:** Clean up the legacy type aliases. Standardize on `AnyPool` and `AnyConnection` everywhere. Consider whether the single-backend compilation paths add enough value to justify their maintenance burden.

---

### EVO-09: Crate Dependency on fidius 0.0.5 (Observation)

**Location:** `Cargo.toml` -- `fidius-host = "0.0.5"`, `fidius-core = "0.0.5"`, used in `cloacina-workflow-plugin`

**Description:** The plugin system depends on `fidius` version 0.0.5, which is a pre-1.0 crate. The `CloacinaPlugin` trait with `#[plugin_interface(version = 1)]` defines the FFI boundary. The `PluginHandleCache` in the core crate caches loaded plugins and never `dlclose`s them to avoid linked-list corruption.

**Impact:** Pre-1.0 semver means breaking changes are expected. Any fidius upgrade could break the FFI ABI, requiring coordinated updates to all existing `.cloacina` packages, the plugin interface crate, and the host loading code. The "never dlclose" strategy means memory leaks when hot-reloading plugins. The FFI boundary is the most version-sensitive seam in the architecture.

**Recommendation:** Pin the fidius version precisely and document the ABI compatibility contract. Consider whether the interface version mechanism (`version = 1`) is sufficient for forward/backward compatibility. Plan for the eventual fidius 1.0 migration.

---

### EVO-10: Test Architecture Supports Refactoring Moderately Well (Observation)

**Location:** `crates/cloacina/tests/` (integration tests), inline `#[cfg(test)]` modules throughout

**Description:** The test architecture has several positive patterns:
- `backend_test!` macro enables writing tests once that run on all backends
- `TestFixture` with `get_all_fixtures()` supports parameterized backend testing
- `poll_until` helper replaces fragile `sleep()` calls
- The `cloacina-testing` crate provides a lightweight test runner without database dependencies

However, the heavy reliance on `#[serial]` (160 instances) due to global registries means tests run slowly. Integration tests require real databases (Postgres via Docker, SQLite in-memory). There is no mock-based DAL testing layer.

**Impact:** Test suite execution time scales linearly due to serial execution. Refactoring database-touching code requires an available database, even for logic-level changes. The `#[serial]` requirement means CI cannot parallelize test execution within a crate.

**Recommendation:** This is largely a consequence of EVO-02 (global registries) and EVO-06 (no DAL trait). Addressing those issues would enable parallel test execution and mock-based testing, significantly improving the feedback loop during refactoring.
