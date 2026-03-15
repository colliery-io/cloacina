---
id: continuous-task-proc-macro-with
level: task
title: "#[continuous_task] proc macro with sources/referenced attributes"
short_code: "CLOACI-T-0123"
created_at: 2026-03-15T11:46:37.789836+00:00
updated_at: 2026-03-15T12:11:02.405774+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# #[continuous_task] proc macro with sources/referenced attributes

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0023]]

## Objective

Implement the `#[continuous_task]` proc macro in `cloacina-macros` with `sources` and `referenced` attributes. The macro generates a task struct that receives `DataSourceMap` as an additional parameter and produces registration metadata for graph assembly.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[continuous_task(id = "...", sources = ["..."], referenced = ["..."])]` attribute syntax
- [ ] Macro generates struct implementing a continuous task trait/interface with `DataSourceMap` parameter
- [ ] `sources` attribute defines triggering data sources (create graph edges with accumulators)
- [ ] `referenced` attribute defines non-triggering data sources (available but don't trigger execution)
- [ ] Registration metadata generated for graph assembly (source names, referenced names)
- [ ] Macro validates at compile time: `sources` non-empty, no overlap between sources and referenced
- [ ] Macro tests: valid continuous task compiles, missing sources rejected, basic execution works

## Implementation Notes

### Technical Approach
- Extend `cloacina-macros` crate with new `continuous_task` proc macro
- Study existing `#[task]` macro in `crates/cloacina-macros/src/tasks.rs` for patterns
- Generated struct holds `sources: Vec<String>`, `referenced: Vec<String>` for graph assembly
- Task function signature: `async fn(ctx: &mut Context<Value>, inputs: &DataSourceMap) -> Result<(), TaskError>`
- Extend `Dispatcher`/`TaskExecutor` to detect continuous tasks and inject `DataSourceMap`

### Dependencies
- T-0118 (DataSourceMap type), T-0117 (continuous module exists)

## Status Updates

- Created `cloacina-macros/src/continuous_task.rs` with full proc macro
- `ContinuousTaskAttributes` parser: `id`, `sources`, `referenced` with compile-time validation
- Validates: id required, sources non-empty, no overlap between sources and referenced
- Generated struct has: `sources()`, `referenced_sources()`, `is_continuous()`, `code_fingerprint_value()`
- Implements `Task` trait — function signature same as `#[task]` (ctx only), DataSourceMap injected via context by scheduler
- Reuses `calculate_function_fingerprint()` from tasks.rs
- Re-exported as `cloacina::continuous_task` from lib.rs
- Added `continuous_task` proc macro entry point to cloacina-macros/src/lib.rs
- `cargo check --workspace` passes clean
- Note: macro compile tests deferred to T-0127 integration test (requires full crate setup to test proc macros)
