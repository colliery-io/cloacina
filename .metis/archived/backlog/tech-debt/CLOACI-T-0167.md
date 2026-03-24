---
id: improve-test-coverage-cron
level: task
title: "Improve test coverage: cron_recovery.rs (10.9% → 80%+)"
short_code: "CLOACI-T-0167"
created_at: 2026-03-16T01:01:40.125367+00:00
updated_at: 2026-03-16T01:01:40.125367+00:00
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

# Improve test coverage: cron_recovery.rs (10.9% → 80%+)

**Priority: P2 — Tech Debt**

## Objective

`cron_recovery.rs` has 230 lines at 10.9% line coverage. This module handles recovery of orphaned cron-triggered tasks after scheduler crashes. The existing `tests/integration/scheduler/recovery.rs` tests cover the event-driven scheduler recovery but not cron-specific recovery paths.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Line coverage ≥ 80% as measured by `cargo llvm-cov`
- [ ] Test: cron task orphaned during execution is recovered on restart
- [ ] Test: cron task at max retries is abandoned
- [ ] Test: cron recovery with no orphaned tasks is a no-op
- [ ] Test: multiple orphaned cron tasks recovered correctly
- [ ] Needs DB fixture (integration tests)

## Status Updates

*To be added during implementation*
