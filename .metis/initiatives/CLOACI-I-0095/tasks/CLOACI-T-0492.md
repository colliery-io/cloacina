---
id: wire-reactivescheduler-and
level: task
title: "Wire ReactiveScheduler and reconciler to use Runtime for CG lookup"
short_code: "CLOACI-T-0492"
created_at: 2026-04-14T12:38:38.266032+00:00
updated_at: 2026-04-14T14:47:06.107077+00:00
parent: CLOACI-I-0095
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0095
---

# Wire ReactiveScheduler and reconciler to use Runtime for CG lookup

## Parent Initiative

[[CLOACI-I-0095]]

## Objective

Wire ReactiveScheduler's `load_graph()` to look up computation graph constructors via Runtime instead of `global_computation_graph_registry()`. Wire stream backend lookup through Runtime. Reconciler registers CG packages into Runtime after loading.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `load_graph()` uses `runtime.get_computation_graph()` instead of global registry
- [ ] Reconciler calls `runtime.register_computation_graph()` after loading CG packages
- [ ] Stream backend lookup goes through `runtime.get_stream_backend()`
- [ ] No direct calls to `global_computation_graph_registry()` or `global_stream_registry()` outside of `Runtime::new()` snapshot
- [ ] `angreal cloacina all` passes

## Implementation Notes

### Key Files
- `crates/cloacina/src/computation_graph/scheduler.rs` — ReactiveScheduler
- `crates/cloacina/src/registry/reconciler/loading.rs` — package loading
- `crates/cloacina/src/computation_graph/stream_backend.rs` — stream registry access points

### Dependencies
- T-0491 must be completed first (Runtime has CG/stream fields)

## Status Updates

*To be added during implementation*
