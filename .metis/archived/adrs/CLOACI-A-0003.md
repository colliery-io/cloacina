---
id: 001-complexity-analysis-simply-complex
level: adr
title: "Complexity Analysis: Simply Complex, Not Complicated"
number: 1
short_code: "CLOACI-A-0003"
created_at: 2026-03-23T02:26:35.487343+00:00
updated_at: 2026-03-23T02:26:35.487343+00:00
decision_date:
decision_maker:
parent:
archived: true

tags:
  - "#adr"
  - "#phase/draft"


exit_criteria_met: false
initiative_id: NULL
---

# ADR-1: Complexity Analysis: Simply Complex, Not Complicated

Comprehensive complexity analysis of the Cloacina Rust codebase (88K lines, 285 .rs files, 5 crates).
Framing: the system should be "simply complex, not complicated" -- necessary complexity from the
problem domain is fine; accidental complexity from poor abstractions should be eliminated.

---

## 1. What Is Already Well-Designed

Before cataloguing problems, it is important to acknowledge what works:

- **Clean crate layering.** `cloacina-workflow` (authoring, no DB deps) / `cloacina` (engine) /
  `cloacina-macros` (proc macros) / `cloacinactl` (server) / `cloacina-testing` (test utils).
  The dependency arrows point in the right direction.

- **Scheduler loop.** `scheduler_loop.rs` (357 lines) is clean, well-structured, has no deep
  nesting, and reads linearly. The outbox-with-fallback pattern for backward compatibility is
  thoughtful.

- **Cron scheduler.** `cron_scheduler.rs` (700 lines) is a model of the saga pattern: clear
  separation of concerns, good doc comments, and the audit-before-handoff guarantee is exactly
  the right design for reliability.

- **Thread task executor.** `thread_task_executor.rs` (871 lines) is long but each method is
  focused. The context merging logic and retry decision logic are clean. The `TaskExecutor` trait
  impl is a natural adapter.

- **Serve command.** `serve.rs` (1032 lines) is long primarily because of thorough inline tests
  (650+ lines of tests). The actual `run()` function is ~100 lines and reads well. The `ServeMode`
  enum (All/Api/Worker/Scheduler) is a sensible operational decomposition.

- **Error type organization.** The error hierarchy (`ContextError`, `TaskError`, `ValidationError`,
  `ExecutorError`, `WorkflowError`, `RegistryError`, `StorageError`, `LoaderError`) maps cleanly
  to the system's conceptual boundaries. Each error type lives in the right crate.

- **Builder pattern for config.** `DefaultRunnerConfig` uses a proper builder with sensible defaults.
  The `#[non_exhaustive]` annotation is a good API stability decision.

---

## 2. The DAL Duplication Problem (Accidental Complexity -- HIGH IMPACT)

### Finding

The unified DAL layer (`crates/cloacina/src/dal/unified/`) totals **13,555 lines** across 33 files.
Every database operation is implemented twice: once for Postgres, once for SQLite. The dispatch
pattern is:

```rust
pub async fn mark_completed(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
    crate::dispatch_backend!(
        self.dal.backend(),
        self.mark_completed_postgres(task_id).await,
        self.mark_completed_sqlite(task_id).await
    )
}
```

Each `_postgres` and `_sqlite` variant follows an identical structure:
1. Get connection from backend-specific pool
2. `conn.interact(|conn| { conn.transaction(|conn| { ... }) })`
3. Execute identical Diesel queries
4. Map errors identically

I examined `task_execution/state.rs` (952 lines) in detail. It has 6 state transition methods
(`mark_completed`, `mark_failed`, `mark_ready`, `mark_skipped`, `mark_abandoned`,
`set_sub_status`, `reset_retry_state`), each duplicated postgres/sqlite. The Diesel queries
inside the postgres and sqlite variants are **character-for-character identical** in most cases.
The only difference is the connection acquisition method.

Similarly, `pipeline_execution.rs` (1,412 lines) has `create_postgres`/`create_sqlite` where
the transaction body is identical line-for-line.

### Classification: Accidental Complexity

The Diesel query DSL already abstracts over backends. The duplication exists because of the
connection pool dispatch pattern, not because the queries differ.

### Proposed Simplification

Extract a generic `with_connection` helper that handles backend dispatch once:

```rust
impl DAL {
    async fn with_connection<F, R>(&self, f: F) -> Result<R, ValidationError>
    where
        F: FnOnce(&mut AnyConnection) -> Result<R, diesel::result::Error> + Send + 'static,
        R: Send + 'static,
    { ... }
}
```

Each DAL method becomes a single function body instead of two:

```rust
pub async fn mark_completed(&self, task_id: UniversalUuid) -> Result<(), ValidationError> {
    self.dal.with_connection(move |conn| {
        conn.transaction(|conn| {
            // single implementation
        })
    }).await
}
```

**Estimated impact:** ~5,000-6,000 lines removed. Every DAL file shrinks by roughly 40-50%.
The `dispatch_backend!` and `connection_match!` macros become unnecessary for most uses.

**Risk:** Low-medium. Diesel's `MultiConnection` support may require careful typing. A phased
approach (start with simple CRUD methods, then tackle transactional state changes) would be safe.

---

## 3. Configuration Sprawl (Accidental Complexity -- MEDIUM IMPACT)

### Finding

`DefaultRunnerConfig` has **27 fields**, each with a getter method and a builder setter method.
The config file alone is 849 lines. The fields break down as:

- **Core (5):** `max_concurrent_tasks`, `scheduler_poll_interval`, `task_timeout`,
  `pipeline_timeout`, `db_pool_size`
- **Cron (7):** `enable_cron_scheduling`, `cron_poll_interval`, `cron_max_catchup_executions`,
  `cron_enable_recovery`, `cron_recovery_interval`, `cron_lost_threshold_minutes`,
  `cron_max_recovery_age`, `cron_max_recovery_attempts`
- **Trigger (3):** `enable_trigger_scheduling`, `trigger_base_poll_interval`, `trigger_poll_timeout`
- **Registry (5):** `enable_registry_reconciler`, `registry_reconcile_interval`,
  `registry_enable_startup_reconciliation`, `registry_storage_path`, `registry_storage_backend`
- **Continuous (2):** `enable_continuous_scheduling`, `continuous_poll_interval`
- **Identity (2):** `runner_id`, `runner_name`
- **Routing (1):** `routing_config`
- **Recovery (1):** `enable_recovery`

### Classification: Accidental Complexity (partially)

The cron, trigger, registry, and continuous subsystems each have their own config structs already
(`CronSchedulerConfig`, `TriggerSchedulerConfig`, `ReconcilerConfig`, `ContinuousSchedulerConfig`).
The runner config duplicates these fields and manually maps them in `services.rs`.

### Proposed Simplification

Nest sub-configs by subsystem:

```rust
pub struct DefaultRunnerConfig {
    pub core: CoreConfig,              // 5 fields
    pub cron: Option<CronConfig>,      // None = disabled, Some = enabled with config
    pub trigger: Option<TriggerConfig>,
    pub registry: Option<RegistryConfig>,
    pub continuous: Option<ContinuousConfig>,
    pub identity: IdentityConfig,
    pub routing: Option<RoutingConfig>,
}
```

Using `Option<SubConfig>` eliminates the `enable_*` booleans -- `None` means disabled,
`Some(config)` means enabled. This also eliminates the manual mapping in `services.rs`
since the sub-configs can be passed directly.

**Estimated impact:** ~200 lines removed from config.rs, ~50 lines from services.rs.
Concept count reduced: 27 flat fields become 7 grouped fields. Builder API becomes
more discoverable.

---

## 4. State Machine Representation (Accidental Complexity -- MEDIUM IMPACT)

### Finding

Task and pipeline statuses are **stringly typed** throughout the system. Status values
appear as string literals scattered across DAL code:

- **Task states used:** `NotStarted`, `Pending`, `Ready`, `Running`, `Completed`, `Failed`,
  `Skipped` (7 states, plus `Abandoned` which maps to `Failed` with an `ABANDONED:` prefix
  in `error_details`)
- **Task sub-states:** `Active`, `Deferred` (nullable)
- **Pipeline states used:** `Pending`, `Running`, `Completed`, `Failed`, `Cancelled`, `Paused`
  (6 states)

Status comparisons are done with string equality:
```rust
if current.status == "Pending" || current.status == "Running" { ... }
if task.status == "Completed" || task.status == "Skipped" { ... }
```

### Classification: Accidental Complexity

The states themselves are essential -- a workflow orchestrator needs these lifecycle states.
But representing them as raw strings creates:
- No compile-time validation of valid transitions
- Risk of typos (no compiler catches misspelled status strings)
- No documentation of valid transitions
- The `Abandoned` hack (prefixing error_details) instead of a proper terminal state

### Proposed Simplification

Define enums with Diesel-compatible serialization:

```rust
#[derive(Debug, Clone, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = Text)]
pub enum TaskStatus {
    NotStarted, Pending, Ready, Running, Completed, Failed, Skipped, Abandoned
}

#[derive(Debug, Clone, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = Text)]
pub enum PipelineStatus {
    Pending, Running, Completed, Failed, Cancelled, Paused
}
```

The Diesel `AsExpression`/`FromSqlRow` impls store as strings in the database (backward compatible)
but enforce type safety in Rust code. State transition methods can enforce valid transitions:

```rust
impl TaskStatus {
    pub fn can_transition_to(&self, next: &TaskStatus) -> bool { ... }
}
```

**Estimated impact:** ~100 string literals replaced with enum variants. Subtle bugs prevented.
No database migration needed (still stored as strings). The `Abandoned` state gets promoted
from a string prefix hack to a proper enum variant.

---

## 5. Concept Count Assessment (Mixed)

### Essential Concepts (keep)

| Concept | Why Essential |
|---------|-------------|
| Pipeline/Workflow | The unit of work composition |
| Task | The unit of execution |
| Context | Data flowing between tasks |
| Schedule (Cron) | Time-based triggering |
| Trigger | Event-based triggering |
| Execution | A running instance of a pipeline |
| Registry | Package management for workflows |
| Namespace | Task identity (package::task) |

### Essential but Could Be Simplified

| Concept | Current State | Suggestion |
|---------|--------------|------------|
| Outbox (pipeline + task) | Two outbox tables for work distribution | Essential for distributed claiming. Keep but document clearly. |
| ExecutionEvent | Audit trail of state changes | Essential for observability. Keep. |
| Accumulator/Watermark/Boundary | Continuous scheduling concepts | Essential for streaming, but only when continuous mode is enabled. Gate documentation and discovery behind the feature. |
| Ledger | Continuous execution tracking | Same as above. |

### Potentially Accidental

| Concept | Assessment |
|---------|-----------|
| `TaskExecutionMetadata` (separate from `TaskExecution`) | Currently stores the `context_id` link. Could be a nullable column on `task_executions` instead of a separate table. Would eliminate one DAL module (~666 lines) and one join. |
| `RecoveryEvent` (separate from `ExecutionEvent`) | Recovery events could be a subtype of execution events with a specific `event_type` rather than a separate table. Would eliminate one DAL module (~390 lines). |
| `DetectorState` | Part of continuous scheduling. Essential within that subsystem. |

**Developer concept load:**
- To use the basic pipeline engine: **5 concepts** (Task, Workflow, Context, Runner, Execution)
- To use cron scheduling: **+2** (CronSchedule, CronExecution)
- To use triggers: **+2** (TriggerSchedule, TriggerExecution)
- To use continuous mode: **+5** (DataSource, Accumulator, Watermark, Boundary, Ledger)
- To work on the engine internals: **+8** (DAL, Outbox, Dispatcher, StateManager, Registry, Reconciler, ExecutionEvent, RecoveryEvent)

The basic user path is good (5 concepts). The internals add up but most are well-motivated.

---

## 6. Module Coupling (Mostly Good)

The `DefaultRunner` struct is the main coupling point, holding references to 12 subsystems
via `Arc<RwLock<Option<...>>>` fields. This is somewhat inherent to a "God object" that
orchestrates everything, but the `Option` wrapping for each subsystem is noisy.

`RuntimeHandles` has 9 fields, 7 of which are `Option<JoinHandle<()>>`. The pattern of
`start_*_services` methods in `services.rs` is repetitive (each follows: create watch channel,
create config, create service, spawn task with select/shutdown, store handle). This is a
~500 line file that is ~80% boilerplate.

### Proposed Simplification

A `ManagedService` abstraction:

```rust
struct ManagedService {
    handle: JoinHandle<()>,
    shutdown_tx: watch::Sender<bool>,
}

impl ManagedService {
    async fn spawn<F>(name: &str, shutdown_rx: broadcast::Receiver<()>, f: F) -> Self
    where F: Future<Output = ()> + Send + 'static { ... }

    async fn shutdown(self) { ... }
}
```

This would let `RuntimeHandles` become `Vec<ManagedService>` and each `start_*` method
could be ~10 lines instead of ~50.

**Estimated impact:** ~300 lines removed from services.rs. `RuntimeHandles` simplified
from 9 named fields to a `Vec`.

---

## 7. Error Type Assessment (Mostly Good, One Issue)

### Finding

The error hierarchy is generally well-organized across crate boundaries. One issue:

`ValidationError` is doing double duty. It covers both:
1. **Graph validation** errors (cyclic deps, missing deps, empty workflow) -- compile/build time
2. **Runtime** errors (execution failed, database connection, connection pool, recovery) -- execution time

This means a function returning `Result<_, ValidationError>` gives no signal about whether
it validates structure or executes operations.

Additionally, there are overlapping variants:
- `CyclicDependency { cycle: Vec<String> }` AND `CircularDependency { cycle: String }` -- same concept, two variants
- `MissingDependency` AND `MissingDependencyOld` -- legacy variant still present
- `Database(diesel::result::Error)` AND `DatabaseQuery { message: String }` AND `DatabaseConnection { message: String }` -- three ways to say "database error"

### Classification: Accidental Complexity

### Proposed Simplification

Split `ValidationError` into `GraphValidationError` (structural) and `RuntimeError` (operational).
Remove the `Old` and duplicate variants.

**Estimated impact:** ~20 lines removed, clarity improved. Breaking change for downstream
`match` arms, so do this with a deprecation cycle.

---

## 8. Summary of Simplification Strategies

| # | Strategy | Category | Lines Saved | Concepts Reduced | Risk | Priority |
|---|----------|----------|-------------|-----------------|------|----------|
| 1 | DAL generic connection dispatch | Accidental | ~5,000-6,000 | 2 macros eliminated | Medium | **HIGH** |
| 2 | Nested sub-configs | Accidental | ~250 | 27 flat -> 7 grouped | Low | Medium |
| 3 | Status enums | Accidental | ~0 (rewrite) | String bugs prevented | Low | Medium |
| 4 | ManagedService abstraction | Accidental | ~300 | RuntimeHandles simplified | Low | Medium |
| 5 | Merge TaskExecutionMetadata into TaskExecution | Accidental | ~666 | 1 table/DAL eliminated | Medium | Low |
| 6 | Split ValidationError | Accidental | ~20 | Clearer error semantics | Low (breaking) | Low |
| 7 | Merge RecoveryEvent into ExecutionEvent | Potentially Accidental | ~390 | 1 table/DAL eliminated | Medium | Low |

**Total estimated reduction:** ~6,500-7,600 lines (7-9% of codebase) with improved type safety
and reduced cognitive load.

### What NOT to Simplify

- The outbox pattern (pipeline_outbox + task_outbox): essential for distributed claiming and
  push-based dispatch. Removing it would lose the ability to scale executors.
- The cron scheduler saga pattern: the audit-before-handoff design is exactly right for reliability.
- The separate `cloacina-workflow` crate: keeps the authoring API free of database dependencies,
  which is critical for the Python/FFI story.
- The task state machine itself: 7 task states and 6 pipeline states are reasonable for a
  production orchestrator. Airflow has more.
- The continuous scheduling concepts (accumulator, watermark, boundary, ledger): these are
  essential complexity for streaming/reactive workloads. They should remain, but be gated
  behind clear feature documentation so batch-only users never encounter them.

---

## Decision

Adopt strategies #1 (DAL dedup) and #3 (status enums) as the highest-value, most impactful
improvements. Strategy #1 alone would remove ~6,000 lines of pure duplication. Strategy #3
prevents an entire class of runtime bugs.

Strategies #2 (nested config) and #4 (ManagedService) should be tackled next as they improve
developer experience significantly with low risk.

Strategies #5, #6, and #7 are lower priority and should be evaluated when those modules are
next touched for feature work.
