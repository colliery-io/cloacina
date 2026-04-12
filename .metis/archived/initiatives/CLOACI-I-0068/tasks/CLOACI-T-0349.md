---
id: t1-python-bindings-test-suite-pyo3
level: task
title: "T1: Python bindings test suite (PyO3 runtime tests)"
short_code: "CLOACI-T-0349"
created_at: 2026-04-03T13:19:21.975943+00:00
updated_at: 2026-04-03T16:52:25.400322+00:00
parent: CLOACI-I-0068
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


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

### 2026-04-03 — Implementation complete (51 tests, all passing)

**Tests added to 5 files:**
- `python/namespace.rs` (10 tests): constructor+getters, from_string valid/invalid, parent, is_child_of, is_sibling_of, str/repr, eq, hash, from_rust/to_rust roundtrip
- `python/context.rs` (14 tests): empty/dict construction, set/get, insert (success+dup error), update (success+missing error), remove (existing+missing), len, contains, JSON roundtrip, to_dict, repr, from_rust_context, clone
- `bindings/value_objects/retry.rs` (14 tests): default, builder defaults, builder chain, should_retry boundary, calculate_delay, repr, all BackoffStrategy variants, all RetryCondition variants, from_rust/to_rust
- `bindings/context.rs` (6 tests): PyDefaultRunnerConfig default, custom params, repr, setters, to_dict
- `python/workflow.rs` (5+2 tests): builder defaults, custom namespace, description/tag, empty build error, build with task

### 2026-04-03 — Runner and admin tests added (69 total tests now)

Added integration tests using real DB:
- `bindings/runner.rs` (10 tests): construction with SQLite + Postgres, shutdown, context manager, list_cron_schedules/list_trigger_schedules (empty), get_trigger_schedule (not found), start/stop (not implemented), ShutdownError display
- `bindings/admin.rs` (8 tests): TenantConfig construction/defaults/repr, is_postgres_url, rejects SQLite/invalid URL/missing DB name, creates with Postgres URL

### 2026-04-03 — Final push: runner.rs 0% → 62.6%

Added 12 more runner tests: register/list/get/update/delete/enable cron schedules, execute (nonexistent + registered workflow), PipelineResult (completed + failed), with_schema validation (rejects sqlite/empty/invalid chars), with_schema Postgres integration.

**Final: 97 Python module tests, runner.rs at 62.6% line coverage (was 0%).**
