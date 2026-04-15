---
id: unify-runtime-lookup-path-add-cg
level: task
title: "Unify Runtime lookup path + add CG and stream backend fields"
short_code: "CLOACI-T-0491"
created_at: 2026-04-14T12:38:37.329965+00:00
updated_at: 2026-04-14T14:47:06.570698+00:00
parent: CLOACI-I-0095
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0095
---

# Unify Runtime lookup path + add CG and stream backend fields

## Parent Initiative

[[CLOACI-I-0095]]

## Objective

Eliminate the dual-mode Runtime (`new()` vs `from_global()` with `use_globals` fallback). Replace with a single-path model: `Runtime::new()` snapshots globals, `Runtime::empty()` for isolation. Add computation graph and stream backend registry fields.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `use_globals` flag removed from `RuntimeInner`
- [ ] `Runtime::new()` snapshots all 5 global registries into local maps
- [ ] `Runtime::empty()` creates a completely empty runtime
- [ ] `from_global()` removed or deprecated (all callers migrated to `new()`)
- [ ] All `get_*()` methods have exactly one code path — local map lookup only, no fallback
- [ ] `computation_graphs` field added with `register_computation_graph()` / `get_computation_graph()`
- [ ] `stream_backends` field added with `register_stream_backend()` / `get_stream_backend()`
- [ ] Reconciler registers into Runtime directly after package loading (not relying on global fallback)
- [ ] `angreal cloacina all` passes
- [ ] `cargo check` passes for all crates

## Implementation Notes

### Key Files
- `crates/cloacina/src/runtime.rs` — main changes
- `crates/cloacina/src/runner/default_runner/mod.rs` — creates Runtime
- `crates/cloacina/src/registry/reconciler/loading.rs` — loads packages, currently registers into globals
- `crates/cloacina-computation-graph/src/lib.rs` — `GLOBAL_COMPUTATION_GRAPH_REGISTRY`, `ComputationGraphConstructor` type
- `crates/cloacina/src/computation_graph/stream_backend.rs` — `GLOBAL_REGISTRY`, `StreamBackendRegistry`

### Approach
1. Add CG + stream backend fields to `RuntimeInner`
2. Change `new()` to snapshot from all 5 globals
3. Add `empty()` constructor
4. Remove `use_globals` flag and all fallback branches in `get_*()` methods
5. Update `from_global()` callers → `new()`
6. Update reconciler to call `runtime.register_task()` / `runtime.register_workflow()` after loading packages
7. Verify with full test suite

### Dependencies
- None. T-0492 depends on this.

## Status Updates

*To be added during implementation*
