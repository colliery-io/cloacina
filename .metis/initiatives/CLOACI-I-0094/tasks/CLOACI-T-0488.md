---
id: code-level-pipeline-to-workflow
level: task
title: "Code-level pipeline-to-workflow rename (no DB changes)"
short_code: "CLOACI-T-0488"
created_at: 2026-04-14T00:57:33.918117+00:00
updated_at: 2026-04-14T02:56:10.147485+00:00
parent: CLOACI-I-0094
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0094
---

# Code-level pipeline-to-workflow rename (no DB changes)

## Parent Initiative

[[CLOACI-I-0094]]

## Objective

Rename all internal Rust code references from "pipeline" to "workflow" without touching database schema or migrations. This covers file renames, type/variable renames, error messages, log strings, and Python binding property names.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Files renamed: `pipeline_executor.rs` → `workflow_executor.rs`, `pipeline_result.rs` → `workflow_result.rs`, `pipeline_executor_impl.rs` → `workflow_executor_impl.rs`, `pipeline_execution.rs` (model) → `workflow_execution.rs`
- [ ] All internal type/variable references updated (`pipeline_execution_id` → `workflow_execution_id`, `pipeline_name` → `workflow_name`, etc.)
- [ ] Error messages and log strings say "workflow" not "pipeline"
- [ ] Python binding config: `pipeline_timeout_seconds` → `workflow_timeout_seconds` (with deprecation alias if needed)
- [ ] DAL code left unchanged (still references DB column names as-is — that's T-0489)
- [ ] `cargo check` passes for all crates
- [ ] Unit tests pass

## Implementation Notes

### Key Areas (~700 occurrences, excluding DAL)
- `executor/pipeline_executor.rs` — types, error variants, functions
- `executor/types.rs` — `PipelineExecutionId` and related
- `models/pipeline_execution.rs` — model struct
- `execution_planner/` — state_manager, recovery, scheduler_loop, context_manager
- `runner/default_runner/pipeline_executor_impl.rs`, `pipeline_result.rs`
- `python/bindings/runner.rs`, `context.rs`
- `error.rs`, `lib.rs`, `cron_trigger_scheduler.rs`

### Approach
- Rename files first with `git mv`
- Then find-and-replace identifiers systematically
- Keep DB column references in DAL code unchanged (they still map to `pipeline_executions` table)

### Dependencies
- None. T-0489 depends on this.

## Status Updates

- 2026-04-13: Completed. 4 files renamed, domain model fields renamed, error messages updated, DAL conversion layer updated, 833 unit tests pass. Committed as 30d18b6.
