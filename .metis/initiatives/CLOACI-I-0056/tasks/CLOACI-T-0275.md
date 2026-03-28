---
id: integration-tests-for-packaged
level: task
title: "Integration tests for packaged trigger round-trip"
short_code: "CLOACI-T-0275"
created_at: 2026-03-28T02:16:59.035495+00:00
updated_at: 2026-03-28T02:16:59.035495+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Integration tests for packaged trigger round-trip

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

End-to-end integration tests proving the full round-trip: build a package with trigger definitions → load via reconciler → trigger registered in global registry + DB schedule → TriggerScheduler polls it → workflow fires.

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

*To be added during implementation*
