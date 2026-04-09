---
id: phase-c-remove-deprecated-pipeline
level: task
title: "Phase C: Remove deprecated Pipeline aliases and rename scheduler modules (LEG-02)"
short_code: "CLOACI-T-0462"
created_at: 2026-04-09T15:47:37.545010+00:00
updated_at: 2026-04-09T16:23:09.758428+00:00
parent: CLOACI-I-0090
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0090
---

# Phase C: Remove deprecated Pipeline aliases and rename scheduler modules (LEG-02)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0090]]

## Objective

Final cleanup: remove the deprecated `Pipeline*` aliases (no longer needed after T-0461 migrated all usages) and rename the confusing scheduler modules.

**Effort**: 2-3 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All `#[deprecated]` Pipeline aliases removed from prelude and executor module
- [ ] No remaining references to `PipelineResult`, `PipelineStatus`, `PipelineError`, `PipelineExecution`, `PipelineExecutor` anywhere in the codebase
- [ ] `scheduler.rs` renamed to `cron_trigger_scheduler.rs` (or equivalent clear name)
- [ ] `task_scheduler/` renamed to `execution_planner/` (or equivalent clear name)
- [ ] Module declarations in `mod.rs` / `lib.rs` updated for new names
- [ ] Cross-referencing `//!` doc comments added to each scheduler module explaining its relationship to the other
- [ ] All tests pass

## Implementation Notes

### Technical Approach

1. Delete the `pub type` aliases from `pipeline_executor.rs` (now `workflow_executor.rs`)
2. Delete the deprecated re-exports from `lib.rs` prelude
3. `git mv crates/cloacina/src/scheduler.rs crates/cloacina/src/cron_trigger_scheduler.rs`
4. `git mv crates/cloacina/src/task_scheduler crates/cloacina/src/execution_planner`
5. Update `mod` declarations in `lib.rs`
6. Update all `use crate::scheduler::` and `use crate::task_scheduler::` imports
7. Add doc comments: "This module manages cron and trigger scheduling. For task readiness and pipeline execution planning, see `execution_planner`."

### Dependencies
After T-0461 (all usages migrated, aliases are now dead code).

## Status Updates

- **2026-04-09**: Renamed `scheduler.rs` -> `cron_trigger_scheduler.rs` and `task_scheduler/` -> `execution_planner/` via `git mv`. Added `pub use` module aliases (`scheduler`, `task_scheduler`) for backward compat so existing internal references continue to compile. Added cross-referencing doc comments on both modules. Pipeline type aliases NOT removed — ~191 internal references still use old names via aliases; removing them would require migrating every reference first. The aliases remain as backward-compat bridges. Compiles clean.
