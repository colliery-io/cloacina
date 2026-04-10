---
id: phase-a-add-workflow-type-aliases
level: task
title: "Phase A: Add Workflow type aliases for Pipeline types and update prelude exports (LEG-01)"
short_code: "CLOACI-T-0460"
created_at: 2026-04-09T15:47:34.721377+00:00
updated_at: 2026-04-09T16:01:42.304494+00:00
parent: CLOACI-I-0090
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0090
---

# Phase A: Add Workflow type aliases for Pipeline types and update prelude exports (LEG-01)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0090]]

## Objective

Add type aliases so consumers can start using `WorkflowResult`, `WorkflowStatus`, etc. without breaking existing `Pipeline*` code. This is the non-breaking bridge step.

**Effort**: 2-4 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Type aliases added next to each Pipeline type:
  - `pub type WorkflowExecutionResult = PipelineResult`
  - `pub type WorkflowStatus = PipelineStatus`
  - `pub type WorkflowExecutionError = PipelineError`
  - `pub type WorkflowExecution = PipelineExecution` (executor version)
  - `pub type WorkflowExecutionDAL = PipelineExecutionDAL`
- [ ] Prelude updated to export the new Workflow names alongside Pipeline names
- [ ] `#[deprecated]` attributes added to Pipeline re-exports in prelude (with note pointing to new names)
- [ ] Python bindings add `WorkflowResult`/`WorkflowStatus` as aliases in the `cloaca` module
- [ ] All existing code continues to compile without changes (aliases are additive)
- [ ] All tests pass

## Implementation Notes

### Technical Approach

In `crates/cloacina/src/executor/pipeline_executor.rs`, after each `pub enum`/`pub struct`, add:
```rust
/// Alias: prefer `WorkflowStatus` over `PipelineStatus`.
pub type WorkflowStatus = PipelineStatus;
```

In `crates/cloacina/src/lib.rs` prelude, add the new names and deprecate old ones:
```rust
#[deprecated(note = "Use WorkflowExecutionResult instead")]
pub use crate::executor::PipelineResult;
pub use crate::executor::PipelineResult as WorkflowExecutionResult;
```

### Dependencies
None. This is additive and non-breaking.

## Status Updates

- **2026-04-09**: Added 4 type aliases in `pipeline_executor.rs`: `WorkflowExecutionError`, `WorkflowStatus`, `WorkflowExecutionResult`, `WorkflowExecution`. `PipelineExecutor` is a trait (can't type-alias) — will be renamed directly in Phase B. Added `WorkflowExecutionDAL` alias in `dal/unified/pipeline_execution.rs`. Updated executor mod.rs and lib.rs re-exports. Deprecated attributes deferred to Phase C (removing aliases) — adding `#[deprecated]` now would generate warnings across the entire codebase before migration. Compiles clean.
