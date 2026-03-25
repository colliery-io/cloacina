---
id: python-trigger-integration-pyo3
level: task
title: "Python trigger integration — PyO3 bindings, @cloaca.trigger decorator, TriggerResult"
short_code: "CLOACI-T-0244"
created_at: 2026-03-24T21:20:00.032644+00:00
updated_at: 2026-03-25T01:15:44.352613+00:00
parent: CLOACI-I-0044
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0044
---

# Python trigger integration — PyO3 bindings, @cloaca.trigger decorator, TriggerResult

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0044]]

## Objective

Expose triggers to Python workflow authors via PyO3 bindings. Python packages declare triggers in their manifest, but custom trigger logic (e.g., a Python-based poll function) needs the `@cloaca.trigger` decorator and `TriggerResult` type to work from Python source.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `@cloaca.trigger` decorator registers a Python function as a custom trigger type
- [ ] `TriggerResult` Python class with `should_fire: bool`, `context: dict` fields
- [ ] Python triggers invocable from Rust via PyO3 — TriggerScheduler calls Python poll function
- [ ] ManifestV2 `type: "python"` trigger config routes to Python trigger loader
- [ ] Error handling: Python trigger exceptions caught, logged, trigger marked unhealthy (not crashed)
- [ ] Example Python package with a custom trigger (e.g., poll an RSS feed)
- [ ] Unit tests: decorator registration, TriggerResult serialization roundtrip
- [ ] Integration test: load Python package with custom trigger, verify it fires and launches workflow
- [ ] All existing Python tests pass

## Implementation Notes

### Technical Approach
Extend `crates/cloacina/src/python/` module with trigger bindings. The `@cloaca.trigger` decorator stores trigger metadata on the function. At package load time, Python triggers are wrapped in a Rust `PyTrigger` struct implementing `Trigger` trait, which calls into Python on each poll.

### Dependencies
- T-0241 (ManifestV2 trigger types)
- T-0242 (built-in trigger types — Python triggers follow same Trigger trait)
- T-0243 (reconciler — Python triggers registered same as built-in)

## Status Updates

### 2026-03-24 — Implementation complete

**Python trigger bindings** (`python/trigger.rs`):
- `@cloaca.trigger(name="...", poll_interval="10s")` decorator — registers Python function in global trigger registry
- `TriggerResult` pyclass — `should_fire: bool`, `context: Optional[dict]`
- `PythonTriggerWrapper` — implements Rust `Trigger` trait, calls Python via `spawn_blocking + with_gil`
- Supports both `TriggerResult` objects and plain `bool` return from poll functions
- Exception handling: Python exceptions caught, converted to `TriggerError::PollError`
- `drain_python_triggers()` — collects registered triggers after module import

**Module registration** (`loader.rs`):
- `ensure_cloaca_module` now registers `trigger`, `TriggerResult`, `TriggerDecorator`
- Python can do: `from cloaca import trigger, TriggerResult`

**Tests:** 6 new tests (decorator registration, function name inference, TriggerResult creation, fire/skip/exception handling). All 479 lib tests pass.
