---
id: remove-stub-python-start-stop
level: task
title: "Remove stub Python start/stop methods and fix runner error propagation"
short_code: "CLOACI-T-0480"
created_at: 2026-04-11T14:49:48.669569+00:00
updated_at: 2026-04-13T02:07:56.034453+00:00
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

# Remove stub Python start/stop methods and fix runner error propagation

## Objective

Delete Python `start()` and `stop()` stubs that raise at runtime. Replace `.expect()` in `PyDefaultRunner::new()` with proper error propagation.

## Review Finding References

API-001, API-009 (REC-009)

## Backlog Item Details

### Type
- [x] Tech Debt - Code improvement or refactoring

### Priority
- [x] P2 - Medium (nice to have)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `start()` and `stop()` removed from `PyDefaultRunner` `#[pymethods]` block
- [ ] `PyDefaultRunner::new()` and `with_config()` return `PyResult` with proper error on DB failure (not panic)
- [ ] Python smoke tests pass

## Implementation Notes

### Key Files
- `crates/cloacina/src/python/runner.rs`

### Dependencies
None. ~3 hours.

## Status Updates

*To be added during implementation*