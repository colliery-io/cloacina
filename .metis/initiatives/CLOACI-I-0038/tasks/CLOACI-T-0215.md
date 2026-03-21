---
id: wire-python-package-registration
level: task
title: "Wire Python package registration into server and daemon — register_workflow rejects Python .cloacina"
short_code: "CLOACI-T-0215"
created_at: 2026-03-18T14:54:06.339015+00:00
updated_at: 2026-03-20T15:39:43.429154+00:00
parent: CLOACI-I-0038
blocked_by: [CLOACI-T-0217]
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Wire Python package registration into server and daemon — register_workflow rejects Python .cloacina

## Objective

`register_workflow()` in `WorkflowRegistryImpl` assumes every `.cloacina` package contains a `.so`/`.dylib`. Python `.cloacina` packages (produced by `cloaca build`) contain `manifest.json` + `workflow/` + `vendor/` — no binary. Uploading a Python package to the server or registering it with the daemon fails with "No dynamic library file found in .cloacina package."

The extraction and manifest code exists (`python_loader.rs`), the task execution path exists (PyO3), but the **registration path** doesn't branch on package kind. This needs to be wired end-to-end.

### Priority
- [x] P1 - High (important for user experience)

### Impact Assessment
- **Affected Users**: Any user trying to deploy Python workflows to the server or daemon
- **Reproduction Steps**:
  1. `cd examples/features/python-workflow && cloaca build -o .`
  2. `curl -X POST http://localhost:8080/workflows/packages -F "package=@data-pipeline-example-1.0.0.cloacina"`
  3. Server returns 400: "No dynamic library file found in .cloacina package"
- **Expected**: Package registered, tasks available for execution
- **Actual**: Rejected at upload time

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `register_workflow()` detects Python vs Rust packages using `detect_package_kind()` (checks `manifest.json` language field)
- [ ] Python packages: extract via `python_loader.rs`, store binary + manifest, register tasks via PyO3 task executor
- [ ] Rust packages: existing path unchanged
- [ ] `POST /workflows/packages` accepts Python `.cloacina` packages
- [ ] `cloacinactl daemon register` accepts Python `.cloacina` packages
- [ ] Python smoke test: after extraction, import the entry module via PyO3 and verify task functions exist
- [ ] Soak test updated to upload and execute a Python workflow package alongside the Rust one
- [ ] `PackageValidator` handles Python packages (manifest validation, entry module check) instead of rejecting them as "not a dynamic library"

## Implementation Notes

### What exists

| Component | Location | Status |
|-----------|----------|--------|
| `cloaca build` CLI | `bindings/cloaca-backend/python/cloaca/cli/build.py` | Working — produces Python `.cloacina` |
| `python_loader.rs` | `registry/loader/python_loader.rs` | Working — `peek_manifest()`, `detect_package_kind()`, `extract_python_package()` |
| `ManifestV2` | `packaging/manifest_v2.rs` | Working — `PackageLanguage::Python` variant |
| `register_workflow()` | `registry/workflow_registry/mod.rs:240` | **Broken** — assumes Rust, calls `extract_so_from_cloacina()` |
| `PackageValidator` | `registry/loader/validator/` | **Broken** — checks for ELF/Mach-O/PE, rejects Python packages |
| PyO3 task execution | `cloacinactl` depends on `pyo3` | Available — used by `cloacinactl package build` |

### What needs to change

1. **`register_workflow()`**: Before extracting `.so`, call `detect_package_kind()`. Branch:
   - `PackageKind::Rust` → existing path (extract .so, validate, register FFI tasks)
   - `PackageKind::Python` → extract via `python_loader.rs`, validate manifest, store package, register Python tasks via PyO3

2. **`PackageValidator`**: Add Python validation path — if package has `manifest.json` with `language: "python"`, validate manifest fields, check workflow dir exists, optionally import entry module via PyO3

3. **Python task registration**: The reconciler needs to load Python packages and register their tasks. This requires PyO3 to import the module and discover `@cloaca.task` decorated functions at runtime.

4. **Python task execution**: `ThreadTaskExecutor` needs to handle Python tasks — call into PyO3 to execute the task function with a context. This may already be partially implemented in the cloaca backend.

### Related
- CLOACI-T-0211 — FFI smoke test (Rust side done, Python side blocked on this)
- CLOACI-I-0026 — Python/Cloaca Continuous Task Support (broader initiative)
- `bindings/cloaca-backend/` — The cloaca Python bindings crate

## Status Updates

### 2026-03-20 — Exploration complete, implementation starting

**Current state of `register_python_workflow()`** (lines 198-287 in workflow_registry/mod.rs):
- Branching logic exists: `register_workflow()` detects Python via `peek_manifest()` and routes to `register_python_workflow()`
- Stores package binary + metadata in DB
- BUT: never imports the Python module via PyO3 → `@task` decorators never fire → tasks never registered
- `loaded_packages` gets empty vec → no task namespaces tracked

**What needs to happen:**
1. After storing, extract package to staging dir via `extract_python_package()`
2. Create embedded `cloaca` Python module (so `from cloaca import task` works without pip install)
3. Push WorkflowBuilder context with proper namespace
4. Import entry module → @task decorators fire → tasks registered in global registry
5. Collect tasks, build workflow, validate, finalize, register
6. Track registered namespaces

**Key insight:** User Python code does `from cloaca import task`. In the server, there's no `cloaca` wheel installed. We need to inject a synthetic `cloaca` module into the embedded Python interpreter that re-exports the PyO3 types from `cloacina::python`.

**New file:** `crates/cloacina/src/python/loader.rs` — Python package import + task registration

**Writing code now...**

### Implementation complete

**New file: `crates/cloacina/src/python/loader.rs`**
- `ensure_cloaca_module(py)` — creates synthetic `cloaca` Python module in embedded interpreter (exports task decorator, WorkflowBuilder, Context, etc.)
- `import_and_register_python_workflow()` — complete flow: sys.path setup → push context → import module → collect tasks → build workflow → validate → register

**Updated: `crates/cloacina/src/registry/workflow_registry/mod.rs`**
- `register_python_workflow()` now:
  1. Validates manifest via `PackageValidator::validate_python_package()`
  2. Extracts package to staging dir via `extract_python_package()`
  3. Imports module via PyO3 (`import_and_register_python_workflow()`)
  4. `@task` decorators fire, tasks registered in global registry
  5. Stores package binary + metadata
  6. Tracks registered namespaces (was empty vec, now real namespaces)
  7. Keeps staging dir alive (Python needs extracted files for execution)

**Updated: `crates/cloacina/src/registry/loader/validator/mod.rs`**
- New `validate_python_package()` method: size check, manifest fields, entry_module, task ID validation
- Called before extraction/import in the Python registration path

**Test results:**
- Workspace: 484 passed, 0 failed
- cloaca-backend: 4 passed, 0 failed

**Acceptance criteria status:**
- [x] `register_workflow()` detects Python vs Rust using `detect_package_kind()` — was already implemented
- [x] Python packages: extract → import via PyO3 → register tasks — NEW
- [x] Rust packages: existing path unchanged
- [x] `POST /workflows/packages` accepts Python packages — via `register_workflow()` trait
- [x] `cloacinactl daemon register` accepts Python packages — via same trait
- [x] Python smoke test: import entry module, verify tasks registered — done in `import_and_register_python_workflow()`
- [ ] Soak test updated — deferred to T-0211 (testing task)
- [x] `PackageValidator` handles Python packages — new `validate_python_package()` method
