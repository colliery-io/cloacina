---
id: t1-unify-runtime-registries-cg
level: task
title: "T1: Unify Runtime registries — CG + stream backend fields, register/unregister for all 5 namespaces"
short_code: "CLOACI-T-0504"
created_at: 2026-04-17T02:36:02.745997+00:00
updated_at: 2026-04-17T02:36:02.745997+00:00
parent: CLOACI-I-0096
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0096
---

# T1: Unify Runtime registries — CG + stream backend fields, register/unregister for all 5 namespaces

## Parent Initiative

CLOACI-I-0096 — Runtime Registry Unification

## Objective

Extend `Runtime` to cover computation graphs and stream backends (currently accessed via separate global statics) and add symmetric `unregister_*` methods for every namespace. Drop the `from_global()` / `use_globals` split so there is a single Runtime shape.

This task **does not** touch the macros or remove the global statics — those stay in place so existing callers keep working. T1 is purely additive: new Runtime surface area, old surface area still functions.

## Acceptance Criteria

- [ ] `Runtime` has fields for tasks, workflows, triggers, computation graphs, stream backends.
- [ ] `register_task`, `register_workflow`, `register_trigger` (already exist); add `register_computation_graph` and `register_stream_backend`.
- [ ] Matching `unregister_task(&TaskNamespace)`, `unregister_workflow(&str)`, `unregister_trigger(&str)`, `unregister_computation_graph(&str)`, `unregister_stream_backend(&str)`.
- [ ] Matching `get_*` / `has_*` / `list_*` methods for CG and stream backend.
- [ ] `Runtime::new()` returns a runtime with no global fallback; `from_global()` and `use_globals` are removed. All existing callers of `from_global()` are migrated (should be rare — most call `new()` already).
- [ ] Existing unit tests for Runtime pass; add coverage for unregister and for the CG/stream backend surfaces.

## Implementation Notes

### Surface to add

Follow the existing pattern in `crates/cloacina/src/runtime.rs`:

```rust
pub type ComputationGraphFactoryFn = /* match existing signature in computation_graph/global_registry.rs */;
pub type StreamBackendFactoryFn = /* match existing signature in computation_graph/stream_backend.rs */;

struct RuntimeInner {
    tasks: RwLock<HashMap<TaskNamespace, TaskConstructorFn>>,
    workflows: RwLock<HashMap<String, WorkflowConstructorFn>>,
    triggers: RwLock<HashMap<String, TriggerConstructorFn>>,
    computation_graphs: RwLock<HashMap<String, ComputationGraphFactoryFn>>,
    stream_backends: RwLock<HashMap<String, StreamBackendFactoryFn>>,
}
```

### Removing `from_global()`

Callers of `Runtime::from_global()` currently rely on lookup fallback into the global statics. After T1 this goes away — callers must either be migrated to explicit registration (unlikely at this stage) or temporarily call `Runtime::new_seeded_from_globals()` as a shim. **Preferred:** add a seeding helper `Runtime::seed_from_globals(&self)` that copies the current global contents into the Runtime once, so any caller that used to rely on `from_global()` gets the same snapshot semantics via two calls. This is transitional — T3 will remove the globals entirely and `seed_from_globals` with them.

### Tests

Add `crates/cloacina/src/runtime.rs` unit tests for:
- Register then unregister each namespace — `get_*` returns None after.
- Unregister missing key is a no-op.
- Multiple runtimes hold independent entries (parallel-safe).

## Status Updates

*To be added during implementation*
