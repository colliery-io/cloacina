---
id: add-heartbeat-driven-cancellation
level: task
title: "Add heartbeat-driven cancellation for claim-lost tasks"
short_code: "CLOACI-T-0481"
created_at: 2026-04-11T14:49:50.083060+00:00
updated_at: 2026-04-11T14:49:50.083060+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Add heartbeat-driven cancellation for claim-lost tasks

## Objective

When heartbeat detects `ClaimLost`, cancel the running task via `CancellationToken` to prevent it from saving results that belong to another runner.

## Review Finding References

COR-005 (REC-010)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

## Acceptance Criteria

- [ ] `CancellationToken` passed into `ThreadTaskExecutor::execute`
- [ ] Heartbeat triggers cancellation on `ClaimLost`
- [ ] `complete_task_transaction` checks token before saving
- [ ] No regression in existing tests

## Implementation Notes

### Key Files
- `crates/cloacina/src/executor/thread_task_executor.rs`
- `crates/cloacina/src/executor/heartbeat.rs`

### Dependencies
None.

## Status Updates

*To be added during implementation*
