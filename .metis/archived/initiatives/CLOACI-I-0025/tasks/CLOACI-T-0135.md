---
id: triggerpolicy-any-all-composition
level: task
title: "TriggerPolicy Any/All composition + BoundaryCount + WallClockDebounce"
short_code: "CLOACI-T-0135"
created_at: 2026-03-15T13:51:27.826684+00:00
updated_at: 2026-03-15T13:55:59.997187+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# TriggerPolicy Any/All composition + BoundaryCount + WallClockDebounce

## Parent Initiative

[[CLOACI-I-0025]]

## Objective

Implement `Any`/`All` TriggerPolicy combinators and two additional policy presets: `WallClockDebounce` and `BoundaryCount`. These enable patterns like "every 5 minutes OR 20 boundaries" or "at least 1000 rows AND at least 1 minute since last drain."

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `AnyPolicy(Vec<Box<dyn TriggerPolicy>>)` — fires when ANY sub-policy returns true
- [ ] `AllPolicy(Vec<Box<dyn TriggerPolicy>>)` — fires when ALL sub-policies return true
- [ ] Both implement `TriggerPolicy` and nest arbitrarily
- [ ] `WallClockDebounce { duration }` — fires when no new boundary received for `duration` (silence = burst over)
- [ ] `BoundaryCount { count }` — fires when N boundaries are buffered
- [ ] All new types are `Send + Sync`
- [ ] Unit tests: Any fires on first match, All requires all, nesting Any(All(...)), debounce fires after silence, count fires at threshold

## Implementation Notes

### Technical Approach
- All in `continuous/trigger_policy.rs` alongside existing `Immediate` and `WallClockWindow`
- `WallClockDebounce` checks `newest_received_at.elapsed() >= duration` — needs to track last receive time, which is available from `BufferedBoundary.received_at`
- `BoundaryCount` simply checks `buffer.len() >= count`
- `AnyPolicy`/`AllPolicy` iterate sub-policies

### Dependencies
- T-0121 (TriggerPolicy trait from I-0023)

## Status Updates

- Added `AnyPolicy` and `AllPolicy` combinators implementing `TriggerPolicy`
- `AllPolicy` with empty vec returns false (safety)
- Added `BoundaryCount::new(count)` — fires when `buffer.len() >= count`
- Added `WallClockDebounce::new(duration)` — fires when newest `received_at` is older than `duration`
- 12 new tests: count (at/below/above threshold), debounce (silence/burst/empty), any (match/no match), all (all/one fails/empty), nested any-in-all
- 18 total trigger policy tests passing
