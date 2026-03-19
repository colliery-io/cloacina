---
id: wire-python-package-registration
level: task
title: "Wire Python package registration into server and daemon — register_workflow rejects Python .cloacina"
short_code: "CLOACI-T-0215"
created_at: 2026-03-18T14:54:06.339015+00:00
updated_at: 2026-03-18T15:06:33.355869+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/active"


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

*To be added during implementation*
