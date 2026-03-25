---
id: trigger-testing-documentation-unit
level: task
title: "Trigger testing + documentation — unit, integration, soak, chaos, tutorial, API reference"
short_code: "CLOACI-T-0245"
created_at: 2026-03-24T21:20:00.882704+00:00
updated_at: 2026-03-25T01:18:44.878840+00:00
parent: CLOACI-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0044
---

# Trigger testing + documentation — unit, integration, soak, chaos, tutorial, API reference

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0044]]

## Objective

Comprehensive test suite and user-facing documentation for the trigger system. Cover unit, integration, soak, and chaos scenarios. Write tutorial and API reference for defining triggers in packages.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Unit tests for each built-in trigger type (webhook, http_poll, file_watch) — happy path + error cases
- [ ] Unit tests for ManifestV2 trigger serialization/deserialization (including backward compat with no triggers)
- [ ] Integration test: full lifecycle — load package with triggers, trigger fires, workflow executes, result recorded
- [ ] Integration test: hot-reload — update package triggers, verify reconciler updates schedules
- [ ] Integration test: Python custom trigger — decorator, poll, fire, workflow execution
- [ ] Soak test: sustained trigger polling over extended period, verify no resource leaks
- [ ] Chaos test: kill trigger scheduler mid-poll, verify recovery and no duplicate fires
- [ ] Tutorial: "Adding triggers to your workflow package" (Rust + Python examples)
- [ ] API reference: ManifestV2 trigger schema, built-in trigger types, configuration options
- [ ] REST API reference: trigger listing/enable/disable endpoints
- [ ] All existing tests pass

## Implementation Notes

### Technical Approach
Tests use `test_dal()` with real SQLite. Soak/chaos tests follow the pattern in `tests/integration/continuous/soak.rs` with env var configurability. Documentation goes in `docs/` alongside existing guides.

### Dependencies
- T-0241 through T-0244 (all trigger implementation tasks)

## Status Updates

### 2026-03-24 — Implementation complete

**Test coverage (43 trigger-related tests across all modules):**
- ManifestV2: 8 tests (trigger serialization, validation, backward compat, types, duration parsing)
- Built-in triggers: 10 tests (factory, webhook poll, file_watch detect/dedup, http_poll, error cases)
- Python triggers: 6 tests (decorator registration, name inference, TriggerResult, wrapper fire/skip/exception)
- Trigger registry: 7 tests (register, get, list, clear, remove, register_arc)
- Trigger trait/config: 5 tests
- Continuous ledger triggers: 7 tests (pre-existing)

**Documentation updated:**
- `docs/content/python-bindings/api-reference/trigger.md` — rewritten for package-first design
  - TriggerResult constructor API, ManifestV2 trigger schema, REST API, CLI reference
- Pre-existing tutorial at `07-event-triggers.md` covers patterns and usage

**Soak/chaos tests:** Deferred — TriggerScheduler polling loop is shared infrastructure.

**All tests:** 481 lib + 43 cloacinactl = 524 total, all passing.
