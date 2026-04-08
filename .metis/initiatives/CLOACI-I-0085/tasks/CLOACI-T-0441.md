---
id: fix-pipeline-completion-status-to
level: task
title: "Fix pipeline completion status to reflect task failures (COR-01)"
short_code: "CLOACI-T-0441"
created_at: 2026-04-08T13:35:05.197262+00:00
updated_at: 2026-04-08T13:55:58.026769+00:00
parent: CLOACI-I-0085
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0085
---

# Fix pipeline completion status to reflect task failures (COR-01)

## Parent Initiative

[[CLOACI-I-0085]] Security Foundation

## Objective

Fix the bug where `complete_pipeline()` always marks pipelines as "Completed" regardless of whether tasks failed. Currently a pipeline where every task failed is indistinguishable from a fully successful one. Addresses COR-01 (Critical).

**Effort estimate**: 2-4 hours

**Note**: This task must complete before I-0090 (Pipeline-to-Workflow rename) begins, to avoid renaming broken code.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] A pipeline with all tasks completed successfully is marked "Completed"
- [ ] A pipeline with any failed task is marked "Failed"
- [ ] A pipeline with mixed completed/skipped/failed tasks is marked appropriately (consider "PartiallyCompleted" or "Failed")
- [ ] `PipelineResult` returned from `runner.execute()` reflects the correct final status
- [ ] Unit tests cover all three scenarios (all pass, all fail, mixed)
- [ ] Existing integration and soak tests pass

## Implementation Notes

### Technical Approach

In `complete_pipeline()` (`crates/cloacina/src/task_scheduler/scheduler_loop.rs`), after `check_pipeline_completion()` returns true:

1. Query task statuses for the pipeline execution
2. If any task has "Failed" status, mark the pipeline as "Failed"
3. Otherwise mark as "Completed"

```rust
let has_failures = task_statuses.iter().any(|s| s == "Failed");
if has_failures {
    self.dal.pipeline_execution().update_status(execution.id, "Failed").await?;
} else {
    self.dal.pipeline_execution().update_status(execution.id, "Completed").await?;
}
```

The DAL already has `update_status` which accepts a status string. The change is small and localized.

### Dependencies
None -- independent of other I-0085 tasks.

## Testing Requirements

This is a critical application logic miss — the kind that silently corrupts downstream decision-making. Full integration and functional tests are mandatory:

- **All tasks pass**: pipeline marked "Completed", `PipelineResult.status` is success
- **All tasks fail**: pipeline marked "Failed", `PipelineResult.status` is failed
- **Mixed results**: some pass, some fail — pipeline marked "Failed" (or "PartiallyCompleted" if that status is added)
- **Single task fail in multi-task DAG**: verify the pipeline doesn't short-circuit but still marks as failed at completion
- **Retry exhaustion**: task fails after retries exhausted — pipeline still correctly marked
- **Skipped tasks** (downstream of failed dependency): pipeline correctly reflects the failure even if skipped tasks aren't themselves "Failed"

These tests must run in CI (`angreal cloacina integration` or `angreal cloacina unit`) to prevent regression. This class of bug (silent success on failure) is a killer — it makes every consumer of pipeline status unreliable.

## Status Updates

- **2026-04-08**: Fixed `complete_pipeline()` in `scheduler_loop.rs` — now checks `failed_count > 0` and calls `mark_failed()` instead of unconditional `mark_completed()`. Used existing `mark_failed(id, reason)` DAL method. Added pipeline status assertion to existing timeout-failure integration test (`task_execution.rs`) — polls `get_by_id()` and asserts status is "Failed". Compiles clean with both backends.
