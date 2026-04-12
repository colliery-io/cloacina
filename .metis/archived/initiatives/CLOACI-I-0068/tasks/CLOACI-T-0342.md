---
id: t1-python-bindings-test-suite-pyo3
level: task
title: "T1: Python bindings test suite (PyO3 runtime tests)"
short_code: "CLOACI-T-0342"
created_at: 2026-04-03T13:09:20.014898+00:00
updated_at: 2026-04-03T13:09:20.014898+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0068
---

# T1: Python bindings test suite (PyO3 runtime tests)

## Parent Initiative
[[CLOACI-I-0068]] — Tier 1 (biggest impact, ~2,565 missed lines)

## Objective
Add PyO3 runtime tests for the Python binding layer. Currently 0% coverage on bindings/runner.rs (1,626 lines), bindings/context.rs (242), bindings/admin.rs (106), bindings/retry.rs (161), python/workflow.rs (219), python/context.rs (141), python/namespace.rs (70).

## Acceptance Criteria

## Acceptance Criteria
- [ ] bindings/runner.rs: test PyRunner construction, execute, execute_async, get_execution_status, shutdown
- [ ] bindings/context.rs: test PyContext get/set/remove/keys/contains, JSON serialization
- [ ] bindings/admin.rs: test PyDatabaseAdmin create_tenant/remove_tenant/list_tenants
- [ ] bindings/retry.rs: test PyRetryPolicy construction, default values, custom values
- [ ] python/workflow.rs: test WorkflowBuilder context manager (__enter__/__exit__), task registration
- [ ] python/context.rs: test PythonContext wrapper get/set/remove/iter
- [ ] python/namespace.rs: test TaskNamespace Python conversions
- [ ] All tests use pyo3::prepare_freethreaded_python() + Python::with_gil pattern
- [ ] Coverage of target files moves from 0% to >50%

## Source Files
- crates/cloacina/src/python/bindings/runner.rs (1,626 missed)
- crates/cloacina/src/python/bindings/context.rs (242 missed)
- crates/cloacina/src/python/bindings/admin.rs (106 missed)
- crates/cloacina/src/python/bindings/retry.rs (161 missed)
- crates/cloacina/src/python/workflow.rs (219 missed)
- crates/cloacina/src/python/context.rs (141 missed)
- crates/cloacina/src/python/namespace.rs (70 missed)

## Implementation Notes
Use the existing pattern from `python/mod.rs` tests — `pyo3::prepare_freethreaded_python()` then `Python::with_gil`. Some bindings need a real DB (runner, admin) — use TestFixture. Others are pure Python/Rust boundary tests.

## Status Updates
*To be added during implementation*
