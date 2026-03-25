---
id: move-pyo3-bindings-into-cloacina
level: task
title: "Move PyO3 bindings into cloacina::python — native decorator machinery in core"
short_code: "CLOACI-T-0217"
created_at: 2026-03-20T01:30:10.685441+00:00
updated_at: 2026-03-20T15:07:28.256659+00:00
parent: CLOACI-I-0038
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0038
---

# Move PyO3 bindings into cloacina::python — native decorator machinery in core

## Parent Initiative

[[CLOACI-I-0038]] — Native Python Workflow Support

## Objective

Move `PythonTaskWrapper`, `PyWorkflowBuilder`, `PyContext`, and the `@task` decorator implementation from `bindings/cloaca-backend/` into `cloacina/src/python/`. After this task, the decorator machinery that registers Python tasks is compiled into the cloacina binary — no separate `cloaca` wheel needed at runtime.

`bindings/cloaca-backend` becomes a thin re-export layer for `pip install cloaca` convenience only.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `pyo3` is a direct dependency of the `cloacina` crate
- [ ] `cloacina/src/python/` module exists with: `task.rs` (PythonTaskWrapper + @task decorator), `workflow.rs` (PyWorkflowBuilder + context stack), `context.rs` (PyContext wrapper)
- [ ] `PythonTaskWrapper` implements the `Task` trait, same as today
- [ ] `PyWorkflowBuilder` context manager pushes/pops context, builds + registers workflow on `__exit__`
- [ ] `@task` decorator registers task functions into the global task registry
- [ ] `PyTaskHandle` (defer_until) works from the new location
- [ ] `bindings/cloaca-backend` re-exports from `cloacina::python` — `pip install cloaca` still works
- [ ] Existing cloaca test suite passes against the re-export layer
- [ ] `cargo test -p cloacina` includes at least one test that creates a Python workflow via `Python::with_gil`

## Implementation Notes

### What moves

| Component | From | To |
|-----------|------|----|
| `PythonTaskWrapper` | `bindings/cloaca-backend/src/task.rs` | `cloacina/src/python/task.rs` |
| `PyWorkflowBuilder` + context stack | `bindings/cloaca-backend/src/workflow.rs` | `cloacina/src/python/workflow.rs` |
| `PyContext` wrapper | `bindings/cloaca-backend/src/context.rs` | `cloacina/src/python/context.rs` |
| `@task` decorator function | `bindings/cloaca-backend/src/task.rs` | `cloacina/src/python/task.rs` |
| `PyTaskHandle` | `bindings/cloaca-backend/src/task.rs` | `cloacina/src/python/task.rs` |

### Key decisions

- No feature flag — `pyo3` is always compiled in
- `cloacina::python` is a public module so `cloaca-backend` can re-export
- The `#[pymodule]` definition stays in `cloaca-backend` (it's the Python entry point), but all implementations move to core

### Risks

- PyO3 adds to cloacina compile time and binary size for all users, even Rust-only
- Need to ensure `pyo3` version compatibility between cloacina and cloaca-backend (workspace dep)

## Status Updates

### 2026-03-20 — Migration in progress

**Exploration complete.** Full source audit of all files in bindings/cloaca-backend/src/ and crates/cloacina/src/python/.

**Key findings:**
- `crates/cloacina/src/python/` already exists with abstract `PythonTaskExecutor` trait (no PyO3)
- Need to add concrete PyO3 types alongside the abstract trait
- runner.rs and trigger.rs in cloaca-backend reference `crate::context::PyContext` — need re-import after move
- PyDefaultRunnerConfig stays in cloaca-backend (runner-specific, not core decorator machinery)
- value_objects/retry.rs stays in cloaca-backend (convenience wrappers)

**Migration plan:**
1. Add `pyo3` + `pythonize` to cloacina Cargo.toml
2. Create new files in `crates/cloacina/src/python/`: namespace.rs, workflow_context.rs, context.rs, task.rs, workflow.rs
3. Update `crates/cloacina/src/python/mod.rs` to export new modules
4. Update cloaca-backend: remove moved code, re-import from cloacina::python
5. Verify compilation

**Writing code now...**

### Migration complete

**New files in `crates/cloacina/src/python/`:**
- `namespace.rs` — PyTaskNamespace (from value_objects/namespace.rs)
- `workflow_context.rs` — PyWorkflowContext (from value_objects/context.rs)
- `context.rs` — PyContext (from context.rs, without PyDefaultRunnerConfig)
- `task.rs` — PythonTaskWrapper, TaskDecorator, PyTaskHandle, @task fn, context stack
- `workflow.rs` — PyWorkflowBuilder, PyWorkflow, register_workflow_constructor
- `mod.rs` — updated with new modules, re-exports, and integration test

**Updated `crates/cloacina/Cargo.toml`:**
- Added `pyo3 = { version = "0.25" }` and `pythonize = { version = "0.25" }`
- Added `pyo3-build-config = "0.25"` as build dependency
- New `build.rs` for Python rpath setup

**Updated `bindings/cloaca-backend/`:**
- `lib.rs` — imports from `cloacina::python::*` instead of local modules
- `context.rs` — only PyDefaultRunnerConfig remains; PyContext re-exported from core
- `value_objects/mod.rs` — re-exports PyWorkflowContext and PyTaskNamespace from core
- Removed: `task.rs`, `workflow.rs`, `value_objects/context.rs`, `value_objects/namespace.rs`
- `Cargo.toml` — extension-module moved to feature flag, added build-dependencies, dev-dependencies
- New `build.rs` for rpath

**Test results:**
- Workspace: 484 passed, 0 failed (includes new `python::tests::test_python_workflow_via_with_gil`)
- cloaca-backend: 4 passed, 0 failed (with `--no-default-features`)
- Smoke test: pre-existing failure (references `get_backend()` which never existed)
