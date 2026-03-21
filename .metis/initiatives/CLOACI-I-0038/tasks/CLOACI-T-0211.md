---
id: ffi-boundary-testing-framework-for
level: task
title: "FFI Boundary Testing Framework for Packaged Workflows"
short_code: "CLOACI-T-0211"
created_at: 2026-03-18T01:45:15.524070+00:00
updated_at: 2026-03-21T01:29:29.362338+00:00
parent: CLOACI-I-0038
blocked_by: [CLOACI-T-0215]
archived: false

tags:
  - "#task"
  - "#feature"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# FFI Boundary Testing Framework for Packaged Workflows

## Objective

Make FFI boundary validation a **defensive check in the packaging and upload pipeline**, not just an author-side test harness. When a `.cloacina` package is built or uploaded, every task should be smoke-tested through the real `cloacina_execute_task` FFI path. If a task panics (e.g. missing tokio reactor, serialisation mismatch, symbol error), the build fails or the upload is rejected with a clear error — before the package ever reaches a running scheduler.

## Background / Motivation

During the soak test build-out for the Cloacina server, the `simple-packaged-demo` example passed all its unit and integration tests but panicked at runtime with *"there is no reactor running"* when the server loaded the `.cloacina` package and executed a task through the macro-generated `cloacina_execute_task` FFI symbol. The root cause was that `futures::executor::block_on` (used in the FFI entry point) doesn't provide a tokio reactor, so any task that called `tokio::time::sleep` blew up.

The fix (switching the macro to `tokio::runtime::Handle::try_current()` / `block_in_place`) resolved this specific case, but the class of bug remains: any issue that only manifests at the FFI boundary (panics, ABI mismatches, missing symbols, architecture incompatibilities) is invisible to `cargo test` and only surfaces at runtime in the server or daemon.

The right place to catch these is **at the gate** — during `cloacinactl package build` and `POST /workflows/packages`.

### Priority
- [x] P1 - High (important for user experience)

### Business Justification
- **User Value**: Broken packages are caught at build time or upload time with a clear error, not at runtime with a cryptic panic or silent failure.
- **Business Value**: Reduces support burden, prevents production incidents, builds trust in the packaging system.
- **Effort Estimate**: M

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `PackageValidator` gains an FFI smoke test step: after symbol validation, execute each task via `cloacina_execute_task` with an empty context and verify it doesn't panic
- [ ] Panics in FFI smoke tests are caught (via subprocess or `catch_unwind`) and reported as validation errors, not process crashes
- [ ] `cloacinactl package build` runs the FFI smoke test after compiling the cdylib — build fails if any task panics at the FFI boundary
- [ ] `POST /workflows/packages` (server upload) runs the FFI smoke test as part of `register_workflow` validation — rejects the upload with 400 and a clear error message
- [ ] `cloacinactl daemon register` runs the same validation
- [ ] Clear error messages: "Task 'collect_data' panicked during FFI validation: there is no reactor running" rather than a raw stack trace
- [ ] The `simple-packaged-demo` example includes at least one test that exercises the FFI path
- [ ] Documentation explains what the FFI validation checks and how to debug failures

## Implementation Notes

### Integration points

**1. `PackageValidator` (existing, extend)**

Location: `crates/cloacina/src/registry/loader/validator/`

The validator already loads the `.so` and checks for required symbols (`cloacina_execute_task`, `cloacina_get_task_metadata`). Add a new validation step after symbol checking:

```
validate_package()
  ├── check_symbols()          ← exists
  ├── check_architecture()     ← exists
  └── smoke_test_ffi()         ← NEW: load, call each task, catch panics
```

The smoke test:
1. Extract task names from `cloacina_get_task_metadata`
2. For each task, call `cloacina_execute_task(task_name, "{}", buffer, capacity, &len)`
3. The call is expected to either succeed or return an error code (task logic may fail on empty context — that's fine). What we're catching is **panics** — the FFI boundary breaking.
4. Run in a subprocess or use `catch_unwind` at the FFI boundary to contain panics without crashing the validator process.

**2. `cloacinactl package build` (extend)**

After compiling the cdylib and before creating the `.cloacina` archive, invoke `PackageValidator::validate_package()` with the new smoke test enabled. Build fails if validation fails.

**3. Server upload handler (already wired)**

`register_workflow()` in `WorkflowRegistryImpl` already calls `PackageValidator::validate_package()`. Once the validator has the smoke test, uploads automatically get validated.

**4. Daemon register (already wired)**

`cloacinactl daemon register` calls `WorkflowRegistryImpl::register_workflow()` which calls the validator. Same path as server.

### Panic containment strategy

The FFI smoke test must not crash the host process if a task panics. Options:

**Option A: Subprocess** — fork a child process that loads the `.so` and calls `cloacina_execute_task`. If it crashes, the parent captures the exit code and stderr. Safest but heavier.

**Option B: `catch_unwind` at the FFI call site** — wrap the `cloacina_execute_task` call in `std::panic::catch_unwind`. This catches Rust panics but not `abort()` (which the current macro uses for "panic in a function that cannot unwind"). Would need the macro to use `panic` instead of `abort` in validation mode, or mark the FFI function as `extern "C-unwind"`.

**Option C: Signal handler** — install a SIGSEGV/SIGABRT handler during validation. Platform-specific and fragile.

Recommendation: **Option A (subprocess)** for robustness. The validator already has `tempfile` for staging — spawn `cloacinactl validate-ffi <path>` (a thin internal subcommand) and capture its output. If the process crashes, the parent reports "FFI validation failed: task 'X' caused process abort."

### Optional: Author-facing test harness

As a secondary deliverable, expose the smoke test logic as a library API in `cloacina-testing`:

```rust
use cloacina_testing::ffi::FfiTestHarness;

#[test]
fn test_collect_data_via_ffi() {
    let harness = FfiTestHarness::from_path("target/release/libmy_workflow.dylib")
        .expect("failed to load library");

    let result = harness.execute_task("collect_data", serde_json::json!({}))
        .expect("FFI execution failed");

    assert!(result.get("raw_data").is_some());
}
```

This is a nice-to-have on top of the pipeline validation. The pipeline check is the critical path.

### Files to modify

| File | Change |
|------|--------|
| `crates/cloacina/src/registry/loader/validator/` | Add `smoke_test_ffi()` to `PackageValidator` |
| `crates/cloacinactl/src/commands/package.rs` | Call validator after cdylib build |
| `crates/cloacina/src/registry/workflow_registry/mod.rs` | Ensure `register_workflow` uses the enhanced validator |
| `examples/features/simple-packaged/tests/` | Add FFI boundary test |
| `docs/content/tutorials/07-packaged-workflows.md` | Document FFI validation |

### Related
- Macro fix: `futures::executor::block_on` → `tokio::runtime::Handle::try_current()` (this session)
- `PackageLoader` and `PackageValidator` already do dlopen + symbol lookup
- `cloacina-testing` crate exists for no-DB workflow unit testing — could host the optional author API

## Status Updates

### 2026-03-18 — Rust FFI smoke test implemented, Python blocked

**Completed:**
- [x] `PackageValidator` gains FFI smoke test: `ffi_smoke.rs` calls `cloacina_execute_task` for each task with empty context, catches panics via `catch_unwind`
- [x] Wired into `validate_package()` flow — runs after symbol validation, before metadata validation
- [x] `POST /workflows/packages` (server) — automatically validates via `register_workflow` → `PackageValidator`
- [x] `cloacinactl daemon register` — same path
- [x] `cloacinactl package build` (Python/cloaca path) — validates after build completes
- [x] `cloacina::packaging::package_workflow` (Rust library path) — validates after archive creation
- [x] Clear error messages: "FFI validation failed: task 'X' panicked during smoke test: ..."
- [x] Explanation doc: `docs/content/explanation/packaged-workflow-validation.md`
- [x] Tested end-to-end: daemon soak (PASS), server soak (PASS), invalid package rejection (confirmed)

**Blocked → Unblocked by T-0215 + T-0216 + T-0217:**

### 2026-03-20 — Python path complete

**Python package validation is now handled end-to-end:**
- `validate_python_package()` validates manifest fields, task IDs, entry_module (T-0215)
- `import_and_register_python_workflow()` is the smoke test — imports module, fires decorators, verifies tasks registered (T-0215)
- `cloacinactl package build` validates Python packages after building (T-0216)
- `register_python_workflow()` extracts + imports + validates before storing (T-0215)

**Soak test updated for Python packages:**
- `deploy/Dockerfile.soak` now builds Python package using `cloacinactl package build` (Rust builder)
- Python `.cloacina` copied into soak runner stage
- `soak_test.py` already handles Python package upload (lines 510-527)

**All acceptance criteria met:**
- [x] FFI smoke test for Rust packages — done (prior session)
- [x] Panics caught via catch_unwind — done (prior session)
- [x] cloacinactl package build validates — done for both Rust (prior) and Python (T-0216)
- [x] Server upload validates — done via register_workflow → validate_python_package + import
- [x] Daemon register validates — same path
- [x] Clear error messages — done
- [x] simple-packaged-demo has FFI test — exists at tests/ffi_tests.rs
- [x] Documentation — done (prior session)
- [x] Python package smoke test — resolved via T-0215 import_and_register flow
- [x] Soak test includes Python package — Dockerfile updated

**Files modified (this session):**
- `deploy/Dockerfile.soak` — build + copy Python package into soak container
