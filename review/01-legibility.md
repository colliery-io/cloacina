# Legibility Review

## Summary

Cloacina's codebase is well-organized at the macro level, with clear module boundaries and extensive doc comments. A newcomer can follow the primary workflow path (task -> workflow -> scheduler -> dispatcher -> executor) thanks to consistent documentation patterns and a logical crate structure. However, a persistent "pipeline" vs "workflow" naming collision throughout the data layer, plus high code duplication in the dispatch_backend pattern, are the two dominant sources of cognitive friction.

## Key Themes

**Positive patterns:**
- Thorough module-level doc comments with usage examples and architecture diagrams
- Clean trait-based abstractions (Task, Dispatcher, TaskExecutor, WorkflowExecutor, WorkflowRegistry)
- Consistent builder pattern usage (DefaultRunnerBuilder, RetryPolicyBuilder, etc.)
- Well-decomposed execution_planner module (scheduler_loop, state_manager, context_manager, trigger_rules each under 500 lines)
- DAL accessor pattern (dal.context(), dal.task_execution()) is highly readable
- Comprehensive test coverage with descriptive test names

**Negative patterns:**
- "Pipeline" and "workflow" used interchangeably for the same concept across layers
- Near-identical Postgres and SQLite code blocks repeated via dispatch_backend macro throughout the DAL
- Multiple naming indirections for the same concept (e.g., cloaca vs cloacina, TaskScheduler in execution_planner, Scheduler in cron_trigger_scheduler)
- lib.rs has a very large flat re-export surface (70+ symbols) that obscures the prelude's role
- Debug-level tracing calls duplicated (same message logged twice in build_task_context)

## Findings

## LEG-001: "Pipeline" vs "Workflow" naming collision
**Severity**: Major
**Location**: Pervasive; key files include `crates/cloacina/src/models/pipeline_execution.rs`, `crates/cloacina/src/dal/unified/pipeline_execution.rs`, `crates/cloacina/src/executor/pipeline_executor.rs`, `crates/cloacina/src/database/schema.rs` (table `pipeline_executions`), `crates/cloacina/src/execution_planner/scheduler_loop.rs`
**Confidence**: High

### Description
The public API consistently uses "workflow" terminology (WorkflowExecutor, WorkflowExecutionResult, WorkflowStatus, DefaultRunner.execute("workflow_name")), but the database layer, DAL, and internal scheduler use "pipeline" throughout. The database table is called `pipeline_executions`, the DAL accessor is `workflow_execution()` but operates on `pipeline_name` columns, the model struct is `WorkflowExecutionRecord` but has fields named `pipeline_name` and `pipeline_version`, and error messages mix both terms ("Pipeline execution failed" in WorkflowExecutionError::ExecutionFailed, "Pipeline timeout after 300s" in WorkflowExecutionError::Timeout).

A newcomer will encounter `workflow_execution().mark_completed(execution.id)` in the scheduler and then see the underlying table is `pipeline_executions` with a `pipeline_name` column, creating confusion about whether "pipeline" and "workflow" are the same thing or different concepts.

### Evidence
- `models/pipeline_execution.rs` line 29: `pub struct WorkflowExecutionRecord` with field `pub pipeline_name: String`
- `dal/unified/pipeline_execution.rs` line 35: `pub struct WorkflowExecutionDAL` operating on `pipeline_executions::table`
- `executor/pipeline_executor.rs` line 104: `WorkflowExecutionError::ExecutionFailed` displays "Pipeline execution failed"
- `executor/pipeline_executor.rs` line 107: `WorkflowExecutionError::Timeout` displays "Pipeline timeout after 300s"
- `executor/pipeline_executor.rs` lines 534-588: Test functions named `test_pipeline_status_*` but testing `WorkflowStatus` enum
- Database schema table name: `pipeline_executions` (not `workflow_executions`)
- 539 occurrences of `pipeline_exec` vs 35 of `workflow_exec` across the src directory

### Suggested Resolution
Align on one term. Since the public API uses "workflow" and the project documentation consistently says "workflow", rename the database table (via migration) and internal references from `pipeline_*` to `workflow_*`. Alternatively, add a prominent comment in the DAL explaining that "pipeline" is the legacy database term for "workflow".

---

## LEG-002: Postgres/SQLite code duplication via dispatch_backend macro
**Severity**: Major
**Location**: `crates/cloacina/src/dal/unified/pipeline_execution.rs` (1152 lines), `crates/cloacina/src/dal/unified/task_execution/claiming.rs` (829 lines), `crates/cloacina/src/dal/unified/task_execution/crud.rs` (284 lines), `crates/cloacina/src/execution_planner/mod.rs` lines 440-557, and 21 other files using `dispatch_backend!`
**Confidence**: High

### Description
Nearly every DAL method is implemented twice: once as `*_postgres()` and once as `*_sqlite()`, with the only difference being the connection acquisition call (`get_postgres_connection()` vs `get_sqlite_connection()`). The business logic inside the `conn.interact(move |conn| { ... })` closure is virtually identical between the two implementations. This pattern is repeated in `pipeline_execution.rs` (1152 lines where roughly half is duplication), `claiming.rs` (829 lines), and the `create_pipeline_postgres`/`create_pipeline_sqlite` pair in `execution_planner/mod.rs` (lines 440-557).

This doubles the code a reader must understand to verify any DAL behavior, and creates maintenance risk where a bugfix applied to one backend may be missed in the other.

### Evidence
- `dal/unified/pipeline_execution.rs` lines 60-120 (`create_postgres`) vs lines 122-160+ (`create_sqlite`): identical transaction structure, only `get_postgres_connection()` vs `get_sqlite_connection()` differs.
- `execution_planner/mod.rs` lines 441-498 (`create_pipeline_postgres`) vs lines 501-557 (`create_pipeline_sqlite`): identical task insertion loop body.
- `dispatch_backend!` macro at `database/connection/backend.rs` lines 265-290 merely selects which branch to call, but doesn't reduce the duplication of the inner logic.

### Suggested Resolution
Extract the common logic into a generic function parameterized by connection type, or create a helper macro that generates both backend methods from a single implementation body. Diesel's `MultiConnection` or a custom trait on the connection pool could enable a single code path with backend-agnostic connection retrieval.

---

## LEG-003: Multiple "Scheduler" concepts without disambiguation
**Severity**: Major
**Location**: `crates/cloacina/src/execution_planner/mod.rs` (TaskScheduler), `crates/cloacina/src/cron_trigger_scheduler.rs` (Scheduler), `crates/cloacina/src/computation_graph/scheduler.rs` (ReactiveScheduler), `crates/cloacina/src/runner/default_runner/mod.rs` line 87 (unified_scheduler field)
**Confidence**: High

### Description
There are three distinct scheduler types, each with overlapping names:
1. `TaskScheduler` (in `execution_planner`) -- evaluates task readiness within a workflow execution
2. `Scheduler` (in `cron_trigger_scheduler`) -- manages cron and trigger-based workflow firing
3. `ReactiveScheduler` (in `computation_graph::scheduler`) -- manages computation graph lifecycle

The module `execution_planner` contains `TaskScheduler`, but the module name says "planner" not "scheduler". The `Scheduler` in `cron_trigger_scheduler` is exported directly into the prelude-adjacent namespace via `lib.rs` line 543. Meanwhile, `DefaultRunner` has a field called `unified_scheduler` of type `Scheduler`, and a separate `scheduler` field of type `TaskScheduler`.

The doc comments on `lib.rs` lines 493-504 attempt to disambiguate with inline comments ("For task readiness and workflow execution planning, see execution_planner" vs "For cron and trigger scheduling, see cron_trigger_scheduler") but this is easy to miss.

### Evidence
- `lib.rs` line 543: `pub use cron_trigger_scheduler::{Scheduler, SchedulerConfig};` -- bare name `Scheduler` gives no hint it handles cron/triggers
- `runner/default_runner/mod.rs` line 77: `pub(super) scheduler: Arc<TaskScheduler>` and line 87: `pub(super) unified_scheduler: Arc<RwLock<Option<Arc<Scheduler>>>>`
- `execution_planner/mod.rs` line 187: `pub struct TaskScheduler` but the module is named `execution_planner`

### Suggested Resolution
Rename `Scheduler` in `cron_trigger_scheduler.rs` to `CronTriggerScheduler` or `ScheduleRunner` to distinguish it from `TaskScheduler`. Alternatively, consider renaming the `execution_planner` module to `task_scheduler` to match its primary export, or adding a type alias that makes the purpose clear.

---

## LEG-004: lib.rs re-export surface is excessively large
**Severity**: Minor
**Location**: `crates/cloacina/src/lib.rs` lines 530-578
**Confidence**: High

### Description
The root `lib.rs` exports over 70 individual symbols via `pub use` statements, in addition to the prelude module. This creates two competing import paths: `use cloacina::prelude::*` for the common types, and `use cloacina::SomeSpecificType` for everything else. A newcomer has to decide which import path to use, and the flat re-exports obscure which types are "important" vs "internal plumbing".

The prelude exports 20 types, while `lib.rs` directly re-exports another 50+, including internal types like `SlotToken`, `return_task_handle`, `take_task_handle`, `global_task_registry`, `register_task_constructor`, `dispatch_backend!`, and `DependencyEdge`.

### Evidence
- `lib.rs` lines 530-578: 48 lines of `pub use` re-exports
- `lib.rs` lines 453-486: prelude with 20 types
- Types like `global_task_registry()`, `register_task_constructor()`, `return_task_handle()`, `take_task_handle()` are implementation details exposed at crate root

### Suggested Resolution
Audit the root-level re-exports. Move implementation-detail types (global registry accessors, task handle manipulation, slot tokens) behind their respective modules, accessible only as `cloacina::task::global_task_registry()` or `cloacina::executor::take_task_handle()`. Reserve the crate root for the prelude and a small number of primary types.

---

## LEG-005: "cloaca" vs "cloacina" naming for Python bindings
**Severity**: Minor
**Location**: `crates/cloacina/src/lib.rs` line 592, Python module entry point
**Confidence**: Medium

### Description
The Rust project is named "cloacina" everywhere, but the Python wheel is named "cloaca" (the PyO3 module function is `fn cloaca()`). This means Python users write `from cloaca import task, workflow` while Rust users write `use cloacina::prelude::*`. There is no in-code comment explaining why the Python name differs. The system overview (review/00-system-overview.md) mentions it but a developer reading the code directly would be puzzled.

### Evidence
- `lib.rs` line 592: `fn cloaca(m: &Bound<'_, PyModule>) -> PyResult<()>`
- `python/mod.rs` line 25: mentions "The `cloaca` Python wheel re-exports these types"
- Cargo.toml name: `cloacina`, wheel name: `cloaca`

### Suggested Resolution
Add a doc comment on the `fn cloaca()` function explaining the naming choice (presumably "cloaca" is shorter/more idiomatic for Python users, or has a naming convention reason). A one-line comment like `// Python wheel is named "cloaca" (short form) to match pip naming conventions` would suffice.

---

## LEG-006: Duplicate debug log in build_task_context
**Severity**: Minor
**Location**: `crates/cloacina/src/executor/thread_task_executor.rs` lines 188-199
**Confidence**: High

### Description
The `build_task_context` method logs the same information twice at debug level with nearly identical messages, differing only by the "DEBUG:" prefix on the second one. This appears to be a leftover from development/debugging that was not cleaned up.

### Evidence
```rust
tracing::debug!(
    "Building context for task '{}' with {} dependencies: {:?}",
    claimed_task.task_name,
    dependencies.len(),
    dependencies
);
tracing::debug!(
    "DEBUG: Building context for task '{}' with {} dependencies: {:?}",
    claimed_task.task_name,
    dependencies.len(),
    dependencies
);
```

### Suggested Resolution
Remove the second `tracing::debug!` call with the "DEBUG:" prefix.

---

## LEG-007: Execution_planner module name does not match its primary export
**Severity**: Minor
**Location**: `crates/cloacina/src/execution_planner/mod.rs`
**Confidence**: Medium

### Description
The module is named `execution_planner` but its primary public export is `TaskScheduler`. The internal submodules are named `scheduler_loop`, `state_manager`, `recovery`, etc. -- all scheduler-related concepts. A newcomer looking for "the scheduler" would likely search for a `scheduler` module, not `execution_planner`. The module doc says "# Task Scheduler" as its heading, further emphasizing the mismatch.

### Evidence
- Module path: `cloacina::execution_planner::TaskScheduler`
- Module doc heading: `//! # Task Scheduler`
- Internal submodules: `scheduler_loop.rs`, `state_manager.rs` -- all scheduler-oriented

### Suggested Resolution
Either rename the module to `task_scheduler` (matching its doc heading and primary export), or rename the struct to `ExecutionPlanner` (matching the module name). The former is more consistent with how the codebase actually refers to this component.

---

## LEG-008: Well-structured DAL accessor pattern (positive)
**Severity**: Observation
**Location**: `crates/cloacina/src/dal/unified/mod.rs` lines 130-183
**Confidence**: High

### Description
The DAL's accessor pattern is exceptionally legible. Each entity type has a dedicated accessor method (`dal.context()`, `dal.task_execution()`, `dal.workflow_execution()`) that returns a scoped sub-DAL with only the methods relevant to that entity. This creates a natural, discoverable API: `self.dal.task_execution().mark_completed(id)` reads almost like English.

### Evidence
- `dal/unified/mod.rs` lines 130-183: 12 accessor methods, each returning a dedicated type
- Usage in `scheduler_loop.rs` line 276: `self.dal.task_execution().get_ready_for_retry().await?`
- Usage in `state_manager.rs` line 68: `self.dal.task_execution().mark_ready(task_execution.id).await?`

### Suggested Resolution
No change needed. This is an excellent pattern that should be maintained.

---

## LEG-009: Well-decomposed execution_planner module (positive)
**Severity**: Observation
**Location**: `crates/cloacina/src/execution_planner/` directory
**Confidence**: High

### Description
The execution_planner module is cleanly decomposed into focused submodules: `scheduler_loop.rs` (417 lines), `state_manager.rs` (manages dependency checking), `context_manager.rs` (handles context merging), `trigger_rules.rs` (trigger evaluation), `recovery.rs` (orphaned task recovery), and `stale_claim_sweeper.rs`. Each file has a single responsibility and reasonable size. The `mod.rs` serves as the public facade, exporting only `TaskScheduler` and the trigger rule types.

### Evidence
- `scheduler_loop.rs`: 417 lines -- run loop with circuit breaker
- `state_manager.rs`: dependency checking and readiness evaluation
- `context_manager.rs`: context merging logic
- `mod.rs`: 647 lines, mostly the `TaskScheduler` struct and `schedule_workflow_execution`

### Suggested Resolution
No change needed. This decomposition is exemplary.

---

## LEG-010: Runtime struct provides clean registry isolation
**Severity**: Observation
**Location**: `crates/cloacina/src/runtime.rs`
**Confidence**: High

### Description
The `Runtime` struct (248 lines) is a well-designed abstraction for registry isolation. It provides two clear modes: `Runtime::new()` for isolated testing and `Runtime::from_global()` for production with fallback to global registries. The doc comments clearly explain the delegation model, and the test suite (lines 269-447) thoroughly demonstrates both modes, including the precedence behavior.

### Evidence
- Clear doc comment at line 56-64 explaining the two modes
- `from_global()` at line 101: "delegates to globals" is self-documenting
- Test at line 397: `test_local_registration_takes_precedence_over_global` -- test name IS the spec

### Suggested Resolution
No change needed.

---

## LEG-011: var.rs is a model of concise, self-documenting design
**Severity**: Observation
**Location**: `crates/cloacina/src/var.rs`
**Confidence**: High

### Description
The `var.rs` module (233 lines including tests) is a standout example of legibility. It provides three functions (`var()`, `var_or()`, `resolve_template()`) with clear naming, thorough doc comments that include the environment variable convention, and comprehensive tests that double as usage examples. The `CLOACINA_VAR_` prefix convention is documented with concrete examples.

### Evidence
- Function names match their behavior perfectly: `var()` gets a variable, `var_or()` gets with default
- Doc comment at lines 19-51 explains the convention with real-world examples (KAFKA_BROKER, ANALYTICS_DB)
- `resolve_template()` handles edge cases (unclosed `{{`, multiple missing vars) with clear logic
- Error type `VarNotFound` includes the expected env var name in its message

### Suggested Resolution
No change needed. This module should serve as a template for other small utility modules.

---

## LEG-012: DefaultRunner struct has high field count and Arc<RwLock<Option<Arc<T>>>> nesting
**Severity**: Minor
**Location**: `crates/cloacina/src/runner/default_runner/mod.rs` lines 69-91
**Confidence**: High

### Description
`DefaultRunner` has 10 fields, 5 of which follow the pattern `Arc<RwLock<Option<Arc<T>>>>` (triple wrapping). This triple-nesting is cognitively expensive: readers must understand that the field is shared (Arc), mutably settable at runtime (RwLock), optionally present (Option), and the inner value is also shared (inner Arc). The field names (cron_recovery, workflow_registry, registry_reconciler, unified_scheduler, reactive_scheduler) suggest these are optional background services, but the wrapping pattern obscures this intent.

### Evidence
- Line 81: `pub(super) cron_recovery: Arc<RwLock<Option<Arc<crate::CronRecoveryService>>>>`
- Line 83: `pub(super) workflow_registry: Arc<RwLock<Option<Arc<dyn WorkflowRegistry>>>>`
- Line 85: `pub(super) registry_reconciler: Arc<RwLock<Option<Arc<RegistryReconciler>>>>`
- Line 87: `pub(super) unified_scheduler: Arc<RwLock<Option<Arc<Scheduler>>>>`
- Line 89-90: `pub(super) reactive_scheduler: Arc<RwLock<Option<Arc<...ReactiveScheduler>>>>`
- Constructor at lines 253-258: five `Arc::new(RwLock::new(None))` initializations

### Suggested Resolution
Consider extracting the optional services into a `ServiceRegistry` or `OptionalServices` struct that manages the init/shutdown lifecycle, reducing `DefaultRunner` to its core fields (runtime, database, config, scheduler) plus a single `services: Arc<RwLock<Services>>` field.

---

## LEG-013: cloacinactl CLI is cleanly structured (positive)
**Severity**: Observation
**Location**: `crates/cloacinactl/src/main.rs`
**Confidence**: High

### Description
The CLI entrypoint at 200 lines is a model of clarity. The command hierarchy is expressed directly via Clap derive macros, each variant has a descriptive doc comment that serves as the help text, and the `main()` function is a clean match tree with no business logic -- it delegates immediately to `commands::*` modules. The server module is similarly well-organized by route group (auth, executions, health_reactive, keys, tenants, triggers, workflows, ws).

### Evidence
- Lines 31-86: Command enum with doc comments that double as CLI help
- Lines 134-199: main() is purely routing, no inline logic
- Server module files: auth.rs, executions.rs, health_reactive.rs, keys.rs, tenants.rs, triggers.rs, workflows.rs, ws.rs -- each handles one route group

### Suggested Resolution
No change needed.

---

## LEG-014: Error types are well-organized but WorkflowExecutionError messages still say "Pipeline"
**Severity**: Minor
**Location**: `crates/cloacina/src/executor/pipeline_executor.rs` lines 97-120
**Confidence**: High

### Description
The error type hierarchy is well-structured -- ContextError, TaskError, ValidationError, ExecutorError, and WorkflowExecutionError each cover a distinct failure domain. However, `WorkflowExecutionError` variants display "Pipeline" in their user-facing messages, which contradicts the type name. This is a specific instance of LEG-001 manifesting in error output that users/operators will see in logs.

### Evidence
- Line 104: `ExecutionFailed { message }` displays `"Pipeline execution failed: {message}"`
- Line 107: `Timeout { timeout_seconds }` displays `"Pipeline timeout after {timeout_seconds}s"`
- Test at line 595: `test_pipeline_error_display_execution_failed` -- even the test name uses "pipeline"

### Suggested Resolution
Update the error message strings to use "Workflow" instead of "Pipeline", and rename the test functions to match.

---

## LEG-015: Comprehensive module-level documentation (positive)
**Severity**: Observation
**Location**: All major modules (task.rs, context.rs, execution_planner/mod.rs, dispatcher/mod.rs, executor/mod.rs, cron_trigger_scheduler.rs, var.rs)
**Confidence**: High

### Description
Nearly every module has a multi-section doc comment that includes: a heading, an overview of what the module does, key features/components, usage examples, and often ASCII/mermaid architecture diagrams. The `task.rs` module doc is 335 lines of documentation before any code, covering tutorials, how-to guides, lifecycle diagrams, and testing patterns. This follows the Diataxis documentation framework and dramatically reduces the time needed for a newcomer to understand each module.

### Evidence
- `task.rs` lines 17-335: complete tutorial from basic task creation through testing
- `execution_planner/mod.rs` lines 17-114: architecture overview with task state list, error handling, performance, and thread safety sections
- `dispatcher/mod.rs` lines 17-57: ASCII architecture diagram
- `cron_trigger_scheduler.rs` lines 17-44: ASCII architecture diagram showing saga pattern
- `lib.rs` lines 17-429: full crate-level documentation with mermaid diagrams

### Suggested Resolution
No change needed. This is an exceptional standard of documentation.
