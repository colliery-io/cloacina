---
id: implement-or-remove
level: task
title: "Implement or remove RouteToSideChannel and fix WallClockWindow.mark_drained"
short_code: "CLOACI-T-0157"
created_at: 2026-03-15T18:24:33.009664+00:00
updated_at: 2026-03-15T19:26:09.341689+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Implement or remove RouteToSideChannel and fix WallClockWindow.mark_drained

**Priority: P1 — HIGH**
**Parent**: [[CLOACI-I-0025]]

## Objective

Fix two dead-code / no-op issues in the late arrival and trigger policy implementations:

1. **`RouteToSideChannel`** (`scheduler.rs:467-477`): The `LateArrivalPolicy::RouteToSideChannel { task_name }` variant exists in the enum and can be configured by users, but the match arm is just a `debug!` log — no actual side-channel storage or task routing. Using this policy silently drops late data.

2. **`WallClockWindow.mark_drained()`** (`trigger_policy.rs:67-70`): The method exists and is public but is never called anywhere in the codebase. The `last_drain_at` field is never updated, so the WallClockWindow trigger policy's timing is broken. This policy is dead code in practice.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **Decision**: Either implement `RouteToSideChannel` (route late boundaries to a named side-channel task) or remove the variant from the enum and document that it's deferred
- [ ] If implemented: late boundaries matching this policy are stored and routable to a specified task
- [ ] If removed: compile error for any user code referencing it (clean break)
- [ ] `mark_drained()` is called by the scheduler after each accumulator drain when `WallClockWindow` is the active trigger policy
- [ ] Unit test: WallClockWindow fires correctly based on wall-clock intervals after drains
- [ ] Unit test or compile-time guarantee: `RouteToSideChannel` either works or doesn't exist

## Implementation Notes

- For `mark_drained()`: the scheduler's drain path (`scheduler.rs` around line 524-530) should call `trigger_policy.mark_drained()` if the policy trait exposes it. May need to add `mark_drained(&mut self)` to the `TriggerPolicy` trait with a default no-op impl.
- For `RouteToSideChannel`: recommend removing for now and filing a separate task when the design is ready. A no-op match arm that silently drops data is worse than a compile error.

## Status Updates

### 2026-03-15 — Completed
- **RouteToSideChannel removed**: Deleted the `LateArrivalPolicy::RouteToSideChannel` variant, its match arm in the scheduler, and its test. A no-op that silently drops data is worse than a compile error. Comment left noting it will be re-added when the side-channel design is complete.
- **mark_drained() fixed**: Added `fn mark_drained(&mut self) {}` with default no-op to the `TriggerPolicy` trait. `WallClockWindow` overrides it to reset `last_drain_at`. Both `SimpleAccumulator::drain()` and `WindowedAccumulator::drain()` now call `self.policy.mark_drained()` after clearing the buffer.
- All 411 unit tests pass (1 less: removed RouteToSideChannel test)
