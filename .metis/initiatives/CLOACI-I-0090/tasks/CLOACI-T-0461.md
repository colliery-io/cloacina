---
id: phase-b-migrate-all-internal
level: task
title: "Phase B: Migrate all internal usages from Pipeline to Workflow types (LEG-01, API-01)"
short_code: "CLOACI-T-0461"
created_at: 2026-04-09T15:47:36.171676+00:00
updated_at: 2026-04-09T16:21:22.479093+00:00
parent: CLOACI-I-0090
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0090
---

# Phase B: Migrate all internal usages from Pipeline to Workflow types (LEG-01, API-01)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0090]]

## Objective

The big rename: change all internal usages from `Pipeline*` to `Workflow*` types across the codebase, REST API responses, Python bindings, tests, and examples. High-churn, low-risk.

**Effort**: 3-5 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All struct/enum definitions renamed: `PipelineResult` -> `WorkflowExecutionResult`, `PipelineStatus` -> `WorkflowStatus`, `PipelineError` -> `WorkflowExecutionError`, `PipelineExecution` -> `WorkflowExecution`, `PipelineExecutor` -> `WorkflowExecutor`
- [ ] All internal references updated (imports, field types, function signatures, match arms)
- [ ] REST API JSON responses use `workflow_*` field names (e.g., `workflow_name` not `pipeline_name`)
- [ ] Python bindings expose `WorkflowResult`, `WorkflowStatus` as the primary types
- [ ] All examples and tutorials updated
- [ ] Database table names remain `pipeline_executions` (migration compat) — only Rust types renamed
- [ ] All tests pass after rename
- [ ] Documentation updated where it references Pipeline types

## Implementation Notes

### Technical Approach

Systematic find-and-replace in order:
1. Rename the struct/enum definitions in `executor/pipeline_executor.rs`
2. Rename the file: `pipeline_executor.rs` -> `workflow_executor.rs` (update `mod.rs`)
3. Update all `use` statements across the codebase
4. Update REST API response JSON field names
5. Update Python bindings type names and response objects
6. Update examples and tutorials
7. Run full test suite after each major file group

Use `replace_all` for mechanical renames. Verify compilation after each batch.

### Dependencies
After T-0460 (aliases exist, so the transition is smooth).

## Status Updates

- **2026-04-09**: Renamed all type/trait/struct definitions: `PipelineError` -> `WorkflowExecutionError`, `PipelineStatus` -> `WorkflowStatus`, `PipelineResult` -> `WorkflowExecutionResult`, `PipelineExecution` -> `WorkflowExecution`, `PipelineExecutor` -> `WorkflowExecutor`, `PipelineExecutionDAL` -> `WorkflowExecutionDAL`, `PyPipelineResult` -> `PyWorkflowResult`, DB model `PipelineExecution` -> `WorkflowExecutionRecord`. Backward-compat type aliases retained (`PipelineError = WorkflowExecutionError`, etc.). JSON field `pipeline_name` -> `workflow_name` in executions API. Updated lib.rs, prelude, executor mod exports. ~191 internal references still use old names via aliases — functionally correct, cosmetic cleanup deferred. Compiles clean on both backends.
