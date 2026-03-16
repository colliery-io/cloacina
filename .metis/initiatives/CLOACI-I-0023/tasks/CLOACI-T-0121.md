---
id: triggerpolicy-trait-with-immediate
level: task
title: "TriggerPolicy trait with Immediate and WallClockWindow implementations"
short_code: "CLOACI-T-0121"
created_at: 2026-03-15T11:46:30.309110+00:00
updated_at: 2026-03-15T12:03:26.810816+00:00
parent: CLOACI-I-0023
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0023
---

# TriggerPolicy trait with Immediate and WallClockWindow implementations

## Parent Initiative

[[CLOACI-I-0023]]

## Objective

Implement the `TriggerPolicy` trait and two framework-provided implementations: `Immediate` and `WallClockWindow`, as specified in CLOACI-S-0005. These control when an accumulator fires its downstream task.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TriggerPolicy` trait with `should_fire(&self, buffer: &[BufferedBoundary]) -> bool`
- [ ] `Immediate` policy — returns true if buffer is non-empty
- [ ] `WallClockWindow` policy — returns true if wall clock time since last drain exceeds configured duration
- [ ] `TriggerPolicy` is `Send + Sync` (stored in accumulators across threads)
- [ ] Unit tests: Immediate fires on first boundary, WallClockWindow respects duration, WallClockWindow doesn't fire early

## Implementation Notes

### Technical Approach
- Trait and impls in `continuous/trigger_policy.rs`
- `WallClockWindow` needs `last_drain_at: Option<Instant>` — updated by the accumulator on drain, or passed as context
- Note: `Any`/`All` composition deferred to CLOACI-I-0025, but trait design should not preclude it

### Dependencies
- T-0117 (BufferedBoundary type)

## Status Updates

- Created `continuous/trigger_policy.rs`
- `TriggerPolicy` trait (Send + Sync) with `should_fire(&self, buffer: &[BufferedBoundary]) -> bool`
- `Immediate` — fires when buffer non-empty
- `WallClockWindow` — fires when `last_drain_at.elapsed() >= duration`, with `mark_drained()` for accumulator callback
- 6 passing tests: immediate fires/empty, window fires/early/empty/mark_drained
