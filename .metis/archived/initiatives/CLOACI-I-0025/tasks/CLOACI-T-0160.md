---
id: promote-silent-debug-logs-to-warn
level: task
title: "Promote silent debug logs to warn for rejected boundaries and failed advances"
short_code: "CLOACI-T-0160"
created_at: 2026-03-15T18:24:41.134435+00:00
updated_at: 2026-03-15T19:31:32.635154+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Promote silent debug logs to warn for rejected boundaries and failed advances

**Priority: P2 — MEDIUM**
**Parent**: [[CLOACI-I-0025]]

## Objective

Multiple error conditions in the continuous scheduling module are logged at `debug!` level and silently discarded. In production, these are invisible — operators cannot detect data loss, validation failures, or watermark advance errors. Promote critical logs to appropriate levels.

## Specific Locations

1. **`scheduler.rs:444-451`** — Invalid boundary validation: `debug!("Rejected invalid Custom boundary...")` → should be `warn!`
2. **`scheduler.rs:393-396`** — Persistence errors: only `debug!` on failure to persist watermark state → should be `error!`
3. **`detector.rs:56-59`** — `DetectorOutput::from_context()` returns `None` on malformed JSON with no logging at all → should log `warn!`
4. **`scheduler.rs:244-246`** — If detector_output is `None`, silently skipped → should log `warn!` with task name
5. **`watermark.rs` advance errors** — Logged but not exposed; should be `warn!` at minimum
6. **`trigger_policy.rs:165`** — `Duration::from_std()` failure falls back to 52-week duration with no logging → should be `error!`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All boundary rejection events logged at `warn!` level
- [ ] All persistence failures logged at `error!` level
- [ ] All deserialization failures logged at `warn!` level with context (task name, source)
- [ ] Duration conversion fallback logged at `error!` level
- [ ] No silent data drops in the scheduler hot path
- [ ] Grep for `debug!` in scheduler.rs confirms no remaining silent error paths

## Status Updates

### 2026-03-15 — Completed
- `scheduler.rs:474`: Persistence failure promoted from `debug!` → `error!`
- `scheduler.rs:505`: Watermark advance rejection promoted from `debug!` → `warn!`
- `scheduler.rs:537`: Boundary validation rejection promoted from `debug!` → `warn!`
- `scheduler.rs:279`: Added `warn!` for `None` detector output (previously silently skipped)
- `detector.rs:56-59`: `from_context()` now logs `warn!` on deserialization failure instead of silent `None`
- `trigger_policy.rs:160`: `Duration::from_std()` failure now logs `error!` instead of silent 52-week fallback
- Added `warn` and `error` to `tracing` imports in scheduler.rs
- All 412 unit tests pass
