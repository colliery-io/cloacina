---
id: audit-t4b-reconciler-reactor
level: task
title: "Audit T4b: reconciler reactor-unload gap + method-index constant adoption"
short_code: "CLOACI-T-0564"
created_at: 2026-05-04T20:19:12.054404+00:00
updated_at: 2026-05-04T20:19:12.054404+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


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

*To be added during implementation*
