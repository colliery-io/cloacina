---
id: improve-test-coverage-cloacina
level: task
title: "Improve test coverage: cloacina-workflow/retry.rs (67% → 90%+)"
short_code: "CLOACI-T-0171"
created_at: 2026-03-16T01:01:45.430528+00:00
updated_at: 2026-03-16T01:01:45.430528+00:00
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

# Improve test coverage: cloacina-workflow/retry.rs (67% → 90%+)

**Priority: P3 — Tech Debt**

## Objective

`cloacina-workflow/src/retry.rs` has 297 lines at 66.7% line coverage. The retry module provides backoff strategies (fixed, exponential, linear) used by the task execution pipeline. The untested paths are primarily the exponential and linear backoff calculation edge cases and the retry policy builder.

## Acceptance Criteria

- [ ] Line coverage ≥ 90% as measured by `cargo llvm-cov`
- [ ] Test: exponential backoff with max_delay cap
- [ ] Test: exponential backoff with jitter
- [ ] Test: linear backoff increment
- [ ] Test: retry policy builder with all options
- [ ] Test: retry exhaustion (max attempts reached)
- [ ] Test: zero-delay retry
- [ ] All tests are unit tests (no DB needed)

## Status Updates

*To be added during implementation*
