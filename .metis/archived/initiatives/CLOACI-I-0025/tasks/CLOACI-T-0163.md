---
id: fix-wallclockdebounce-silent-52
level: task
title: "Fix WallClockDebounce silent 52-week fallback and add error logging"
short_code: "CLOACI-T-0163"
created_at: 2026-03-15T18:24:45.242509+00:00
updated_at: 2026-03-15T19:40:02.378762+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Fix WallClockDebounce silent 52-week fallback and add error logging

**Priority: P2 — MEDIUM**
**Parent**: [[CLOACI-I-0025]]

## Objective

`trigger_policy.rs:158-165`: `WallClockDebounce` uses `chrono::Duration::from_std()` to convert `std::time::Duration`. If the conversion fails (unlikely but possible with extreme values), it silently falls back to a 52-week duration via `.unwrap_or(chrono::Duration::weeks(52))`. No log, no error. The user configures a 5-second debounce; it becomes a year.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Duration::from_std()` failure logs at `error!` level with the original duration value
- [ ] Fallback behavior is documented or replaced with an explicit error return
- [ ] Consider: change `should_fire()` return type to `Result<bool, PolicyError>` to propagate conversion failures, or validate the duration at construction time
- [ ] If validating at construction: `WallClockDebounce::new(duration)` returns `Result` and rejects durations that can't be converted
- [ ] Unit test: extreme duration values are either rejected at construction or logged at runtime

## Implementation Notes

- Best approach: validate the duration conversion in `WallClockDebounce::new()` at construction time, failing fast instead of failing silently at runtime
- If construction-time validation is chosen, `should_fire()` can use the pre-converted `chrono::Duration` directly, eliminating the runtime conversion entirely
- Also consider using `tokio::time::Instant` instead of `chrono::Utc::now()` for debounce timing to avoid wall-clock skew from NTP adjustments

## Status Updates

### 2026-03-15 — Completed
- Replaced `pub duration: Duration` field with `chrono_duration: chrono::Duration` — pre-converted at construction time
- Added `try_new(duration) -> Result<Self, String>` that validates conversion and fails fast
- `new(duration)` panics on invalid duration (convenience for tests)
- `should_fire()` no longer does runtime conversion — directly compares pre-converted chrono::Duration
- Eliminated the silent 52-week fallback entirely
- Removed `use tracing::error` (no longer needed — validation happens at construction)
- All 412 unit tests pass
