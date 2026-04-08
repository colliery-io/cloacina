# Legibility Review

## Summary

Cloacina is a well-structured Rust workspace with strong documentation at the module and function level, clear top-level architecture, and excellent entrypoint legibility (both `lib.rs` and `main.rs` are concise and easy to follow). However, the codebase imposes significant cognitive load on newcomers through terminology drift between "Pipeline" and "Workflow," three distinct backend-dispatch macros with overlapping purposes, duplicate error variants within the same enum, and a scheduler module namespace collision (`scheduler.rs` vs `task_scheduler/`) that forces the reader to hold two distinct concepts under nearly identical names.

## Key Themes

**Positive patterns:**
- Module-level rustdoc is thorough and consistently present, with ASCII architecture diagrams and example code
- The prelude module is well-curated and provides a genuine "single import" experience
- The crate split (`cloacina-workflow` for authoring vs `cloacina` for runtime) is a clean, well-documented separation
- CLI (`cloacinactl`) is immediately understandable -- clap subcommands with clear help text
- The computation graph subsystem has consistent naming (Accumulator, Reactor, ReactiveScheduler) with a clear mental model
- The universal types (`UniversalUuid`, `UniversalTimestamp`, `UniversalBool`) have excellent inline comments explaining the PostgreSQL/SQLite duality

**Negative patterns:**
- Domain terminology is inconsistent: "Pipeline" and "Workflow" are used interchangeably for the same concept
- Multiple dispatch macros (`dispatch_backend!`, `backend_dispatch!`, `connection_match!`) coexist with no clear guidance on when to use which
- Error enums contain legacy/duplicate variants alongside current ones
- The relationship between `scheduler.rs` (unified cron+trigger) and `task_scheduler/` (task readiness) requires insider knowledge to navigate
- The `lib.rs` re-export surface is very large (35 `pub use` statements) -- re-exports the same types that are available via `prelude`, adding noise

## Findings

## [LEG-01]: "Pipeline" vs "Workflow" terminology drift
**Severity**: Major
**Location**: `crates/cloacina/src/executor/pipeline_executor.rs`, `crates/cloacina/src/runner/default_runner/`, `crates/cloacina/src/task_scheduler/mod.rs`, `crates/cloacina/src/workflow/mod.rs`
**Confidence**: High

### Description
The codebase uses "Pipeline" and "Workflow" to refer to the same concept -- a named DAG of tasks. Users define `Workflow` objects using `WorkflowBuilder` or the `workflow!` macro, but then execute them via `PipelineExecutor`, which returns a `PipelineResult` containing a `workflow_name` field. The `DefaultRunner` doc says "workflow execution" but delegates to `PipelineExecutor::execute()`. The database table is `pipeline_executions`, but the CLI and API say "workflow."

This forces newcomers to mentally equate two terms that sound like separate concepts. "Workflow" suggests the definition, "Pipeline" suggests the runtime execution -- but this distinction is never documented, and they are mixed freely (e.g., `PipelineResult.workflow_name`, `PipelineError::WorkflowNotFound`).

### Evidence
- `pipeline_executor.rs` line 97: `PipelineError::WorkflowNotFound { workflow_name }`
- `PipelineResult` struct (line 164) has `workflow_name: String` but type is `PipelineResult`
- `task_scheduler/mod.rs` line 330: method named `schedule_workflow_execution` returns data stored in `pipeline_executions` table
- `DefaultRunner` doc (line 52): "coordinates workflow scheduling and task execution" but its trait is `PipelineExecutor`

### Suggested Resolution
Choose one term and use it consistently. Since the user-facing API (macro, builder, CLI, HTTP API) already centers on "Workflow," consider renaming `PipelineExecutor` to `WorkflowExecutor`, `PipelineResult` to `WorkflowResult`, `PipelineExecution` to `WorkflowExecution`, and `PipelineStatus` to `WorkflowStatus`. The database table can retain `pipeline_executions` for migration compatibility, but the Rust API should be consistent.

---

## [LEG-02]: Scheduler module naming collision
**Severity**: Major
**Location**: `crates/cloacina/src/scheduler.rs`, `crates/cloacina/src/task_scheduler/mod.rs`
**Confidence**: High

### Description
There are two top-level scheduler modules with confusingly similar names:
- `scheduler.rs` contains `Scheduler` -- the unified cron + trigger scheduler that fires workflow executions on a schedule
- `task_scheduler/` contains `TaskScheduler` -- which converts workflow definitions into database execution plans and manages task readiness within a single execution

These are fundamentally different responsibilities (schedule-level orchestration vs execution-level task readiness), but the names do not convey this distinction. A newcomer reading `use crate::{Scheduler, TaskScheduler}` cannot infer which is which without reading the docs of both.

### Evidence
- `scheduler.rs` line 17: "Unified scheduler for both cron and trigger-based workflow execution"
- `task_scheduler/mod.rs` line 17: "converts Workflow definitions into persistent database execution plans and manages task readiness"
- Both are imported and used side-by-side in `runner/default_runner/mod.rs` lines 48-49

### Suggested Resolution
Rename to better reflect roles. For example: `scheduler.rs` could become `cron_trigger_scheduler.rs` or `schedule_manager.rs`; the `task_scheduler/` could become `execution_planner/` or `readiness_manager/`. Alternatively, add a module-level `//! # How this relates to ...` comment in each that explicitly cross-references the other.

---

## [LEG-03]: Three backend-dispatch macros with overlapping purpose
**Severity**: Minor
**Location**: `crates/cloacina/src/database/connection/backend.rs:265`, `crates/cloacina/src/dal/unified/mod.rs:97-156`
**Confidence**: High

### Description
Three distinct macros handle backend dispatch:
1. `dispatch_backend!` (in `backend.rs`) -- used 132 times across the codebase
2. `backend_dispatch!` (in `dal/unified/mod.rs`) -- used 1 time (in the same file it is defined)
3. `connection_match!` (in `dal/unified/mod.rs`) -- used 1 time (in the same file it is defined)

All three solve the same problem: branching on backend type. A newcomer encountering any of these must discover the other two to understand the full pattern. The nearly-identical names `dispatch_backend` and `backend_dispatch` are especially confusing -- they differ only in word order.

### Evidence
- `dispatch_backend!` is the dominant macro (132 uses across 21 files)
- `backend_dispatch!` is defined at `dal/unified/mod.rs:97` and only referenced once
- `connection_match!` is defined at `dal/unified/mod.rs:136` and only referenced once

### Suggested Resolution
Since `dispatch_backend!` is the established convention with 132 call sites, remove or deprecate `backend_dispatch!` and `connection_match!`. If they serve a genuinely different purpose (e.g., `connection_match!` binds a connection variable), document that distinction clearly, or fold the functionality into `dispatch_backend!`.

---

## [LEG-04]: Duplicate and legacy error variants in ValidationError
**Severity**: Minor
**Location**: `crates/cloacina/src/error.rs:186-253`
**Confidence**: High

### Description
`ValidationError` contains multiple pairs of variants that represent the same concept:
- `CyclicDependency { cycle: Vec<String> }` (line 189) AND `CircularDependency { cycle: String }` (line 200) -- same concept, different field types
- `MissingDependency { task, dependency }` (line 192) AND `MissingDependencyOld { task_id, dependency }` (line 196) -- explicitly named "Old" indicating it is legacy

`WorkflowError` also has `CyclicDependency(Vec<String>)` (line 325), a third representation of the same concept.

This forces contributors to choose between near-identical variants and makes pattern matching more verbose than necessary.

### Evidence
- `error.rs` line 189: `CyclicDependency { cycle: Vec<String> }`
- `error.rs` line 200: `CircularDependency { cycle: String }`
- `error.rs` line 192: `MissingDependency { task, dependency }`
- `error.rs` line 196: `MissingDependencyOld { task_id, dependency }`
- `task.rs` line 505 uses `MissingDependencyOld` (3 total references)
- `error.rs` line 325: `WorkflowError::CyclicDependency(Vec<String>)`

### Suggested Resolution
Remove the `Old` variants and consolidate cyclic dependency into one variant name and field shape. Migrate the 3 call sites for `MissingDependencyOld` to `MissingDependency`. Choose either `CyclicDependency` or `CircularDependency` and remove the other.

---

## [LEG-05]: Lossy error conversion in ContextError-to-TaskError From impl
**Severity**: Minor
**Location**: `crates/cloacina/src/error.rs:354-379`
**Confidence**: High

### Description
The `From<ContextError> for TaskError` implementation converts `ContextError::Database`, `ContextError::ConnectionPool`, and `ContextError::InvalidScope` by wrapping them in `ContextError::KeyNotFound` with a stringified message. This is semantically misleading: a database connection failure is not a "key not found" error. It would confuse anyone debugging a production failure from logs, and it makes error-specific handling impossible downstream.

### Evidence
```rust
// error.rs lines 364-373
ContextError::Database(e) => {
    cloacina_workflow::ContextError::KeyNotFound(format!("Database error: {}", e))
}
ContextError::ConnectionPool(msg) => cloacina_workflow::ContextError::KeyNotFound(
    format!("Connection pool error: {}", msg),
),
```
The comment on line 363 acknowledges this: "Database and ConnectionPool errors don't have workflow equivalents, so convert them to a generic message."

### Suggested Resolution
Add a generic/other variant to `cloacina_workflow::ContextError` (e.g., `Other(String)`) that can carry these infrastructure errors without misrepresenting them as key-not-found. This is a cross-crate change but a small one.

---

## [LEG-06]: lib.rs re-exports duplicate what prelude already provides
**Severity**: Minor
**Location**: `crates/cloacina/src/lib.rs:525-573`
**Confidence**: High

### Description
`lib.rs` has 35 `pub use` statements that flatten types to the crate root (e.g., `pub use context::Context`, `pub use task::Task`). The prelude module (lines 453-483) already provides these same key types. This creates two import paths for every core type:
- `use cloacina::Context` (via root re-export)
- `use cloacina::prelude::Context` (via prelude)

The root re-exports also include internal machinery like `return_task_handle`, `take_task_handle`, `parse_namespace`, and `global_task_registry` -- items a typical user never needs.

### Evidence
- `lib.rs` line 529: `pub use context::Context;`
- `lib.rs` line 455: `pub use crate::context::Context;` (inside `mod prelude`)
- `lib.rs` line 546: `pub use executor::{return_task_handle, take_task_handle, with_task_handle, ...}` -- internal executor helpers exposed at root
- `lib.rs` line 557: `pub use task::{global_task_registry, register_task_constructor, ...}` -- registry internals

### Suggested Resolution
Consider trimming the root re-exports to only items that are genuinely needed at `cloacina::` level (e.g., `Database`, `init_logging`). Document the prelude as the canonical import path for user-facing types. Internal helpers should be importable via their module path (`cloacina::executor::take_task_handle`) rather than promoted to the crate root.

---

## [LEG-07]: Excellent module-level documentation throughout
**Severity**: Observation (Positive)
**Location**: All `mod.rs` and major source files
**Confidence**: High

### Description
Virtually every module in the codebase has detailed rustdoc headers with:
- A one-line summary of the module's purpose
- Key features bullet list
- ASCII architecture diagrams (e.g., `dispatcher/mod.rs` lines 24-28, `scheduler.rs` lines 36-44)
- Code examples with `rust,ignore` annotations
- Cross-references to specifications (e.g., `accumulator.rs` references CLOACI-S-0004)

This is well above typical Rust project standards and significantly aids orientation for newcomers.

### Evidence
- `task.rs`: 335 lines of doc comments including tutorials, how-to guides, state machine diagram
- `scheduler.rs`: ASCII architecture diagram and specification reference
- `computation_graph/reactor.rs`: Concise 3-concern description and spec reference (CLOACI-S-0005)
- `dal/unified/mod.rs`: Architecture explanation with example code

### Suggested Resolution
Continue this practice. Consider cross-linking the scheduler modules (LEG-02) with similar quality.

---

## [LEG-08]: The DAL accessor pattern is clear and consistent
**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/dal/unified/mod.rs:174-304`
**Confidence**: High

### Description
The `DAL` struct provides a clean, discoverable API via short-lived accessor methods that return typed sub-DALs:
```rust
dal.context().create(&ctx).await?;
dal.task_execution().claim(id).await?;
dal.pipeline_execution().get_last_version(name).await?;
```
Each sub-DAL (`ContextDAL`, `TaskExecutionDAL`, etc.) is a lightweight reference borrowing `&DAL`, keeping the API fluent without ownership complexity. The naming is consistent (`dal.X()` returns `XDAL`), making the pattern immediately predictable after seeing one example.

### Evidence
- `dal/unified/mod.rs` lines 206-260: 13 accessor methods following the same `fn x(&self) -> XDAL<'_>` pattern

### Suggested Resolution
No change needed. This pattern successfully balances discoverability with separation of concerns.

---

## [LEG-09]: DefaultRunner struct has many Arc<RwLock<Option<Arc<...>>>> fields
**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/mod.rs:68-88`
**Confidence**: Medium

### Description
The `DefaultRunner` struct has 5 fields of the form `Arc<RwLock<Option<Arc<T>>>>` (lines 79-88). This triple-wrapped pattern creates high cognitive load when reading code that accesses these fields:
```rust
let lock = self.reactive_scheduler.write().await;  // Arc<RwLock<...>>
*lock = Some(scheduler);                            // Option<Arc<...>>
```
The `Option` exists because these services are initialized lazily after construction, and the `Arc<RwLock<...>>` enables shared mutable access across cloned runners. Each layer serves a purpose, but the combined nesting is hard to parse at a glance.

### Evidence
- `runner/default_runner/mod.rs` lines 79-88: Five fields with `Arc<RwLock<Option<Arc<...>>>>`
- The `with_config` constructor (line 182) initializes them all to `Arc::new(RwLock::new(None))`
- The `Clone` impl (line 330) simply clones all the Arcs

### Suggested Resolution
Consider extracting these into a single `OptionalServices` struct that holds the 5 optional `Arc<T>` values, reducing `DefaultRunner` to `services: Arc<RwLock<OptionalServices>>`. This reduces 5 separate lock acquisitions to one shared struct, and makes the initialization pattern (`OptionalServices::default()`) more transparent.

---

## [LEG-10]: Computation graph subsystem is well-isolated and named
**Severity**: Observation (Positive)
**Location**: `crates/cloacina/src/computation_graph/`
**Confidence**: High

### Description
The computation graph subsystem uses a distinct vocabulary (Accumulator, Reactor, ReactiveScheduler, SourceName, InputCache, Boundary, ReactionCriteria) that is internally consistent and avoids colliding with the task/workflow terminology. The module structure directly mirrors the conceptual model: sources feed accumulators, accumulators produce boundaries, boundaries feed reactors, and the ReactiveScheduler supervises everything. Each file maps 1:1 to one concept.

### Evidence
- `accumulator.rs`: defines `Accumulator` trait, `EventSource`, `AccumulatorHealth`
- `reactor.rs`: defines `Reactor`, `ReactorHealth`, `ReactionCriteria`
- `scheduler.rs`: defines `ReactiveScheduler`, `ComputationGraphDeclaration`
- `types.rs`: defines `InputCache`, `GraphResult`, `SourceName`
- `global_registry.rs`: registration helpers following the same pattern as task/workflow registries

### Suggested Resolution
No change needed. This is a good model for how the scheduler module naming (LEG-02) could be improved.

---

## [LEG-11]: Task scheduler code duplication between PostgreSQL and SQLite branches
**Severity**: Minor
**Location**: `crates/cloacina/src/task_scheduler/mod.rs:423-542`
**Confidence**: High

### Description
The `create_pipeline_postgres` (lines 423-481) and `create_pipeline_sqlite` (lines 484-542) methods are structurally identical -- the same transaction logic, the same loop inserting tasks, the same field mappings. The only difference is the pool accessor method (`get_postgres_connection` vs `get_sqlite_connection`). This duplication exists throughout the DAL layer (132 uses of `dispatch_backend!`), but in this scheduler file the two 60-line methods are placed back-to-back, making the repetition visually striking.

This is a known consequence of the runtime backend selection architecture (ADR-1), but for a newcomer reading this file, seeing two identical methods raises the question "what's different?" and finding the answer requires a careful diff.

### Evidence
- `task_scheduler/mod.rs` lines 423-481 (`create_pipeline_postgres`)
- `task_scheduler/mod.rs` lines 484-542 (`create_pipeline_sqlite`)
- The bodies differ only in `get_postgres_connection()` vs `get_sqlite_connection()`

### Suggested Resolution
Add a brief comment at the top of each method (or before the pair) explaining the architectural reason: "These methods are intentionally duplicated per backend because Diesel's type system requires separate connection types. See ADR-1." This converts "wait, is this a bug?" into "understood, architectural constraint."

---

## [LEG-12]: cloacinactl main.rs is a model of CLI legibility
**Severity**: Observation (Positive)
**Location**: `crates/cloacinactl/src/main.rs`
**Confidence**: High

### Description
The CLI entrypoint is only 200 lines and achieves excellent legibility:
- All subcommands are visible at a glance via the `Commands` enum (lines 45-86)
- Each variant's doc comment doubles as the CLI help text
- The `main` function (lines 135-199) is a clean match over commands with no logic beyond delegation
- Default values and env var bindings are declared inline with the clap attributes

A newcomer can understand the entire CLI surface in under 2 minutes.

### Evidence
- `main.rs` lines 45-86: `Commands` enum with `Daemon`, `Serve`, `Config`, `Admin` -- each with doc comments
- `main.rs` lines 135-199: `main()` is purely dispatch, no business logic

### Suggested Resolution
No change needed.

---

## [LEG-13]: Workflow vs Graph conceptual overlap
**Severity**: Observation
**Location**: `crates/cloacina/src/graph.rs`, `crates/cloacina/src/workflow/graph.rs`
**Confidence**: Medium

### Description
There are two "graph" modules:
- `graph.rs` (top-level): defines `WorkflowGraph` using petgraph, with `TaskNode` and `DependencyEdge` -- a rich serializable graph representation with analysis algorithms
- `workflow/graph.rs`: defines `DependencyGraph` also using petgraph -- a simpler DAG tracking task dependencies

Both represent the same conceptual thing (a workflow's task dependency structure) but at different levels of richness. The top-level `graph.rs` is used for package metadata serialization; `workflow/graph.rs` is used for runtime dependency resolution. The distinction is valid but not immediately obvious from the names alone.

### Evidence
- `graph.rs` line 74: `pub struct WorkflowGraph { graph: DiGraph<TaskNode, DependencyEdge> }`
- `workflow/graph.rs` line 22: uses `petgraph::{Directed, Graph}` for a simpler dependency tracker
- Both are exported from `lib.rs` (`pub use graph::WorkflowGraph` and via `workflow::DependencyGraph`)

### Suggested Resolution
Consider renaming `graph.rs` to something like `workflow_graph_data.rs` or moving it under `packaging/` since its primary purpose is serializable metadata for packages. Alternatively, a module-level comment in each cross-referencing the other would help.

---

## [LEG-14]: "task_name" field stores full namespace strings
**Severity**: Observation
**Location**: `crates/cloacina/src/dal/unified/task_execution/mod.rs:58-59`, `crates/cloacina/src/executor/types.rs:198-208`
**Confidence**: Medium

### Description
Several data structures use a `task_name: String` field that actually stores a full 4-part namespace (`tenant::package::workflow::task_id`). The field name "task_name" suggests a human-readable name, but it is actually a structured identifier. Similarly, `ClaimedTask.task_name` holds values like `"t::p::w::my_task"` (as shown in tests at line 334). A newcomer reading `task.task_name` would expect something like `"extract_data"`, not `"public::embedded::etl::extract_data"`.

### Evidence
- `executor/types.rs` line 205: `pub task_name: String` with test value `"tenant::pkg::wf::my_task"`
- `dal/unified/task_execution/mod.rs` line 59: `pub task_name: String` in `ClaimResult`
- `TaskNamespace` has a `Display` impl that produces `tenant::package::workflow::task_id`

### Suggested Resolution
Rename these fields to `task_namespace` or `task_qualified_name` where they store the full namespace. Reserve `task_name` for the short, human-readable task ID (`task_id` component of the namespace).
