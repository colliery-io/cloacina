---
id: code-organization-refactoring
level: initiative
title: "Code Organization Refactoring - Split Monolithic Files"
short_code: "CLOACI-I-0017"
created_at: 2025-12-07T00:57:34.541819+00:00
updated_at: 2025-12-07T04:19:41.588212+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
strategy_id: NULL
initiative_id: code-organization-refactoring
---

# Code Organization Refactoring - Split Monolithic Files Initiative

## Context

A deep analysis of the Cloacina codebase (35,651 lines across 3 crates) revealed that while the overall architecture is sound with clean layering and no circular dependencies, there are several monolithic files that have grown too large and mix multiple responsibilities. This impacts maintainability, testability, and contributor onboarding.

The codebase scores 8/10 for overall health, but these refactorings would improve long-term maintenance.

## Goals & Non-Goals

**Goals:**
- Split files over 1,500 lines into focused modules with single responsibilities
- Improve testability by separating concerns
- Maintain existing public API (internal refactor only)
- Improve code discoverability for contributors

**Non-Goals:**
- Changing public APIs
- Adding new functionality
- Performance optimization (covered by separate initiatives)
- Refactoring well-organized modules (packaging, cloacina-workflow)

## Detailed Design

### Priority 1: Critical Files (>1,700 lines)

#### 1.1 workflow.rs (1,798 lines)

Split into:
```
src/workflow/
  mod.rs          (~150 lines - public API and re-exports)
  metadata.rs     (~150 lines - WorkflowMetadata, versioning)
  graph.rs        (~400 lines - DependencyGraph, cycle detection, topo sort)
  builder.rs      (~300 lines - WorkflowBuilder fluent API)
  registry.rs     (~300 lines - global registry, constructors)
  types.rs        (~150 lines - Workflow struct, serialization)
```

#### 1.2 task_scheduler.rs (1,781 lines)

Split into:
```
src/task_scheduler/
  mod.rs              (~200 lines - TaskScheduler struct, public API)
  scheduler_loop.rs   (~400 lines - run_scheduling_loop, polling)
  state_manager.rs    (~350 lines - state transitions, readiness checks)
  recovery.rs         (~250 lines - orphan detection, recovery events)
  trigger_rules.rs    (~300 lines - TriggerRule, TriggerCondition, ValueOperator)
  context_manager.rs  (~200 lines - context loading, merging)
```

#### 1.3 default_runner.rs (1,728 lines)

Split into:
```
src/runner/
  mod.rs              (~100 lines - public API, DefaultRunner struct)
  config.rs           (~200 lines - DefaultRunnerConfig, builder)
  executor_setup.rs   (~200 lines - executor initialization)
  scheduler_setup.rs  (~200 lines - scheduler initialization)
  cron_setup.rs       (~150 lines - cron scheduler setup)
  reconciler_setup.rs (~150 lines - registry reconciliation)
  lifecycle.rs        (~300 lines - run method, shutdown, signals)
```

#### 1.4 dal/unified/task_execution.rs (1,539 lines)

Split into:
```
src/dal/unified/task_execution/
  mod.rs           (~200 lines - public API, DAL struct)
  crud.rs          (~300 lines - create, list, update operations)
  state.rs         (~300 lines - state transitions, queries)
  context.rs       (~250 lines - context aggregation, merging)
  claiming.rs      (~200 lines - claim, lock, release operations)
  queries.rs       (~200 lines - complex filtering, recovery queries)
```

### Priority 2: Large Files (1,000-1,100 lines)

#### 2.1 database/connection.rs (1,010 lines)

Split into:
```
src/database/
  pool.rs              (~250 lines - connection pool, pool builder)
  backend.rs           (~150 lines - BackendType enum, detection)
  connection_builder.rs (~200 lines - connection setup, migrations)
```

#### 2.2 workflow_registry.rs (1,094 lines)

Split loading, validation, and storage operations.

#### 2.3 reconciler.rs (1,036 lines)

Split conflict detection, validation, and update scheduling.

### Priority 3: Secondary Files (900-1,000 lines)

- task_registrar.rs (985 lines) - split extraction, validation, registration
- validator.rs (950 lines) - modular validators by concern
- cron_schedule.rs (1,032 lines) - split schedule CRUD from state queries
- cron_execution.rs (956 lines) - split by operation type

### Additional Improvements

#### lib.rs Reorganization

Create a prelude module:
```rust
pub mod prelude {
    pub use crate::context::Context;
    pub use crate::task::Task;
    pub use crate::workflow::Workflow;
    pub use crate::error::{TaskError, WorkflowError};
    pub use crate::retry::RetryPolicy;
}
```

## Alternatives Considered

**1. Leave as-is**: The code works, but long-term maintenance burden increases as files grow. Rejected due to contributor friction.

**2. Full rewrite**: Would allow ideal structure but risks introducing bugs. Rejected as unnecessary - the architecture is sound.

**3. Incremental refactoring (chosen)**: Split files one at a time, run tests after each change. Lowest risk, highest confidence.

## Implementation Plan

**Phase 1: Core Workflow/Scheduler**
- Split workflow.rs into module hierarchy
- Split task_scheduler.rs into module hierarchy
- Run full test suite after each split

**Phase 2: Runner/DAL**
- Split default_runner.rs
- Split task_execution.rs DAL

**Phase 3: Infrastructure**
- Split database/connection.rs
- Reorganize registry loader files
- Create prelude module

Each phase should be a separate PR to minimize review burden and risk.
