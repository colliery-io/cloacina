---
id: minimal-footprint-crate-for
level: task
title: "Minimal Footprint Crate for Workflow Compilation and Distribution"
short_code: "CLOACI-T-0013"
created_at: 2025-12-04T00:01:29.041141+00:00
updated_at: 2025-12-05T22:34:24.337220+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# Minimal Footprint Crate for Workflow Compilation and Distribution

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[Parent Initiative]]

## Objective

Create a minimal `cloacina-workflow` crate that contains only the types and macros needed to compile packaged workflows. This allows workflow developers to depend on a lightweight crate without pulling in the full runtime, database drivers, or execution engine.

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P1 - High (important for user experience)

### Technical Debt Impact
- **Current Problems**:
  - Workflow authors must depend on the full `cloacina` crate to compile workflows
  - This pulls in Diesel, database drivers (libpq), tokio runtime, and other heavy dependencies
  - Increases compile times and binary sizes for workflow packages
  - Makes cross-compilation harder due to native database driver requirements
- **Benefits of Fixing**:
  - Workflow authors only need `cloacina-workflow` (minimal deps: serde, async-trait)
  - Faster workflow compilation
  - Smaller `.cloacina` package sizes
  - Easier cross-compilation for workflow distribution
  - Clear separation between "authoring workflows" and "running workflows"
- **Risk Assessment**: Medium - requires careful API design to ensure workflows compiled with minimal crate are compatible with full runtime

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `cloacina-workflow` crate created with minimal dependencies
- [ ] Contains: Context, TaskError, task macro, packaged_workflow macro
- [ ] Does NOT contain: Database, DAL, Runner, Scheduler, Registry internals
- [ ] Existing workflows can be recompiled against new crate
- [ ] Full `cloacina` crate re-exports `cloacina-workflow` for backwards compatibility
- [ ] Documentation updated with workflow authoring guide using minimal crate

## Implementation Notes

### Key Design Decisions

#### Decision 1: Context Design - RuntimeContext Wrapper (Composition)
RuntimeContext wraps the minimal Context and adds runtime fields:
- `cloacina-workflow::Context<T>` - minimal, only `data: HashMap<String, T>`
- `cloacina::RuntimeContext<T>` - wraps Context, adds `execution_scope` and `dependency_loader`
- Task trait signature stays clean: `fn execute(Context<T>)` not `RuntimeContext<T>`

#### Decision 2: Dependency Loading - Pre-inject Pattern
Executor loads ALL dependency data into context BEFORE task execution:
- Remove `load_from_dependencies_and_cache()` from public API
- Executor calls `prepare_context_for_task()` before each `task.execute()`
- Tasks just use `ctx.get("key")` - simpler, no async loading needed
- Matches FFI pattern (context is pre-populated JSON)

#### Decision 3: Macro crate_path Inheritance
`packaged_workflow` sets `crate_path` once, all contained `#[task]` macros inherit it:
- Individual `#[task]` can override with explicit `crate_path` if needed
- Default remains `cloacina` when no crate_path specified

### Crate Structure
```
cloacina/
  Cargo.toml                    # Workspace root (add cloacina-workflow member)
  cloacina-workflow/            # NEW: Minimal workflow primitives
    Cargo.toml
    src/
      lib.rs                    # Re-exports all public types
      context.rs                # Minimal Context<T> (data only)
      error.rs                  # Minimal errors (no diesel/tokio)
      task.rs                   # Task trait, TaskState
      namespace.rs              # TaskNamespace
      retry.rs                  # RetryPolicy, BackoffStrategy, RetryCondition
  cloacina-macros/              # Existing (modified for crate_path)
  cloacina/                     # Main crate (depends on cloacina-workflow)
```

### Dependencies for cloacina-workflow (Minimal)
```toml
[dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
rand = "0.8"          # For retry jitter
tracing = "0.1"       # Optional logging
```

**Explicitly NOT included:** diesel, deadpool, tokio (full), libloading, petgraph, libsqlite3-sys

### Implementation Phases

#### Phase 1: Create cloacina-workflow Crate
- Create `cloacina-workflow/Cargo.toml` with minimal deps
- Create minimal `Context<T>` (only data, no runtime fields)
- Create minimal errors (no diesel/tokio variants)
- Copy `TaskNamespace`, `TaskState`, `Task` trait, `RetryPolicy`

#### Phase 2: Update Main cloacina Crate
- Add dependency on `cloacina-workflow`
- Create `RuntimeContext<T>` wrapper with runtime fields
- Re-export workflow types for backwards compatibility
- Update executor for pre-inject dependency pattern

#### Phase 3: Update cloacina-macros
- Add `crate_path` attribute to `packaged_workflow`
- Implement crate_path inheritance for contained tasks
- Replace hardcoded `cloacina::` with configurable path

#### Phase 4: Testing and Documentation
- Unit tests for cloacina-workflow types
- Integration tests for backwards compatibility
- Workflow authoring guide using minimal crate

### Breaking Changes (Intentional)
These methods removed from public Context API:
- `context.load_from_dependencies_and_cache()` - replaced by pre-injection
- `context.get_with_dependencies()` - replaced by pre-injection
- `context.set_dependency_loader()` - internal to executor
- `context.set_execution_scope()` - internal to executor

### Critical Files to Modify
**New Files:**
- `cloacina-workflow/Cargo.toml`
- `cloacina-workflow/src/{lib,context,error,task,namespace,retry}.rs`
- `cloacina/src/runtime_context.rs`

**Modified Files:**
- `Cargo.toml` (workspace)
- `cloacina/Cargo.toml`
- `cloacina/src/lib.rs`
- `cloacina/src/context.rs`
- `cloacina/src/error.rs`
- `cloacina/src/executor/pipeline_executor.rs`
- `cloacina-macros/src/tasks.rs`
- `cloacina-macros/src/packaged_workflow.rs`

### Blocked By
- CLOACI-T-0012 (repo restructure) - easier to create new crate in `crates/` structure

## Status Updates **[REQUIRED]**

### 2025-12-04: Deep Dive Analysis Complete
- Completed comprehensive codebase analysis
- Identified all types to extract and their dependencies
- Designed RuntimeContext wrapper pattern for clean separation
- Designed pre-inject dependency pattern (simpler than lazy loading)
- Designed macro crate_path inheritance
- Full implementation plan documented
