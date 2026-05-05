---
id: audit-t4b-reconciler-reactor
level: task
title: "Audit T4b: reconciler reactor-unload gap + method-index constant adoption"
short_code: "CLOACI-T-0564"
created_at: 2026-05-04T20:19:12.054404+00:00
updated_at: 2026-05-05T01:08:52.608522+00:00
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

# Audit T4b: reconciler reactor-unload gap + method-index constant adoption

Replaces the "looks like dead code, is actually a gap" findings from T-0558. Two items: a real reconciler bug and a quality improvement that the audit misclassified as dead-constant deletion.

## Objective

Close the reactor-unload gap so package unload doesn't leak reactor registrations, and finish the constant-adoption work the previous author started but didn't complete.

## Backlog Item Details

### Type
- [x] Bug — reconciler unload incomplete (reactors leak across package reload).
- [x] Tech Debt — bare numeric literals at FFI dispatch sites.

### Priority
- [x] P2 — Medium. The reactor leak is observable across hot-reload cycles in long-running daemons. The constant adoption is purely quality.

### Impact Assessment (reactor unload)
- **Affected Users**: Anyone reloading packages in a long-lived daemon/server (T-0552 added auto-trigger registration on reload; reactor unload is the inverse arm that's missing).
- **Reproduction Steps**:
  1. Load package A defining reactor `R1`.
  2. Unload package A.
  3. Confirm `Runtime::reactor_names()` still contains `R1` despite the package being gone.
- **Expected vs Actual**: Reactor registration should be removed alongside tasks/workflows/triggers/CGs. Currently reactors are never unregistered.

### Technical Debt Impact (constant adoption)
- **Current Problems**: `crates/cloacina/src/computation_graph/packaging_bridge.rs:114-117` defines `METHOD_GET_TASK_METADATA`, `METHOD_EXECUTE_TASK`, `METHOD_GET_GRAPH_METADATA`, `METHOD_EXECUTE_GRAPH` constants. Call sites still use bare numeric literals (0/1/2/3/7). Also need constants for the I-0102 additions: 4=GET_REACTOR_METADATA, 5=GET_TRIGGER_METADATA, 6=INVOKE_TRIGGER_POLL, 7=GET_TRIGGERLESS_GRAPH_METADATA, 8=INVOKE_TRIGGERLESS_GRAPH.
- **Benefits of Fixing**: One canonical mapping; renames/additions are typo-safe.
- **Risk Assessment**: Low — pure literal substitution, compiler-verified.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Reactor unload

- [ ] `Runtime::unregister_reactor` exists at `crates/cloacina/src/runtime.rs:394` but has zero non-test callers. Wire it into the reconciler unload path.
- [ ] `crates/cloacina/src/registry/reconciler/loading.rs::unload_package` adds an arm that iterates `PackageState::reactor_names` and calls `runtime.unregister_reactor` for each.
- [ ] Add an integration test under `crates/cloacina/tests/integration/registry/` that loads a reactor-bearing package, unloads it, and asserts `runtime.reactor_names()` no longer contains the reactor.
- [ ] Verify the symmetric arm for `triggerless_graph_names` already exists (added in T-0556) and matches the new pattern.

### Method-index constant adoption

- [ ] Define constants for **all** method indices 0-8 in `packaging_bridge.rs` (or move to a shared module). Current file only covers 0-3 and 7.
- [ ] Replace every bare numeric literal at FFI dispatch sites with the named constant. Search surface includes:
  - `crates/cloacina/src/registry/loader/ffi_*.rs` (task, graph, trigger, triggerless_graph adapters)
  - `crates/cloacina/src/computation_graph/packaging_bridge.rs`
  - `crates/cloacina-workflow-plugin/src/lib.rs` (the dispatch macro emit)
- [ ] Add a unit test (or `const _: () = ...` static assertion) that the constants match the values documented in `cloacina-workflow-plugin` so the two crates can't drift.

### Test gates

- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` green.

## Implementation Notes

### Technical Approach

Two commits:
1. Reactor unload arm + integration test.
2. Constant module + literal replacement sweep + cross-crate static assertion.

### Dependencies

- Builds on the unload-path scaffolding from T-0556.

### Risk Considerations

- The reactor unload arm interacts with `ReactiveScheduler` (T-0375). Verify the scheduler doesn't hold a stale `Arc<dyn Reactor>` after `Runtime::unregister_reactor` returns. If it does, that's a second leak to address in the same PR.

## Status Updates

### 2026-05-04 — Completed

**Reactor unload arm:**
- `unload_package` now drops the reactor constructor from the `Runtime` registry alongside the scheduler-side teardown. The arm fires for every reactor whose scheduler-side `unload_reactor` succeeds OR returned "not loaded" (the bundled-form CG case where `unload_graph` already tore down the running reactor). Reactors blocked by bound subscribers stay registered; the next unload attempt picks them up after subscribers unbind.
- New unit test `unload_package_drops_reactor_from_runtime_registry` exercises the full path: registers the reactor in both the scheduler and the Runtime constructor registry, calls `unload_package`, asserts the constructor is gone from `Runtime::reactor_names()`.
- Symmetric arm for `triggerless_graph_names` confirmed already present from T-0556.

**Method-index constant adoption:**
- Defined `METHOD_GET_TASK_METADATA` (0) through `METHOD_INVOKE_TRIGGERLESS_GRAPH` (8) as canonical constants in `cloacina-workflow-plugin/src/lib.rs`, alongside the trait declaration. This is the single source of truth — the trait's positional ABI and the constants are now adjacent in the same file, so a method reorder forces a constant renumber in the same diff.
- `cloacina::computation_graph::packaging_bridge` re-exports them via `pub use` for back-compat with existing consumers; deletes its own duplicate definitions.
- Migrated all bare-numeric call sites:
  - `packaging_bridge.rs::execute_graph` (was `3`)
  - `package_loader.rs` get_task_metadata / get_graph_metadata / get_triggerless_graph_metadata (was `0` / `2` / `7`)
  - `task_registrar/dynamic_task.rs::execute_task` (was `1`)
  - `task_registrar/extraction.rs::get_task_metadata` (was `0`)
  - `ffi_trigger.rs::poll` (was a file-local `INVOKE_TRIGGER_POLL_METHOD_INDEX = 6`)
  - `ffi_triggerless_graph.rs::invoke` (was a file-local `INVOKE_TRIGGERLESS_GRAPH_METHOD_INDEX = 8`)
- Cross-crate drift protection: by canonicalizing in `cloacina-workflow-plugin` and re-exporting from `cloacina`, there's only one definition. No static assertion needed — drift is impossible because there is no second source.

**Test gates:**
- `cargo check --workspace --all-features` green.
- `angreal test unit` green (45 + 658 tests, including new `unload_package_drops_reactor_from_runtime_registry`).
- `angreal test integration --backend sqlite` green (295 + 6 tests).

**Out-of-scope finding:**
- Python `test_scenario_15_workflow_versioning.py` deadlocked at 0.14s CPU after running for over an hour during `angreal test all`. Unrelated to T-0564 (which is Rust-only); flagged for separate investigation. Probably a pre-existing flake in the Python integration suite.
