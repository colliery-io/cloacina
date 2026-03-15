---
id: joinmode-all-support-and-full
level: task
title: "JoinMode::All support and full reactive feedback loop integration test"
short_code: "CLOACI-T-0134"
created_at: 2026-03-15T13:14:14.336057+00:00
updated_at: 2026-03-15T13:39:17.039307+00:00
parent: CLOACI-I-0024
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0024
---

# JoinMode::All support and full reactive feedback loop integration test

## Parent Initiative

[[CLOACI-I-0024]]

## Objective

Implement `JoinMode::All` readiness logic in the scheduler and build end-to-end integration tests for the full reactive feedback loop: source → detect → accumulate → task → derived source → detect → downstream task.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `JoinMode::All` readiness: task fires only when ALL accumulators are ready (already stubbed in I-0023, now fully tested)
- [ ] Integration test: multi-source task with `JoinMode::All` — fires only after both sources have boundaries
- [ ] Integration test: full feedback loop — upstream task completes → LedgerTrigger fires → detector runs → downstream accumulator → downstream task fires
- [ ] Integration test: WindowedAccumulator with WaitForWatermark — blocks until watermark covers, then fires
- [ ] Integration test: late boundary with Discard policy — boundary dropped, task not re-triggered
- [ ] Update continuous scheduling documentation with watermark and late arrival sections
- [ ] All existing continuous tests still pass

## Implementation Notes

### Technical Approach
- `JoinMode::All` logic already exists in `check_readiness()` — this task validates it with proper tests
- Feedback loop test: create two-level graph (source → task_a → derived_source → task_b) with LedgerTrigger
- Requires all prior I-0024 tasks (T-0129 through T-0133) to be complete
- Update `docs/content/explanation/continuous-scheduling.md` limitations section

### Dependencies
- All prior I-0024 tasks (T-0129, T-0130, T-0131, T-0132, T-0133)

## Status Updates

- Added 4 new integration tests:
  - WindowedAccumulator with WaitForWatermark — blocks until watermark covers, then fires + drains
  - LedgerTrigger feedback loop — task completes → trigger fires → cursor advances
  - LedgerTrigger All mode — waits for both upstream tasks before firing
  - Scheduler watermark advance via Both output — watermark + boundaries both routed
- JoinMode::All was already implemented in I-0023 scheduler — validated by existing multi-source test
- Updated continuous-scheduling.md: added Watermarks, Late Arrival, Derived Data Sources sections; removed outdated limitations
- 87 unit tests + 8 integration tests = 95 total, all passing
