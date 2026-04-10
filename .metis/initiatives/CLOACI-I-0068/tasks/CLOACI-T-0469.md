---
id: integration-test-embedded-cloaca
level: task
title: "Integration test: embedded cloaca module exports all CG decorators and var functions"
short_code: "CLOACI-T-0469"
created_at: 2026-04-10T12:45:34.098057+00:00
updated_at: 2026-04-10T13:06:01.631790+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# Integration test: embedded cloaca module exports all CG decorators and var functions

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0068]]

## Objective

The embedded `cloaca` Python module (registered in `loader.rs` for server-side use) was missing CG decorators and variable registry functions. The pip-installed wheel worked fine (separate `#[pymodule]`), but packaged Python CGs in the server failed with `AttributeError: module 'cloaca' has no attribute 'passthrough_accumulator'`.

**Bug:** `ensure_cloaca_module` only registered workflow-related types (`task`, `WorkflowBuilder`, `trigger`, `Context`). CG decorators and `var`/`var_or` were never added.
**Fix:** Added `passthrough_accumulator`, `stream_accumulator`, `polling_accumulator`, `batch_accumulator`, `node`, `ComputationGraphBuilder`, `var`, `var_or` to the module.

**Existing test coverage:** `test_ensure_cloaca_module_registers_in_sys_modules` — already updated with assertions for all new exports.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `cloaca.passthrough_accumulator`, `stream_accumulator`, `polling_accumulator`, `batch_accumulator` all callable
- [ ] `cloaca.node` callable inside `ComputationGraphBuilder` context
- [ ] `cloaca.var(name)` raises `KeyError` when missing, returns value when set
- [ ] `cloaca.var_or(name, default)` returns default when missing
- [ ] Unit test `test_ensure_cloaca_module_registers_in_sys_modules` passes with all attribute checks

## Files

- `crates/cloacina/src/python/loader.rs` — `ensure_cloaca_module`
- `crates/cloacina/src/python/mod.rs` — test

## Status Updates

- **2026-04-10**: Added `test_cloaca_var_and_var_or_from_python` (functional var/var_or via Python eval) and `test_cloaca_cg_decorators_are_callable` (verifies decorators are callable, not just present). All 3 cloaca tests pass.
