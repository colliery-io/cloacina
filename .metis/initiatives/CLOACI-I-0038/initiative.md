---
id: native-python-workflow-support
level: initiative
title: "Native Python Workflow Support — Move cloaca bindings into cloacina core"
short_code: "CLOACI-I-0038"
created_at: 2026-03-19T20:19:56.290391+00:00
updated_at: 2026-03-21T03:06:28.885239+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: native-python-workflow-support
---

# Native Python Workflow Support — Move cloaca bindings into cloacina core Initiative

## Context

Python workflow support currently requires the `cloaca` Python package to be installed separately — both at build time (`cloacinactl package build` delegates to Python via PyO3) and at runtime (the server needs `import cloaca` to work). This creates a hidden dependency that breaks in Docker, CI/CD, and any environment where the system Python doesn't have cloaca installed.

The underlying code — `PythonTaskWrapper`, `WorkflowBuilder`, `@task` decorator registration — is already Rust code (via PyO3) that registers into the same global task registry as Rust packages. It just happens to live in a separate crate (`bindings/cloaca-backend`) that gets compiled into a Python wheel.

The fix: move this code into `cloacina` core. If you can run cloacina, you can run Python workflows. No feature flags, no optional installs, no "is cloaca installed?" errors.

## Goals & Non-Goals

**Goals:**
- `pyo3` becomes a direct dependency of the `cloacina` crate (not just cloacinactl)
- `PythonTaskWrapper`, `WorkflowBuilder` context manager, and `@task` decorator logic live in `cloacina::python`
- Loading a Python `.cloacina` package = extract, add to `sys.path`, import module, decorators fire, tasks registered — all in-process
- `cloacinactl package build` for Python projects works without any Python package installation (pure Rust: TOML parsing, AST scanning, vendoring via `uv`, tar.gz creation)
- Server, daemon, and library modes all execute Python workflows natively
- `tests/fixtures/python-workflow.cloacina` committed as a test artifact, built by `cloacinactl`

**Non-Goals:**
- Removing the `cloaca` Python wheel entirely (it's still useful for `pip install cloaca` local dev convenience)
- Python async task support (future — current tasks are synchronous)
- Python continuous task support (covered by I-0026/I-0037)

## Detailed Design

### What moves from `bindings/cloaca-backend` to `cloacina` core

| Component | Current Location | New Location |
|-----------|-----------------|--------------|
| `PythonTaskWrapper` (implements `Task` trait) | `bindings/cloaca-backend/src/task.rs` | `cloacina/src/python/task.rs` |
| `PyWorkflowBuilder` + context stack | `bindings/cloaca-backend/src/workflow.rs` | `cloacina/src/python/workflow.rs` |
| `PyContext` wrapper | `bindings/cloaca-backend/src/context.rs` | `cloacina/src/python/context.rs` |
| `@task` decorator function | `bindings/cloaca-backend/src/task.rs` | `cloacina/src/python/task.rs` |
| `PyTaskHandle` (defer_until) | `bindings/cloaca-backend/src/task.rs` | `cloacina/src/python/task.rs` |

### Python package loading flow (after this initiative)

```
Upload .cloacina (Python) → register_workflow()
  → peek_manifest() detects language: "python"
  → store binary + metadata in registry
  → extract to staging dir
  → Python::with_gil:
      sys.path.insert(workflow_dir)
      sys.path.insert(vendor_dir)
      WorkflowBuilder.__enter__()  ← pushes context
      import entry_module           ← decorators fire, tasks registered
      WorkflowBuilder.__exit__()   ← builds + registers workflow
  → workflow available for execution
```

No separate `cloaca` install needed. The decorator machinery is compiled into the cloacina binary.

### Rust-native Python package builder

Replace the PyO3-delegated `cloacinactl package build` with pure Rust:

1. **TOML parsing** (`toml` crate — already a dep): read `[project]` and `[tool.cloaca]`
2. **AST task discovery** (`ruff_python_parser` or similar): scan Python source for `@task`/`@cloaca.task` decorators, extract `id`, `dependencies`, `description` from kwargs
3. **Vendoring** (subprocess to `uv`): resolve + download + extract wheels
4. **Archive creation** (`tar` + `flate2` — already deps): `manifest.json` + `workflow/` + `vendor/`

No Python runtime needed at build time.

### What happens to `bindings/cloaca-backend`

It becomes a thin re-export layer. The `cloaca` Python wheel still exists for `pip install cloaca` convenience (local dev, notebooks), but it just re-exports the PyO3 module from `cloacina::python`. The implementation lives in core.

## Alternatives Considered

**Keep cloaca as a separate package, fix the linking**: Ensure PyO3 can find the cloaca wheel at runtime via `PYTHONPATH` or bundling. Rejected — adds fragile environment management and doesn't solve the Docker/CI problem.

**Feature flag for Python support**: `cloacina` with `features = ["python"]` enables PyO3. Rejected — the user said "no feature flags, it should just work."

**Manifest-only registration (no import at registration)**: Store metadata from manifest, only import at execution time. Partially implemented in T-0215 but incomplete — the `@task` decorators need to fire for task registration to work. Moving the decorator code into core solves this properly.

## Implementation Plan

### Phase 1: Move PyO3 bindings into cloacina core
- Add `pyo3` as a dependency of `cloacina` crate
- Create `cloacina/src/python/` module with task, workflow, context code
- Move `PythonTaskWrapper`, `PyWorkflowBuilder`, `PyContext`, `@task` decorator
- Ensure existing cloaca tests still pass via re-export

### Phase 2: Wire Python package loading in register_workflow
- `register_workflow()` for Python packages: extract → `sys.path` setup → import module → decorators fire
- Reconciler `load_package()` handles Python packages the same way
- Server upload, daemon register, directory watcher all work for Python packages

### Phase 3: Rust-native package builder
- `cloacinactl package build` detects Python project (pyproject.toml with `[tool.cloaca]`)
- AST-based task discovery in Rust (Python parser crate)
- Vendoring via `uv` subprocess
- Archive creation with `ManifestV2`
- Test fixture: `tests/fixtures/python-workflow.cloacina`

### Phase 4: Testing + soak
- Soak test uploads + executes Python workflow alongside Rust workflow
- Daemon soak registers + schedules + executes Python workflow
- FFI smoke test extended for Python packages (T-0211 unblocked)
- Documentation updated

## Tasks to absorb

- **CLOACI-T-0216** (backlog) — original "rewrite cloaca build in Rust" task, now subsumed
- **CLOACI-T-0215** (active) — Python package registration, partially done, completion folded into Phase 2
- **CLOACI-T-0211** (blocked) — Python smoke test, unblocked by Phase 4
