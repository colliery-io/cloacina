---
id: improve-test-coverage-cloacina
level: task
title: "Improve test coverage: cloacina-testing assertions.rs (65%) and result.rs (42%)"
short_code: "CLOACI-T-0172"
created_at: 2026-03-16T01:01:47.566896+00:00
updated_at: 2026-03-16T01:01:47.566896+00:00
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

# Improve test coverage: cloacina-testing assertions.rs (65%) and result.rs (42%)

**Priority: P3 — Tech Debt**

## Objective

Two files in `cloacina-testing` crate are poorly covered:
- `assertions.rs` — 62 lines at 64.5%. Test assertion helpers (assert_task_succeeded, assert_context_contains, etc.)
- `result.rs` — 48 lines at 41.7%. Result wrapper types for test outcomes.

These are testing utilities — their coverage gap means some assertion/result helpers exist but are never used by any test in the codebase.

## Acceptance Criteria

- [ ] `assertions.rs` line coverage ≥ 90%
- [ ] `result.rs` line coverage ≥ 80%
- [ ] Review: are the uncovered helpers dead code? If so, remove them rather than adding artificial tests
- [ ] If helpers are useful but unused, add tests that demonstrate their use
- [ ] All tests are unit tests (no DB needed)

## Status Updates

*To be added during implementation*
