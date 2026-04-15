---
id: implement-phantom-prometheus
level: task
title: "Implement phantom Prometheus metrics and add tracing spans"
short_code: "CLOACI-T-0479"
created_at: 2026-04-11T14:49:47.727296+00:00
updated_at: 2026-04-13T02:07:55.829346+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Implement phantom Prometheus metrics and add tracing spans

## Objective

Record the 5 described-but-never-emitted Prometheus metrics and add `#[tracing::instrument]` spans at key lifecycle boundaries.

## Review Finding References

OPS-002, OPS-004 (REC-008)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All 5 phantom metrics (`api_requests_total`, `pipeline_duration_seconds`, `task_duration_seconds`, `active_pipelines`, `active_tasks`) actually emitted
- [ ] Tracing spans on `schedule_workflow_execution`, `dispatch_ready_tasks`, `ThreadTaskExecutor::execute`, `complete_pipeline`
- [ ] `/metrics` endpoint returns non-zero values after workflow execution

## Implementation Notes

### Dependencies
CLOACI-T-0474 (double state-update fix) should be done first — state ownership determines which component records each metric.

## Status Updates

*To be added during implementation*