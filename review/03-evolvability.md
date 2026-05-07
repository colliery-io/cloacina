# Evolvability Review

## Summary

Cloacina has invested heavily in structural evolvability — the workspace is split into 11 crates with a deliberate authoring/runtime/binary boundary, and the unified package shell macro (CLOACI-I-0102, in flight on `i-0102-fidius-and-plugin-shell`) is exactly the kind of "make the next change cheaper" architectural move you want to see. Recent git history (last 6 months: 275 commits) shows a healthy pattern of structural refactors taking precedence over feature breadth — `T-0509` removed process-globals, `T-0529` extracted `cloacina-python` from the engine, `T-0483` extracted `ServiceManager` from `DefaultRunner`, `T-0528` renamed reactor/CG drift, the I-0102 shell-macro initiative is mid-rollout. The team is clearly able to refactor at the architectural layer.

That said, the core engine crate `cloacina` has accreted a lot of weight (~63K LOC) and its boundaries with downstream code are *thinner than they look*. Multiple changes that should be local actually require touching the engine: every new DAL operation requires writing two near-identical Postgres+SQLite functions (~218 such pairs today, 141 dispatch sites); every Python surface change passes through a 28-variant `RuntimeMessage` enum in `cloacina-python/src/bindings/runner.rs`; every plugin-ABI extension needs careful coordination across the canonical method-index constants, the `CloacinaPlugin` trait, and the unified shell macro body. The reconciler's `loading.rs` is 2,458 lines of intricate per-language branching that has been rewritten three times in the past six months (T-0549 → T-0553 → T-0554 → T-0556). And there's structural drift between Postgres and SQLite migrations (auth tables exist only on Postgres) that the `auth = ["postgres"]` cargo feature paper covers without resolving.

The biggest single evolvability cliff is the **fidius-based plugin ABI**: it's pinned by positional method indices, hand-coordinated wire-format choices (debug = JSON / release = bincode), and a `cloacina::package!()` macro that emits 250+ lines of code per cdylib. Adding a tenth method to `CloacinaPlugin` is a coordinated change across the trait, the macro body, the host call sites, the wire-format types, and at minimum the in-tree fixture crates — and the moment a third-party cdylib ships, that coordination becomes much harder. The `#[optional(since = N)]` mechanism mitigates this somewhat, but the project has no visible deprecation/migration policy beyond "pre-1.0 atomic migration." That works today; it won't post-1.0.

## Architecture Assessment

**Boundaries (good).** The crate split is real and the discipline holds. `cloacina-workflow`, `cloacina-computation-graph`, and `cloacina-workflow-plugin` are the three "leaf" crates a packaged cdylib can compile against, and per `cargo metadata`/`cargo tree` they pull in zero diesel, zero pyo3, zero rdkafka. The `cloacina-compiler` binary deliberately does not depend on `cloacina-python` (CLOACI-T-0529) and verified to be pyo3-free. Examples are excluded from the workspace root so they compile against the leaf surface, simulating the third-party experience.

**Dependency graph (acyclic, mostly clean).** No cycles. Layering: `cloacina-computation-graph` → `cloacina-workflow-plugin`; `cloacina-workflow` (with `cloacina-macros`) → `cloacina-workflow-plugin`; engine `cloacina` consumes all four; `cloacina-python` and `cloacina-server` consume `cloacina`; `cloacina-compiler` and `cloacinactl` consume `cloacina` minus Python. The `cloacina-server → cloacina-python` edge is real — the server statically links pyo3 because it must register the Python runtime at startup (`cloacina_python::install()`). That's an architectural decision, not an accident.

**Cohesion (mixed).** The engine crate has roughly 18 top-level modules, several of which are genuinely large (`computation_graph/scheduler.rs` 1,350 LOC; `registry/reconciler/loading.rs` 2,458 LOC; `dal/unified/task_execution_metadata.rs` 1,158 LOC; `security/db_key_manager.rs` 1,835 LOC). `loading.rs` in particular conflates per-language branching, FFI loading, runtime registration, and view-building — and has been rewritten three times in six months. The `dal/unified/` subtree has very high paired-function repetition (every CRUD method is duplicated for postgres + sqlite).

**Coupling (the migration-shaped pieces are the worst).** Three high-coupling shapes dominate: (1) the runtime/macro/plugin-shell triangle that depends on positional method indices and wire-format conventions, (2) the per-backend DAL duplication, and (3) the Python/Rust runtime bridge with its 28-variant message enum. None are hard to evolve in isolation; they're hard to evolve *quickly* because each change ripples through several files.

**Test architecture (mixed).** Tests reach into engine internals (`cloacina::computation_graph::accumulator::*`, `cloacina::computation_graph::reactor::*`, `cloacina::computation_graph::registry::EndpointRegistry`) — a refactor of those modules will break tests even when the public API is preserved. 108 `#[serial]` annotations indicate substantial shared global state in test DBs / metric recorders. The `cloacina-testing` crate is small (6 files) and well-scoped — it's a TestRunner harness for unit-testing tasks, not a fixture/mock surface for integration tests, which limits its leverage.

## Change Cost Analysis

### Adding a new task type (e.g., a "checkpoint task" with custom semantics)

**Blast radius: small if hand-rolled, large if macro-supported.** Implementing a new `Task` impl is just `cloacina_workflow::Task` — a single trait, single crate. But integrating with the macro layer (`#[task(kind = "checkpoint")]`) means touching `cloacina-macros/src/tasks.rs` (1,047 LOC), the `TaskEntry` inventory shape in `cloacina-workflow-plugin/src/inventory_entries.rs`, and the unified shell's `execute_task` body in `cloacina-workflow-plugin/src/lib.rs:208-278`. Touch count: 3 crates if macro-aware, 1 if not.

### Adding a new trigger (e.g., a webhook trigger)

**Blast radius: small.** `Trigger` trait lives in `cloacina-workflow/src/trigger.rs:91`. Implementing a new trigger type is a single-crate change. The cron/poll distinction is handled at the scheduler layer (`cron_trigger_scheduler.rs`) via `Trigger::cron_expression() -> Option<&str>`. Macro-supported triggers go through `trigger_attr.rs` and emit a `TriggerEntry` — adding a new shape (e.g., one with a different lifecycle) means modifying the macro and the shell's `invoke_trigger_poll`. The current shape supports cron + poll cleanly; adding a "push" trigger that doesn't poll would require API extensions — `Trigger` has no "subscribe" surface today.

### Adding a new database backend (e.g., MySQL)

**Blast radius: very large, ~218 paired functions.** Every DAL module has `_postgres` and `_sqlite` variants (`grep "fn .*_postgres\|fn .*_sqlite" | wc -l` = 218). The `dispatch_backend!` macro has three branches (both/postgres-only/sqlite-only); adding a third backend requires extending the macro, the `BackendType` enum, and writing ~110 new functions. Universal types (`UniversalUuid`, `UniversalTimestamp`, `UniversalBool`) need MySQL handling. Migrations are per-backend directories (`crates/cloacina/src/database/migrations/{postgres,sqlite}/`) and would gain a third. The Postgres migration set has 22 entries vs SQLite's 19 — auth tables already drift. Adding MySQL is a multi-week project and would expose every existing place where Postgres-only features (`feature = "auth"`) live.

### Swapping the DAL (e.g., from diesel to sqlx)

**Blast radius: catastrophic.** Diesel models are deeply embedded — every CRUD method has `diesel::insert_into(table).values(...).execute(conn)` style code. The `database/schema.rs` is `diesel::table!` macro-generated. The unified DAL is a *thin* abstraction over diesel; it does not abstract over the query builder. Replacing diesel means rewriting all 218 paired functions plus ~30 model files. This is a "no" change today; it's been observed but not actioned in `00-system-overview.md` Open Question 1 / earlier ADR notes.

### Adding a new HTTP endpoint

**Blast radius: small if read-only, medium if mutation.** Read endpoints follow a simple pattern: `crates/cloacina-server/src/routes/<resource>.rs`, register in `build_router`, use the existing `AuthenticatedKey` extension and `ApiError` envelope. Mutation endpoints that touch the runner state are tougher — see `executions.rs:50` which has the candid TODO comment that "Full multi-tenant execute_workflow requires per-tenant runners or a runner that accepts a Database override." The shared-runner-vs-tenant-runner mismatch is a structural gap, not just a route issue.

### Evolving the plugin ABI (e.g., adding a 10th method)

**Blast radius: medium-to-large, with a sharp version-skew cliff.** Today: add a method to the `CloacinaPlugin` trait in `cloacina-workflow-plugin/src/lib.rs:712-807`, declare a `METHOD_*` constant (lib.rs:682-698), implement it in the unified shell macro body (`package!()` in lib.rs:110-673), update host call sites (`crates/cloacina/src/registry/reconciler/loading.rs`, `crates/cloacina/src/registry/loader/package_loader.rs`), and ensure the new method is `#[optional(since = N)]` so old plugins return `CallError::NotImplemented`. In-tree fixtures rebuild automatically. **Out-of-tree cdylibs** built against an older `cloacina-workflow-plugin` won't expose the new method, but the optional bit handles that. Adding a *required* method is a hard ABI break — there is no documented deprecation policy, and pre-1.0 the answer is "rebuild everything atomically." The `package!()` macro body is monolithic — it doesn't compose, so every method addition means another 30-60 lines inside the same macro definition. **The unified shell macro is itself the high-cost surface for plugin-ABI evolution.**

### Migrating to a new execution paradigm (extending the reactor + computation-graph rollout)

**Blast radius: contained to the engine but spans 6 modules.** Today there are two execution paradigms — the workflow/task DAG and the computation-graph traversal — coexisting cleanly through `Runtime`'s seven separate registries (tasks, workflows, triggers, CGs, trigger-less CGs, reactors, stream backends). Adding a third paradigm would mean: a new constructor type in `runtime.rs:53-65`, a new register/unregister/get triple, a new inventory entry type in `inventory_entries.rs`, a new step in the reconciler's precedence-ordered loader (`loading.rs:251-306`), a new section in the unified shell macro, and a new `cloacina-server` route group. The `Runtime` registry shape supports this — it's not a hidden rewrite — but the per-step duplication compounds across the load/unload/registry/scheduler axes.

## Findings

### EVO-01: Plugin ABI evolution is monolithic — `package!()` macro body is the single high-cost surface

**Severity**: Major
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:110-673` (the `package!()` macro body)
**Confidence**: High

#### Description

The `cloacina::package!()` macro emits a 560-line `mod _ffi { ... }` block per packaged cdylib that hand-implements all nine `CloacinaPlugin` methods inline. Adding a tenth method (or restructuring an existing one) means another 30-60 lines inside the same macro definition, and every cdylib must be rebuilt to pick up the change. There is no composability — you can't add a method by extending a per-feature macro and combining; everything is one indivisible expansion. The wire format (debug=JSON, release=bincode per `feedback_fidius_wire_format.md` memory) is a hand-coordinated convention that doesn't appear in code anywhere — it's enforced by fidius itself, but a new author has no compile-time signal of which wire format is in effect.

#### Evidence

`crates/cloacina-workflow-plugin/src/lib.rs:110-673` — single `macro_rules! package` with body containing inline impls for all 9 methods (METHOD_GET_TASK_METADATA through METHOD_INVOKE_TRIGGERLESS_GRAPH). Each method block creates its own `OnceLock<Runtime>` (lines 214-225, 331-341, 477-487, 574-584) — four separate cdylib runtime initializations duplicated. Comment at lines 99-108 acknowledges the linker conflict risk: "T-A keeps per-macro `_ffi` emission alongside the new shell; T-C strips the old path. The current branch (T-A territory) might have both active." That mid-flight state is a real evolvability hazard.

#### Suggested Resolution

Decompose the `package!()` body into per-method helper macros: `__cloacina_emit_get_task_metadata!()`, `__cloacina_emit_execute_graph!()`, etc. The shell becomes the orchestrator that calls each helper. Adding a method becomes: write a new helper macro and add one line to the shell. Keep a single shared `cdylib_runtime!()` macro that all four execute-* methods reuse. This also gives third-party authors an extension point: a future "custom plugin method" feature could call a user-supplied helper in the same shape.

**Cross-cutting note**: the Correctness lens may flag the four separate `OnceLock` runtime initializations as a hazard — task and trigger and CG and trigger-less-CG each spawn their own 2-thread runtime, so a cdylib has up to 8 worker threads running just to bridge FFI even when no work is in flight.

### EVO-02: DAL paired-function pattern doubles every backend operation

**Severity**: Major
**Location**: `crates/cloacina/src/dal/unified/` (entire subtree)
**Confidence**: High

#### Description

The unified DAL chooses backend at runtime via the `dispatch_backend!` macro, but every public DAL method has two near-identical private implementations: `<op>_postgres` and `<op>_sqlite`. The bodies are typically 90% identical because diesel's query builder is portable across the two backends. By count: 218 `*_postgres`/`*_sqlite` paired functions and 141 `dispatch_backend!` invocations. Adding a third backend would require ~110 new functions and a three-arm dispatcher. Every DAL change today must be made in two places — and the test suite has to validate both paths.

#### Evidence

`crates/cloacina/src/dal/unified/recovery_event.rs:54-147` — `create()` is 11 lines; `create_postgres()` is 39 lines; `create_sqlite()` is 39 lines — and the bodies differ only in `get_postgres_connection()` vs `get_sqlite_connection()` plus the `#[cfg(...)]` gate. `crates/cloacina/src/dal/unified/task_execution/state.rs:54-1033` has 14 paired functions (`mark_completed`, `mark_failed`, `mark_ready`, `mark_skipped`, `mark_abandoned`, `set_sub_status`, `reset_retry_state`) doubled. Migration sets diverge: postgres has 22 directories, sqlite has 19, with auth-related migrations missing on sqlite (`016_create_api_keys`, `019_add_tenant_and_admin_to_api_keys`, `003_standardize_uuid_generation`).

#### Suggested Resolution

Diesel's `MultiConnection` derive (mentioned in the module doc at `dal/unified/mod.rs:22-27`) is the intended path but isn't applied — the doc says it but the code dispatches manually. A real adoption of `MultiConnection` would let one body serve both backends. Short of that: extract the connection-acquisition step into a helper and centralize the per-backend `cfg`-gated module dispatch. For migrations, audit the divergence — every postgres-only migration should be either ported to SQLite or explicitly documented as feature-`auth`-gated. The `auth = ["postgres"]` Cargo feature acknowledges this but doesn't surface why SQLite can't have API keys.

### EVO-03: Reconciler loading pipeline is a known refactor target — 2,458 lines, three rewrites in six months

**Severity**: Major
**Location**: `crates/cloacina/src/registry/reconciler/loading.rs` (2,458 LOC, second largest non-test file)
**Confidence**: High

#### Description

`load_package` is the canonical example of an in-flight refactor target. The function is 540+ lines of intricate per-language branching with five major phases: archive write, unpack, manifest load, language dispatch (rust/python-workflow/python-CG), and post-load registration. The post-I-0102 precedence-ordered pipeline (`step_load_cron_triggers → step_load_custom_triggers → step_load_reactors → step_load_triggerless_cgs → step_load_reactor_bound_cgs → step_load_workflows`) is the right shape — but the current code has both the legacy and the new shape coexisting, with comments throughout (lines 218, 234, 251, 320, 367, 420, 432, 596, 618, 622) explaining what's transitional. T-0549 → T-0553 → T-0554 → T-0556 are four cleanup waves over six months, each removing dead branches uncovered by the previous wave. The function will need at least one more pass to settle.

#### Evidence

Git log: `e79ac63 T-0556: excise dead branches in reconciler load_package + scheduler`, `33fc49d T-0554 Phase 2 finalization: Python paths route through unified pipeline`, `0fb953b T-0554 Phase 1: precedence-ordered pipeline for Rust load path`, `b521316 T-0557 Bug 2, 5: signature verification at upload + unified Python TriggerResult API` — all touching loading.rs. The file currently has 6 helper functions spanning lines 1407-1885 (`step_load_*`) plus the main `load_package` and `unload_package` at lines 111-665 and 665-840. Comments at lines 251-256 and 596-616 acknowledge the cross-language branching is messy.

#### Suggested Resolution

This is the right structural target for the next refactor wave. The `step_load_*` helpers should become methods on a `PackageLoadView` type that hides the rust-vs-python-branching from the orchestrator. The `build_view_rust` and `build_view_python` are halfway there — the next step is to make `load_package` itself language-agnostic, with a single `let view = build_view(language, archive)?;` call followed by the unchanged precedence-ordered step calls. That refactor is in the spirit of CLOACI-I-0102's goal — symmetric authoring, symmetric loading.

### EVO-04: Multi-tenant execution scoping is a structural gap, not a configuration choice

**Severity**: Major
**Location**: `crates/cloacina-server/src/lib.rs:43-92` (TenantDatabaseCache), `crates/cloacina-server/src/routes/executions.rs:43-99` (execute_workflow handler)
**Confidence**: High

#### Description

`cloacina-server` has a `TenantDatabaseCache` that lazily creates per-tenant `Database` instances for *list/inspect* operations, but the actual `DefaultRunner` that executes workflows is constructed once at server startup with the admin schema. The `execute_workflow` handler at line 74 calls `state.runner.execute_async(&name, context)` — that runner is shared across all tenants. The candid TODO at `executions.rs:43-49` says "Full multi-tenant execute_workflow requires per-tenant runners or a runner that accepts a Database override." This is an architectural mismatch: the storage layer has been made multi-tenant aware, but the execution layer hasn't.

#### Evidence

`crates/cloacina-server/src/lib.rs:97-109` — `AppState` has both `runner: Arc<DefaultRunner>` (single, server-wide) and `tenant_databases: Arc<TenantDatabaseCache>` (per-tenant). `crates/cloacina-server/src/routes/executions.rs:74` — uses `state.runner.execute_async`. `crates/cloacina-server/src/routes/executions.rs:111-120` — uses `state.tenant_databases.resolve(...)`. The two paths are inconsistent within the same file.

#### Suggested Resolution

Two paths: (a) Make the `Runtime` argument to a `DefaultRunner` shared across per-tenant runner instances (the registries are inventory-seeded and tenant-agnostic, so this works), and the `Database` per-tenant. The `with_config` constructor already takes a `DatabaseUrl` — extend with a `with_database` variant that accepts an existing `Database`. Cache constructed runners alongside DBs in `TenantDatabaseCache`. (b) Make `execute_async` accept a `Database` override so a single runner serves all tenants. Option (a) is more work but cleaner; option (b) is the kind of "wedge" that creates more multi-tenant gaps later.

### EVO-05: Python runner bridge is a 28-variant message enum — every async API addition costs 4-6 files

**Severity**: Major
**Location**: `crates/cloacina-python/src/bindings/runner.rs` (1,638 LOC; `RuntimeMessage` enum)
**Confidence**: High

#### Description

`PyDefaultRunner` runs the cloacina engine on a dedicated thread because pyo3's GIL cannot be held by a tokio worker thread. Communication is via `mpsc::Sender<RuntimeMessage>` + per-call `oneshot::Sender<Result<T, E>>`. The `RuntimeMessage` enum has 28 variants today — `Execute`, `RegisterCronWorkflow`, `ListCronSchedules`, `SetCronScheduleEnabled`, `DeleteCronSchedule`, `GetCronSchedule`, `UpdateCronSchedule`, `GetCronExecutionHistory`, `GetCronExecutionStats`, `ListTriggerSchedules`, `GetTriggerSchedule`, `SetTriggerEnabled`, `GetTriggerExecutionHistory`, etc. Every Rust API the Python user wants exposed needs: a new variant, a match arm in the runtime thread, a Python method on `PyDefaultRunner` that constructs and sends, and the request/response type. Every API change to the Rust side ripples into the Python bindings via this enum.

#### Evidence

`crates/cloacina-python/src/bindings/runner.rs:48-200+` — `enum RuntimeMessage` definition spans hundreds of lines with `oneshot::Sender<...>` per variant. `grep -c "RuntimeMessage::"` returns 28.

#### Suggested Resolution

Two reasonable directions: (a) shrink `RuntimeMessage` to a single variant `Execute(BoxedFuture)` and pass futures through the channel instead of named operations — let the Python wrapper construct closures that capture engine method calls. This requires the engine methods to all be `Send + Future + 'static`, which they should be. (b) Generate the bindings — a proc-macro on the engine API that emits the message variant + the Python method + the runtime-thread match arm. Option (a) makes more sense for maintainability; option (b) is the bigger investment but pays off if the Python surface keeps expanding.

### EVO-06: `cloacina-workflow` claims to be "minimal" but pulls in tokio multi-thread runtime

**Severity**: Minor
**Location**: `crates/cloacina-workflow/Cargo.toml:30-45`
**Confidence**: High

#### Description

The crate doc says "minimal types for authoring Cloacina workflows without runtime dependencies like database drivers" — but the default features include `macros`, which pulls in `tokio = { version = "1", features = ["rt-multi-thread", "time"] }`. The reason: the `package!()` macro body needs `tokio::runtime::Runtime::new` to bridge FFI calls. So the "minimal" crate is actually pulling a tokio multi-thread runtime, which is the second-largest single dependency in the leaf crates (after diesel and pyo3, neither of which it has). Authors who depend on `cloacina-workflow` without macros (e.g., to ship hand-rolled `Task` impls) can disable the feature, but the default story doesn't deliver on the "minimal" promise.

#### Evidence

`cloacina-workflow/Cargo.toml:31`: `macros = ["cloacina-macros", "tokio"]`. `cloacina-workflow/Cargo.toml:45`: `tokio = { version = "1", features = ["rt-multi-thread", "time"], optional = true }`. `cloacina-workflow/src/lib.rs:88-93`: `pub mod __private { pub use tokio; }` — the leaf crate re-exports tokio so the macro can construct runtimes.

#### Suggested Resolution

Two paths: (a) extract the cdylib runtime initialization into a separate crate `cloacina-cdylib-runtime` that pulls tokio; `cloacina-workflow` stays free of tokio; the `package!()` macro depends on both. (b) Accept the situation but rename the crate doc to be honest: "minimal authoring surface, runtime-light" rather than "without runtime dependencies."

### EVO-07: Tests reach into engine internals, locking abstractions in place

**Severity**: Major
**Location**: `crates/cloacina/tests/integration/computation_graph.rs` and most CG-related tests
**Confidence**: High

#### Description

Integration tests directly import private-ish module paths: `cloacina::computation_graph::accumulator::{health_channel, AccumulatorHealth}`, `cloacina::computation_graph::reactor::{ManualCommand}`, `cloacina::computation_graph::registry::EndpointRegistry`, `cloacina::computation_graph::scheduler::{ComputationGraphScheduler}`. Several of these are listed as `pub` in `lib.rs` re-exports but the test setup creates accumulators / reactors / endpoints by hand using internal types. A refactor that hides `EndpointRegistry` or restructures `accumulator_runtime` will break these tests even when the user-facing public API is unchanged. There are 2,458 lines of CG integration test code (`tests/integration/computation_graph.rs` is 3,070 LOC including these patterns).

#### Evidence

`crates/cloacina/tests/integration/computation_graph.rs:208-212`: imports from `cloacina::computation_graph::accumulator` directly. Lines 345-346: imports `EndpointRegistry`. Line 473: imports `ManualCommand`. These types are the runtime-internal mechanisms that the public API hides.

#### Suggested Resolution

Establish a `cloacina-testing-cg` (or extend `cloacina-testing`) with documented test helpers — `make_test_reactor()`, `make_test_accumulator()`, `make_test_endpoint_registry()` — that take stable arguments and produce ready-to-use values. Refactor the tests to use that surface. The 108 `#[serial]` annotations in the test suite suggest the metric recorder + DB pool also need a test-architecture rework — `serial_test` is a sign of shared mutable state, not a clean fixture pattern.

### EVO-08: Process-global `python_runtime` slot is the "intentional" coupling between engine and bindings

**Severity**: Observation
**Location**: `crates/cloacina/src/python_runtime.rs:82` (`OnceLock<Arc<dyn PythonRuntime>>`)
**Confidence**: High

#### Description

The engine has exactly one process-global static: `PYTHON_RUNTIME: OnceLock<Arc<dyn PythonRuntime>>`. The server calls `cloacina_python::install()` at startup; the compiler service deliberately does not. This is documented carefully (CLOACI-T-0529) and works — but it's a single-instance global, which means: (a) you can't run two server instances in the same process with different Python configurations, (b) testing the Python load path requires careful test ordering or `serial_test`, (c) the abstraction is over a *trait object*, not a value, so calls go through `Arc<dyn>` virtual dispatch. The pattern is a *deliberate* and *bounded* coupling, but it's worth noting because adding more process-global slots (e.g., for a different language plugin) would amplify the test-isolation problem.

#### Evidence

`crates/cloacina/src/python_runtime.rs:82`: `static PYTHON_RUNTIME: OnceLock<Arc<dyn PythonRuntime>> = OnceLock::new();`. `crates/cloacina-server/src/lib.rs:125`: `cloacina_python::install();`. The doc comment at `python_runtime.rs:25-29` says "Only the first call wins — subsequent calls are silently ignored."

#### Suggested Resolution

Watch this for divergence. If a third "language runtime" is ever added (Lua? Wasm?), refactor to a runtime registry keyed by language tag rather than another OnceLock. The structure is a textbook plugin-host pattern; the only failure mode is "more of these get added without consolidation."

### EVO-09: Migration drift between Postgres and SQLite is structural, not just feature-gated

**Severity**: Major
**Location**: `crates/cloacina/src/database/migrations/postgres/` (22 migrations) vs `crates/cloacina/src/database/migrations/sqlite/` (19 migrations)
**Confidence**: High

#### Description

Postgres has three additional migrations that SQLite does not — `003_standardize_uuid_generation`, `016_create_api_keys`, `019_add_tenant_and_admin_to_api_keys`. The numbering after these diverge: postgres `017_create_computation_graph_state_tables` corresponds to sqlite `015_create_computation_graph_state_tables`. The drift is real schema drift, not just naming. The `auth = ["postgres"]` Cargo feature gates the API keys at compile time, which works — but the migration directories aren't structured to make that clear: a future contributor might miss that adding a migration for a feature-gated table requires only the postgres directory. The per-memory note (`feedback_sqlite_migration_recreate.md`) explicitly warns about SQLite migration constraints.

#### Evidence

`diff <(ls postgres/) <(ls sqlite/)` shows three postgres-only migrations: `standardize_uuid_generation`, `create_api_keys`, `add_tenant_and_admin_to_api_keys`. The numbering then offsets by 3 throughout. SQLite never gets API keys today.

#### Suggested Resolution

Either port the API key tables to SQLite (the `Universal*` types should support this) or split the auth migrations into a separate `migrations/postgres-auth/` directory that's only loaded when `feature = "auth"` is on. The current arrangement makes the divergence implicit and easy to miss. If SQLite-auth is genuinely out of scope, document it in `dal/unified/mod.rs` near where `api_keys` is `#[cfg(feature = "postgres")]`.

### EVO-10: The unified shell macro and per-macro `_ffi` emission can coexist (and conflict) on a single branch

**Severity**: Major (during transition)
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs:99-108` (the macro doc itself acknowledges this), I-0102 transition state
**Confidence**: High

#### Description

The unified `cloacina::package!()` macro emits a `pub mod _ffi { ... }` containing a `fidius_plugin_registry!()` invocation. The legacy `#[computation_graph]` and `#[workflow]` macros also emit a `_ffi` module with their own `fidius_plugin_registry!()`. A crate that has both `cloacina::package!();` and `#[computation_graph]` produces two `fidius_plugin_registry!()` calls — a linker conflict. The macro documentation at `lib.rs:99-108` explicitly says "T-A keeps per-macro `_ffi` emission alongside the new shell; T-C strips the old path." The branch I'm reviewing (`i-0102-fidius-and-plugin-shell`) is in this transition. Per recent git history, T-0549 Phase 2c (commit 29d91ea) was "strip per-macro _ffi" — the migration is mostly complete, but the docs in the macro itself say it's not. There's a real risk of unit/integration tests passing while a freshly-authored package conflicts.

#### Evidence

`crates/cloacina-workflow-plugin/src/lib.rs:99-108` — comments "Coexistence: in T-A the per-macro `_ffi` emission from `#[computation_graph]` and `#[workflow]` is unchanged. A crate that adds `cloacina::package!();` AND has `#[computation_graph]` / `#[workflow]` would emit two `fidius_plugin_registry!()` calls → linker conflict." Recent commit `29d91ea T-0549 Phase 2c: strip per-macro _ffi + migrate in-tree fixtures` — the per-macro path appears stripped, but the docs in the canonical interface crate haven't been updated to reflect that.

#### Suggested Resolution

Update the `package!()` macro doc to reflect the post-strip state. Add a deny-paths lint (or a runtime check at registration time) for cdylibs that export multiple `fidius_plugin_registry` blocks. The Open Question 10 in `00-system-overview.md` flagged this.

### EVO-11: Workflow registry trait conflates registration, query, and storage concerns

**Severity**: Minor
**Location**: `crates/cloacina/src/registry/traits.rs:64-160` (`WorkflowRegistry` trait)
**Confidence**: Medium

#### Description

`WorkflowRegistry` is a 4-method trait (`register_workflow`, `get_workflow`, `list_workflows`, `unregister_workflow`) but `register_workflow` is responsible for: extracting metadata from binary, validating the package, storing binary data, storing metadata in the database, *and* registering tasks with the global task registry (per the doc at lines 67-73). The fifth concern — task registration — is now handled by the reconciler, not by `register_workflow`. The doc is stale and the trait conflates responsibilities. `RegistryStorage` (lines 195-253) is appropriately narrow (3 methods: `store_binary`, `retrieve_binary`, `delete_binary`). The split is right; the docs aren't.

#### Evidence

`crates/cloacina/src/registry/traits.rs:67-73` — the doc says register_workflow does five things including "Registers tasks with the global task registry," but the actual reconciler (`reconciler/loading.rs`) is what registers tasks. The trait's `register_workflow` only stores. This means a third-party `WorkflowRegistry` impl could rely on the doc and silently break.

#### Suggested Resolution

Update the doc to reflect that `register_workflow` only stores; the reconciler handles registration. Consider splitting the trait into `WorkflowMetadataStore` (CRUD) and `WorkflowRegistry` (the higher-level orchestration). For now: minimum-viable change is the doc fix.

### EVO-12: `cloacina-server` carries pyo3 transitively even when no Python work happens

**Severity**: Observation
**Location**: `crates/cloacina-server/Cargo.toml:33`, `crates/cloacina-python/Cargo.toml`
**Confidence**: High

#### Description

`cloacina-server` depends on `cloacina-python` so it can call `cloacina_python::install()` at startup. This pulls pyo3 + Python embedding into every server binary, even deployments that never receive Python packages. Per `cargo tree -p cloacina-server`, pyo3 is in the dependency graph. Per memory (`feedback_python_is_core.md`): "Python support is a core capability." That's an explicit choice — but it does mean a Python-free server deployment can't be built without modifying the dependency graph. The `cloacina-compiler` story works (no `cloacina-python` dep, no pyo3) — that's the mode of operation that demonstrates the right separation.

#### Evidence

`crates/cloacina-server/Cargo.toml:33`: `cloacina-python = { path = "../cloacina-python", default-features = false }`. `crates/cloacina-server/src/lib.rs:125`: `cloacina_python::install();` — single call site. `cargo tree -p cloacina-server | grep pyo3` shows `pyo3 v0.25.1`.

#### Suggested Resolution

Status quo is defensible per project goals. If a future deployment story needs a Python-free server (smaller binaries, no embedded Python), the lift is small: hide `install()` behind a `feature = "python"` and treat the Python install as opt-in. The compiler service already proves the boundary works.

### EVO-13: Reactor + computation graph rollout is well-staged but the "trigger-less CG" concept duplicates surface

**Severity**: Minor
**Location**: `cloacina_workflow_plugin::TriggerlessGraphRegistration` vs `cloacina_computation_graph::ComputationGraphRegistration`, plus their respective inventory entries
**Confidence**: Medium

#### Description

The CG rollout has produced two parallel registration shapes: `ComputationGraphRegistration` (reactor-bound, in `cloacina-computation-graph`) and `TriggerlessGraphRegistration` (workflow-task-invoked, in `cloacina-workflow-plugin`). Each has its own inventory entry (`ComputationGraphEntry`, `TriggerlessGraphEntry`), its own register/unregister/get methods on `Runtime` (lines 44, 81, etc.), its own step in the reconciler pipeline (`step_load_reactor_bound_cgs`, `step_load_triggerless_cgs`), and its own FFI invocation method on `CloacinaPlugin` (`execute_graph` vs `invoke_triggerless_graph`). The trigger-less variant is a real concept (it's invoked from workflow tasks via `#[task(invokes = computation_graph("name"))]`), but the surface duplication suggests the abstraction could be unified — a `ComputationGraph` with an optional trigger source.

#### Evidence

`crates/cloacina-workflow-plugin/src/inventory_entries.rs:36-69` — `TriggerlessGraphFn` and `TriggerlessGraphRegistration` defined separately from `ComputationGraphRegistration`. `crates/cloacina-workflow-plugin/src/lib.rs:548-667` — `get_triggerless_graph_metadata` and `invoke_triggerless_graph` methods (METHOD_GET_TRIGGERLESS_GRAPH_METADATA = 7, METHOD_INVOKE_TRIGGERLESS_GRAPH = 8) duplicate the shape of `get_graph_metadata` and `execute_graph`. `crates/cloacina/src/runtime.rs:80-83` — `computation_graphs` and `triggerless_graphs` are separate registries.

#### Suggested Resolution

Consider unification in a future cycle: a `ComputationGraph` with `Option<TriggerSource>`. The reconciler's precedence-ordered loader is the right place to dispatch by source. The current shape is workable — but each future CG-related feature has to be added in two places.

### EVO-14: The `ScheduleWakeup` of refactor cycles suggests the architecture is converging — but isn't there yet

**Severity**: Observation
**Location**: Recent git history — T-0509 (remove ctor + global registries), T-0529 (split cloacina-python), T-0483 (extract ServiceManager), T-0528 (rename reactor/CG drift), T-0549/T-0551/T-0553/T-0554/T-0555/T-0556/T-0561/T-0563/T-0565 (cleanup waves)
**Confidence**: High

#### Description

In the past 6 months: 275 commits; the last ~50 are dominated by initiative I-0102 cleanup waves (`T-0549` to `T-0565`), each removing dead code uncovered by the previous wave. T-0509 in 2025 already had to "finish I-0096 cleanup" — meaning a second initiative was needed to clean up after the first. The pattern is healthy in that the team explicitly closes loops; it's a worry in that complete coverage of a refactor seems to take ~5 follow-up tasks beyond the headline initiative. For evolvability: any future initiative should budget for at least 3 cleanup waves.

#### Evidence

Recent commits in last 6 months touching renames/relocations: `T-0552: relocate TriggerEntry`, `T-0549 Phase 1: relocate TaskEntry + ComputationGraphEntry`, `T-0509: finish I-0096 cleanup`, `T-0486: consolidate cloaca angreal namespace`, `T-0483: extract ServiceManager`, `T-0528: rename ReactiveScheduler/reactor-as-graph drift`. All are post-hoc cleanups of architectural decisions.

#### Suggested Resolution

When proposing the next architectural initiative, write down explicit "cleanup task" placeholders alongside the headline task at decomposition time. The team is doing this implicitly already (T-0549 has Phase 1/2a/2b/2c/2d) but a planning convention would help. Also: consider `cargo deny`-style lint rules for "no `pub use` chains across more than 2 hops" or "no `_legacy` / `_old` suffix in module names" to surface drift earlier.

### EVO-15: Server's per-tenant DB cache is read-side only — write-side multi-tenancy is unimplemented

**Severity**: Observation (already cross-referenced in EVO-04)
**Location**: `crates/cloacina-server/src/lib.rs:43-92` (TenantDatabaseCache)
**Confidence**: High

#### Description

This is the same gap as EVO-04, observed from a different angle: the read-side handlers (`list_executions`, etc.) consistently use `state.tenant_databases.resolve(...)`; the write-side (`execute_workflow`) does not. The cache is one-half of the abstraction. As a structural observation, it suggests the cache shape may need to grow to cache `DefaultRunner` instances, not just `Database` instances — see EVO-04's suggested resolution.

### EVO-16: Stream backend extension model is ill-documented, makes adding non-Kafka backends ambiguous

**Severity**: Minor
**Location**: `crates/cloacina/src/computation_graph/stream_backend.rs`, `crates/cloacina/src/inventory_entries.rs:60-64`
**Confidence**: Medium

#### Description

The `StreamBackend` trait + registry supports custom stream backends via `inventory::submit!(StreamBackendEntry { ... })`. There's an in-tree Kafka backend (`feature = "kafka"`) and a `MockBackend`. Open Question 4 in `00-system-overview.md` notes: "The shell's macro body in `cloacina-workflow-plugin/src/lib.rs:128-470` does not appear to" walk `StreamBackendEntry`. So a third-party packaged cdylib that includes `stream_accumulator(type = "redis", ...)` plus a `RedisStreamBackend` impl can't ship as a single `.cloacina` package — the host-side will not see the stream backend. The unified shell skipped this entry type.

#### Evidence

`crates/cloacina-workflow-plugin/src/lib.rs:110-673` — the `package!()` macro body walks `TaskEntry`, `WorkflowDescriptorEntry`, `ComputationGraphEntry`, `ReactorEntry`, `TriggerEntry`, `TriggerlessGraphEntry` but not `StreamBackendEntry`. `crates/cloacina/src/inventory_entries.rs:60-64` — `StreamBackendEntry` is defined in the engine crate, not in `cloacina-workflow-plugin`, so the shell can't reach it.

#### Suggested Resolution

Either relocate `StreamBackendEntry` to `cloacina-workflow-plugin` (matching T-0549's pattern for `TaskEntry`) and add a tenth method to `CloacinaPlugin` for stream-backend metadata + factory bridging, or document that custom stream backends are not packaged-cdylib-supported and must be linked into the host. The current state is implicit — the doc says the shell handles "any combination of declared primitives" but stream backends are silently excluded.

### EVO-17: The `Workflow` struct lives in the engine crate, blocking deeper "minimal authoring" decoupling

**Severity**: Observation
**Location**: `crates/cloacina/src/inventory_entries.rs:46-50` (`WorkflowEntry`), `crates/cloacina/src/workflow/mod.rs` (1,642 LOC)
**Confidence**: High

#### Description

The `WorkflowEntry` inventory type takes `constructor: fn() -> Workflow`, but `Workflow` is a 1,642-line type defined in the engine crate (`cloacina/src/workflow/mod.rs:137`). To compile a `WorkflowEntry`, you need the engine crate. This is why `WorkflowEntry` lives in `cloacina/src/inventory_entries.rs` and not in `cloacina-workflow-plugin/src/inventory_entries.rs` (where `TaskEntry`, `ReactorEntry`, etc. live). The comment at `cloacina-workflow-plugin/src/inventory_entries.rs:31-34` acknowledges this: "`WorkflowEntry` / `StreamBackendEntry` remain in `cloacina/src/inventory_entries.rs` until their constructor return types likewise relocate." So the leaf-crate refactor is not complete — packaged cdylibs that declare a workflow today have to depend on either `cloacina_workflow_plugin::WorkflowDescriptorEntry` (metadata only) or get the actual `Workflow` type elsewhere.

#### Evidence

`crates/cloacina-workflow-plugin/src/inventory_entries.rs:110-120` — `WorkflowDescriptorEntry` is metadata-only (`name`, `description`, `author`, `fingerprint`, `graph_data_json`, `triggers`). It does not carry a `fn() -> Workflow`. The actual `Workflow` constructor relocation has not happened yet.

#### Suggested Resolution

Plan the next phase of the leaf-crate refactor: move `Workflow` (or a thinner authoring representation) to `cloacina-workflow`. The current `Workflow` type is heavy — it has dependency-graph computation, task lookup, validation. A minimal authoring `WorkflowSpec` could live in `cloacina-workflow`, with the engine consuming it via `From<WorkflowSpec>`. This would let packaged cdylibs include the full constructor in their inventory rather than just metadata.

### EVO-18: Drop impl on DefaultRunner cannot run async shutdown — implicit dependency on explicit shutdown call

**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/mod.rs:218-224`
**Confidence**: High

#### Description

`DefaultRunner::Drop` is async-incompatible — it logs a message hoping the user called `shutdown()` explicitly. If the user drops without shutdown: in-flight tasks may abandon, the DB pool isn't closed, and background services keep running until the tokio runtime tears down. This isn't a unique-to-cloacina problem — async Rust + Drop is an open design space — but it does mean any future evolution of "what happens at shutdown" has to thread through every caller's lifecycle, not through Drop.

#### Evidence

`crates/cloacina/src/runner/default_runner/mod.rs:218-224`:
```rust
impl Drop for DefaultRunner {
    fn drop(&mut self) {
        // Note: Can't use async in Drop, but we can attempt shutdown
        // Users should call shutdown() explicitly for graceful shutdown
        tracing::info!("DefaultRunner dropping - consider calling shutdown() explicitly");
    }
}
```

#### Suggested Resolution

Status quo is the typical async-Rust answer. Document the requirement loudly in the rustdoc for `DefaultRunner::new` (the doc at line 79 doesn't currently mention shutdown). Consider a `DefaultRunner::with_shutdown_token` constructor that returns `(DefaultRunner, ShutdownGuard)` where the guard's Drop blocks on shutdown via `tokio::runtime::Handle::current().block_on(...)`. This isn't free — but it makes the lifecycle explicit.

### EVO-19: Macros emit a lot of code per `#[task]` — every task is a 100+ line expansion

**Severity**: Minor
**Location**: `crates/cloacina-macros/src/tasks.rs` (1,047 LOC)
**Confidence**: Medium

#### Description

The `#[task]` macro is the most-used macro and its expansion includes: a struct definition, a `Task` trait impl with `id() / dependencies() / retry_policy() / trigger_rules()`, an `execute()` method, an `inventory::submit!` registration, and (for packaged crates) FFI shimming. Each task is a 100+ line expansion. Compile times for crates with many tasks are noticeably slower; cdylib build times in the compiler service include this overhead. More relevantly for evolvability: changes to the macro propagate to every emitted task body — a change to the `Task` trait shape (e.g., adding a method) means a coordinated update across `cloacina-workflow/src/task.rs`, `cloacina-macros/src/tasks.rs`, and the macro's emitted impls.

#### Evidence

`crates/cloacina-macros/src/tasks.rs:1-1047` — single file, no submodule split; parses `TaskAttributes`, dependency parsing, retry parsing, trigger-rule parsing, code generation. The size is the result of feature accretion (retry_attempts, retry_backoff, retry_delay_ms, retry_max_delay_ms, retry_condition, retry_jitter, trigger_rules, on_success, on_failure, invokes_computation_graph, post_invocation — 11 attributes). Adding a 12th attribute requires adding a parse arm + a code-gen arm.

#### Suggested Resolution

Split `tasks.rs` into `parse.rs` (attribute parsing), `codegen.rs` (token generation), and `validators.rs` (compile-time validation). The internal organization in `computation_graph/` already has this shape (`accumulator_macros.rs`, `codegen.rs`, `graph_ir.rs`, `parser.rs`). Consider extracting the retry attribute set into a separate `#[retry(...)]` macro that can be applied independently — that's an evolvability win since retry policies are also used outside tasks.

### EVO-20: Documentation of evolvability boundaries is weakest at the FFI/wire-format layer

**Severity**: Observation
**Location**: `crates/cloacina-workflow-plugin/src/lib.rs` (whole file)
**Confidence**: Medium

#### Description

The plugin ABI doc at `lib.rs:700-807` describes each method's purpose well, but does not document: (a) the wire format choice (debug=JSON, release=bincode, encoded only in `feedback_fidius_wire_format.md` memory), (b) the version-skew policy beyond `#[optional(since = N)]`, (c) the deprecation procedure for an existing method, (d) how to evolve a wire-format type (e.g., add a field to `TaskMetadataEntry`). For a stable plugin ABI, all four are needed. Today the project relies on "we'll do an atomic migration pre-1.0" — fine for now, but the moment a third-party cdylib ships, the docs become the contract.

#### Evidence

`crates/cloacina-workflow-plugin/src/lib.rs:700-807` — the trait doc comments are good but only describe semantics, not evolution policy. The `METHOD_*` constants are described as "pinned" (line 675) but there's no statement of when reordering would be allowed. `crates/cloacina-workflow-plugin/src/types.rs:24-43` — `TaskMetadataEntry` is a serde struct without `#[serde(default)]` or version markers; adding a field is a wire-format break unless every plugin recompiles.

#### Suggested Resolution

Author an ADR or a section in `cloacina-workflow-plugin`'s lib.rs covering: ABI version field, optional method bit policy, wire-format type evolution rules (add fields with `#[serde(default)]`, never remove), deprecation timeline. The fidius `version = 2` parameter on `#[plugin_interface]` is the right hook — document what version-3 would require and what migration looks like.

## Positive Patterns

1. **Crate split discipline holds.** `cargo tree -p cloacina-compiler` confirms zero pyo3, zero rdkafka in the compiler service. `cargo tree -p cloacina-workflow` (a leaf crate) confirms zero diesel, zero kafka. The "minimal authoring crates" really are minimal — the discipline that the team set out at I-0050 / T-0529 has held against feature pressure.

2. **`Runtime` registry shape is the right abstraction for hot-swapping.** Seven separate registries (tasks, workflows, triggers, CGs, trigger-less CGs, reactors, stream backends), all with `register_*` / `unregister_*` / `get_*` triples, all `Box<dyn Fn() -> X + Send + Sync>` constructors. Adding a new primitive type is a clean +1 registry. The reconciler operates on this shape uniformly. This is genuinely good extensibility.

3. **`#[serde(deny_unknown_fields)]` + migration-hint wrapping at manifest load.** `crates/cloacina/src/registry/reconciler/loading.rs:172-200` catches legacy `package_type` and `[[triggers]]` keys and emits a friendly migration message. This is *exactly* the kind of evolvability investment that pays off — the team is teaching the user how to migrate when a structural change ships.

4. **Service manager extraction (T-0483).** `DefaultRunner.service_manager: Arc<RwLock<ServiceManager>>` is the centralization point for every background service handle + shutdown signal. `shutdown()` is one call. Extracting this from the runner directly was a real evolvability win — it's reusable for Python's `PyDefaultRunner` shape and any future runner variant.

5. **fidius `#[optional(since = N)]` capability bits used correctly.** Methods 4-8 on `CloacinaPlugin` are all `#[optional(since = 2)]`, meaning v1 plugins return `CallError::NotImplemented` and the host treats that as "no entries of that kind." This is the right way to evolve the plugin ABI additively. The pattern is well-applied.

6. **Documentation hygiene at the trait + lib level.** `cloacina/src/lib.rs:1-450` is heavily documented. `python_runtime.rs` is heavily documented. The reconciler load_package fn has section markers (`--- Step 1, 2, 3 ---`). The team writes for the reader. (The exception is the FFI/wire-format layer — see EVO-20.)
