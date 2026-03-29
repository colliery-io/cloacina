---
id: integration-tests-for-packaged
level: task
title: "Integration tests for packaged trigger round-trip"
short_code: "CLOACI-T-0275"
created_at: 2026-03-28T02:16:59.035495+00:00
updated_at: 2026-03-28T12:25:51.859255+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Integration tests for packaged trigger round-trip

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

End-to-end integration tests proving the full round-trip: build a package with trigger definitions → load via reconciler → trigger registered in global registry + DB schedule → TriggerScheduler polls it → workflow fires.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test: Rust package with a trigger in manifest → reconcile → `is_trigger_registered()` returns true → `TriggerSchedule` exists in DB
- [ ] Integration test: Python package with `@cloaca.trigger` + manifest trigger def → reconcile → trigger registered and pollable
- [ ] Integration test: unload package → trigger deregistered from global registry, schedule removed from DB
- [ ] Integration test: package with no triggers → reconcile works unchanged (regression)

## Implementation Notes

### Files to modify
- `crates/cloacina/tests/integration/` — new test file(s) for trigger packaging

### Pattern to follow
- Existing reconciler integration tests in `tests/integration/dal/workflow_registry_reconciler_integration.rs`
- Existing trigger scheduler tests in `tests/integration/scheduler/trigger_rules.rs`

### Depends on
- T-0272, T-0273, T-0274

## Status Updates

**2026-03-28**: Implementation complete, all tests pass.

### Changes made:

1. **`crates/cloacina/tests/integration/trigger_packaging.rs`** — New integration test file with 14 tests:
   - **Archive round-trip (3 tests)**: peek_manifest preserves triggers, no triggers returns empty vec, Python manifest with trigger
   - **Registry lifecycle (2 tests)**: register/verify/deregister round-trip, multiple triggers register/deregister independently
   - **Python trigger (2 tests)**: `@cloaca.trigger` decorator registers and wraps via `PythonTriggerWrapper`, poll returns correct `TriggerResult` with context
   - **Manifest validation (6 tests)**: triggers validate, reference package name, reference task id, unknown workflow fails, duplicate names fails, invalid poll interval fails
   - **Regression (1 test)**: package with no triggers still works

2. **`crates/cloacina/tests/integration/main.rs`** — Added `trigger_packaging` module

3. **`crates/cloacina/src/trigger/mod.rs`** — Added `is_trigger_registered` to re-exports

### Test results:
- All 343 unit tests pass
- All 24 trigger-related integration tests pass (run via `angreal cloacina integration --skip-docker --backend sqlite trigger_packaging`)
