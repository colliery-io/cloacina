---
id: update-stale-cloaca-python-test
level: task
title: "Update stale cloaca Python test scenarios to match post-T-0529 API"
short_code: "CLOACI-T-0532"
created_at: 2026-04-20T17:25:00+00:00
updated_at: 2026-04-21T01:23:45.933872+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Update stale cloaca Python test scenarios to match post-T-0529 API

## Objective

Bring the `tests/python/test_scenario_*.py` suite back in line with the current `cloaca` Python bindings so it runs green in CI. T-0486 (consolidate cloaca angreal namespace into cloacina) re-wired these scenarios into the active test invocation, exposing the fact that they had been silently skipped for a while and have accumulated API drift — most notably from T-0529 (isolate Python runtime into cloacina-python), which renamed/removed several public surfaces the tests still call.

### Type
- [x] Tech Debt

### Priority
- [x] P1 — High. Blocks the `Feature Build (postgres-only)` lane on every push to main; forces `--admin` merges or a skip-commit workaround.

### Technical Debt Impact
- **Current Problems**: Python scenario tests assert against APIs that no longer exist; the suite had been skipped pre-T-0486 and nobody noticed the drift.
- **Benefits of Fixing**: Restores CI signal for cloaca Python bindings; removes a class of false-positive red builds; exercises the real current API.
- **Risk Assessment**: Low. Tests are not load-bearing; the work is port-forward or delete.

## Observed failures (2026-04-20, run 24680287083, SHA 15171f1)

Failing tests and representative errors:

- `tests/python/test_scenario_01_basic_api.py`
  - `test_import_cloaca_successfully` — `assert False`
  - `test_hello_world_function` — `module 'cloaca' has no attribute 'hello_world'`
  - `test_core_classes_available` — `assert False`
  - `test_hello_class_creation` — `module 'cloaca' has no attribute 'HelloClass'`
  - `TestTaskDecorator::*` (6 tests) — `ValueError: WorkflowBuilder.__exit__ called outside a Runtime scope`
  - `TestWorkflowBuilder::*` (7 tests) — same runtime-scope error + one `'WorkflowBuilder.add_task called outside a Runtime scope'`
  - `test_workflow_validation`, `test_workflow_properties`, `test_workflow_version_consistency` — same
  - `TestDefaultRunnerConfig::test_config_property_access` — `'DefaultRunnerConfig' object has no attribute 'pipeline_timeout_seconds'. Did you mean: 'workflow_timeout_seconds'?`
  - `TestWorkflowContextManager::test_basic_workflow_context_manager` — same runtime-scope error
  - `TestWorkflowContextManager::test_register_workflow_constructor` — same, wrapped
- `tests/python/test_scenario_02_single_task_workflow_execution.py::test_task_with_context_manipulation` — runtime-scope error
- `tests/python/test_scenario_03_function_based_dag_topology.py::test_comprehensive_dag_topology_patterns` — runtime-scope error
- `tests/python/test_scenario_08_multi_task_workflow_execution.py::test_comprehensive_multi_pattern_workflow` — runtime-scope error

## Root cause (hypothesis)

- T-0529 renamed `pipeline_timeout_seconds` → `workflow_timeout_seconds` on `DefaultRunnerConfig` and tightened `WorkflowBuilder` lifecycle so it must be entered inside a Runtime scope.
- `hello_world` / `HelloClass` are removed smoke-test surfaces that no longer exist on the `cloaca` module.
- The test suite predates these changes. Under the old `.angreal/cloaca/*` harness it was either not invoked or was reached through a path that silently masked failures (confirm while working).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `Feature Build (postgres-only)` CI lane on main is green.
- [ ] Each `tests/python/test_scenario_*` file is either (a) updated to the current cloaca API, or (b) deleted if it was asserting removed smoke surfaces (`hello_world`, `HelloClass`).
- [ ] `WorkflowBuilder` / `WorkflowContextManager` tests exercise the real post-T-0529 lifecycle (entered inside a Runtime scope).
- [ ] `DefaultRunnerConfig` tests use `workflow_timeout_seconds` (and any other current field names).
- [ ] Run `angreal cloacina python-test` (or equivalent) locally and get a green run before merge.

## Implementation Notes

### Technical Approach
1. Reproduce locally: `angreal cloacina python-test`. Confirm same failures surface.
2. Walk through each failing test:
   - Delete tests asserting removed surfaces (`hello_world`, `HelloClass`).
   - For `WorkflowBuilder` / `WorkflowContextManager` tests, wrap in a Runtime scope to match T-0529 semantics.
   - For `DefaultRunnerConfig`, update field name to `workflow_timeout_seconds` and audit other field references.
3. Cross-check `crates/cloacina-python/` public surface for any other renamed symbols the scenarios still touch.

### Dependencies
- Builds on T-0486 (angreal consolidation) and T-0529 (Python runtime isolation), both merged 2026-04-19 / 2026-04-20.

### Risk Considerations
- Low. No production code changes; test-only.

## Status Updates

- 2026-04-20: Filed after main went red on push of #82.
- 2026-04-20: Reproduced locally via `angreal demos python-tutorial-01` — tutorial fails with the same `@task decorator called outside a Runtime scope` / `WorkflowBuilder.__exit__ called outside a Runtime scope` errors. This is not just stale test fixtures; the standalone cloaca Python API is fundamentally broken post-T-0529. `#[pymodule] cloaca` init never installs a ScopedRuntime and no Python-facing `Runtime` class exists, so every module-level `@cloaca.task` / `WorkflowBuilder` usage errors out. Re-scoping.

## Re-scoped plan (2026-04-20)

1. **Rust binding fix (real regression)**: in `crates/cloacina-python/src/lib.rs` `#[pymodule] cloaca`, install a default `Arc<cloacina::Runtime::empty()>` into the current thread's `CURRENT_RUNTIME` slot if none is present, and leak the `ScopedRuntime` guard so the install lives for the process lifetime. `ScopedRuntime` is thread-local, so this does not conflict with the server loader thread which installs its own scope on a different thread. Accepted limitation: user-spawned Python threads will not inherit the runtime; tasks-within-threads is out of scope.
2. **Test fixture cleanup in `tests/python/test_scenario_01_basic_api.py`**:
   - Remove `TestBasicImports::test_hello_world_function`, `test_hello_world_function`/`HelloClass` asserts in `test_core_classes_available`, and `TestHelloClass` entirely (surfaces removed upstream).
   - Replace `config.pipeline_timeout_seconds` with `config.workflow_timeout_seconds` in `test_config_property_access`.
   - Audit scenarios 02, 03, 08 for similar drift while working the fix.

Verification: `angreal demos python-tutorial-01` runs green, then `angreal cloacina integration` runs green end-to-end.

## Final changes (2026-04-20)

Landed fix:

- `crates/cloacina-python/src/lib.rs`: install a process-default `ScopedRuntime` in `#[pymodule] cloaca` init (leaked guard, thread-local so it does not conflict with the server loader thread).
- `crates/cloacina/src/runner/default_runner/config.rs`: added `runtime_arc: Option<Arc<Runtime>>` field + `DefaultRunnerBuilder::runtime_arc(Arc<Runtime>)` setter. `build()` now prefers the shared `Arc` over wrapping a by-value `Runtime` or creating a fresh inventory-seeded one. This preserves `Arc` identity so callers that hold a clone (e.g. the Python `ScopedRuntime`) see their registrations in the runner.
- `crates/cloacina-python/src/bindings/runner.rs`: `PyDefaultRunner::new` / `with_config` / `with_schema` read `current_runtime()` on the calling Python thread and pass it into the builder via `runtime_arc()`, so workflows registered by module-level `@cloaca.task` + `WorkflowBuilder` land in the runner the scheduler actually uses.
- `tests/python/test_scenario_01_basic_api.py`: removed stale `hello_world` / `HelloClass` asserts, renamed `pipeline_timeout_seconds` → `workflow_timeout_seconds`.
- `tests/python/test_scenario_28_multi_tenancy.py`: updated `pytest.raises` class + match regex to match current `RuntimeError: Failed to set up schema` surface.
- `tests/python/test_scenario_29_event_triggers.py`: dropped removed `workflow=` kwarg from `@cloaca.trigger(...)` decorator calls.
- `crates/cloacina/tests/integration/registry_workflow_registry_tests.rs::test_register_real_workflow_package`: rewrote the verification. Previously asserted via `list_workflows()` (which filters to `build_status = 'success'`); after T-0523 that filter never matches freshly-registered `queued` packages in a test that does not run the compiler-service. Now uses `inspect_package_by_id` (unfiltered) so the test verifies registration, not compilation.

Verification run (2026-04-20, local):
- `angreal demos python-tutorial-01`: green — workflow registered, executed, all three tasks completed.
- `angreal cloacina integration`: green — Rust integration suite + Python pytest scenarios (both postgres and sqlite).

### Feature Build (sqlite-only) CI fix (2026-04-20)

PR 84's initial CI surfaced another pre-existing failure on main: `Feature Build (sqlite-only)` fails with `rust-lld: error: unable to find library -lpq`. Root cause: the sqlite-only lane removes libpq from the runner to verify purity, then `angreal cloacina integration` builds the cloaca wheel via `maturin build --release` without feature overrides, so cloacina-python's defaults (`postgres,sqlite,macros`) kick in and pull in libpq.

Fix rolled into this PR:
- `.angreal/cloacina/integration.py`: forward the caller's `--features` selection into the wheel build when it differs from defaults.
- `.angreal/cloacina/python_utils.py`: `_build_and_install_cloaca_unified` now accepts `cargo_features=` and passes `--no-default-features --features <...>` to maturin when set.

Verified locally: `maturin build --release --manylinux off --no-default-features --features sqlite,macros` in `crates/cloacina-python` succeeds and the resulting wheel has no libpq runtime requirement.
