---
id: introduce-dal-trait-for-testability
level: task
title: "Introduce DAL trait for testability"
short_code: "CLOACI-T-0482"
created_at: 2026-04-11T14:49:51.022862+00:00
updated_at: 2026-04-11T14:49:51.022862+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


exit_criteria_met: false
initiative_id: NULL
---

# Introduce DAL trait for testability

## Objective

Define a `trait DataAccessLayer` with sub-traits per entity to enable in-memory mock implementations for unit testing. Works against the current dual-backend pattern as-is.

## Review Finding References

EVO-005, EVO-006 (REC-012)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P3 - Low (when time permits)

## Acceptance Criteria

## Acceptance Criteria

- [ ] `trait DataAccessLayer` defined with methods returning `&dyn TaskExecutionOps`, `&dyn WorkflowExecutionOps`, etc.
- [ ] `InMemoryDAL` implementation for testing
- [ ] At least one core component (`ThreadTaskExecutor` or `DefaultDispatcher`) refactored to accept `Arc<dyn DataAccessLayer>`
- [ ] At least one test using `InMemoryDAL` instead of database

## Implementation Notes

### Dependencies
None. Independent of DAL backend unification.

## Status Updates

*To be added during implementation*
