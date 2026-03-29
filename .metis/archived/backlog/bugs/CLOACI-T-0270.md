---
id: fix-packaged-workflow-cdylib
level: task
title: "Fix packaged workflow cdylib runtime + validation + demo freshness"
short_code: "CLOACI-T-0270"
created_at: 2026-03-27T12:50:13.255057+00:00
updated_at: 2026-03-27T12:50:13.255057+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Fix packaged workflow cdylib runtime + validation + demo freshness

## Objective

Three related fixes discovered during I-0050 investigation, all needed to make the registry-execution demo work end-to-end with packaged workflows.

## Priority
- [x] P1 - High (packaged workflows are a core feature)

## Three Fixes Required

### 1. FFI function: OnceLock<Runtime> for cdylib tokio isolation

**Problem:** cdylib packages loaded via dlopen have isolated TLS from the host binary. The FFI function `cloacina_execute_task` uses `futures::executor::block_on` which cannot drive tokio futures (sleep, spawn, fs, net).

**Root cause:** RFC 1510 — Rust cdylibs statically link all deps. The host's tokio runtime context is invisible to the cdylib's copy of tokio. Documented in tokio issues #1964, #4102, #4835, #6927.

**Fix (proven working):** Replace `futures::executor::block_on` with `OnceLock<Runtime>` + `tokio::runtime::Runtime::new()` in the `#[packaged_workflow]` macro-generated FFI function. The cdylib creates its own dedicated runtime with `enable_all()`.

**Code changes:**
- `crates/cloacina-macros/src/packaged_workflow.rs` — FFI function uses `OnceLock<Runtime>.get_or_init()` + `rt.block_on()`
- `crates/cloacina-macros/src/packaged_workflow.rs` — Wrap FFI function body in `catch_unwind` to capture panics at `extern "C"` boundary
- `crates/cloacina-workflow/Cargo.toml` — Add `tokio` as optional dep gated behind `macros` feature
- `crates/cloacina-workflow/src/lib.rs` — Add `__private` module re-exporting tokio

### 2. Validation: accept cloacina-macros + cloacina-workflow as valid deps

**Problem:** `validate_cloacina_compatibility()` requires `cloacina` as a dependency, but packaged workflows intentionally only depend on the lightweight `cloacina-macros` + `cloacina-workflow`.

**Fix:** Accept either `cloacina` OR (`cloacina-macros` + `cloacina-workflow`) as valid.

**Code change:** `crates/cloacina/src/packaging/validation.rs`

### 3. Demo: fresh database on each run

**Problem:** `registry-execution` demo uses `/tmp/cloacina_debug.db` which persists across runs. Stale pipeline executions from previous runs cause the scheduler to dispatch tasks before the reconciler finishes loading, leading to "Task not found" errors and empty context propagation.

**Fix:** Delete the database file at demo startup.

**Code change:** `examples/features/registry-execution/src/main.rs` — `std::fs::remove_file(db_path)` before creating DB

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[packaged_workflow]` FFI function uses `OnceLock<Runtime>` with `enable_all()`
- [ ] FFI function wrapped in `catch_unwind` for panic safety at `extern "C"` boundary
- [ ] `tokio::time::sleep` works inside cdylib task code (proven: collect_data with sleep succeeds)
- [ ] `validate_cloacina_compatibility` accepts `cloacina-macros` + `cloacina-workflow`
- [ ] `registry-execution` demo deletes stale DB at startup
- [ ] `angreal demos registry-execution` passes: 3 completed, 0 failed, 0 skipped
- [ ] `angreal cloacina all` passes with no regressions

## Proven Working Code (from investigation)

All three fixes were tested together and the demo produced:
```
🔍 Collecting data...
✅ Collected 1000 records
⚙️  Processing data...
✅ Processed 950 valid records
📊 Generating report...
✅ Report generated successfully
Pipeline completed: 3 completed, 0 failed, 0 skipped
```

The code changes exist in the working tree but were NOT committed. They need to be applied on main after the PyO3-in-core commits.

## Status Updates

*To be added during implementation*
