---
id: t6-reconciler-refactor-load-from
level: task
title: "T6: Reconciler refactor — load from compiled_data, retire inline cargo build"
short_code: "CLOACI-T-0524"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-18T01:50:00+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0523]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T6: Reconciler refactor — load from compiled_data, retire inline cargo build

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Rip the inline `cargo build` out of the reconciler. Runtime instances (server + daemon) now load `compiled_data` bytes directly from the DB. The Rust toolchain is no longer a runtime dependency.

## Acceptance Criteria

- [ ] `list_workflows` (and related DAL read methods) filter to `build_status = 'success'`. `pending` / `building` / `failed` rows are invisible to the reconciler.
- [ ] `registry::reconciler::loading::load_package` is rewritten:
  1. Fetch `compiled_data` from the workflow_packages row (no `workflow_registry.data` fetch needed for the compiled blob).
  2. For Rust / mixed packages: load the cdylib via fidius FFI from `compiled_data` bytes.
  3. For pure-Python packages: read the manifest + dispatch to the existing Python task loader path; no FFI.
  4. No `cargo` subprocess anywhere in this path.
- [ ] The `cargo build` invocation that lives in `compile_source_package` (or equivalent) is deleted; any helper modules that only existed to support in-reconciler compilation are removed or pared down.
- [ ] `cloacina` runtime drops its `cargo`-shell dep path: no `rustup`, no `cargo` required to run the server or daemon.
- [ ] `cloacinactl package inspect <ID>` surfaces `build_status` + `build_error` (feature add on the existing handler).
- [ ] Integration test coverage:
  - Package uploaded → compiler (from T2/T3 + stubbed in tests or real) marks `success` → reconciler picks it up → tasks execute.
  - Package stuck in `pending` → reconciler does NOT attempt to load, does NOT error loudly. Warn-level log only.
  - Package in `failed` → same as pending: skipped, operator sees via `package inspect`.

## Implementation Notes

### Ordering relative to T5

T5 already inserts with `build_status` populated. By the time T6 lands, every path that creates a row writes a valid status; this task just makes the reconciler honor it.

### `cargo build` removal

The big win here is deleting `reconciler/extraction.rs::compile_source_package` (and its friends). That path was specific to the inline-build model.

### Existing tests

Several integration tests in `crates/cloacina/tests/integration/` currently rely on the reconciler compiling packages (e.g. the `test_registry_dynamic_loading` suite). They need to either:
- Pre-populate `compiled_data` in the DB before the reconciler tick, or
- Run a compiler instance alongside the server fixture (preferred; matches production).

Update the shared `server-soak` / CLI-e2e fixtures to also spin up `cloacina-compiler` so integration tests continue to pass end-to-end.

### Server vs daemon

Both use the same reconciler. No mode-specific logic here — `compiled_data` is a column either backend reads.

## Status Updates

*To be added during implementation*
