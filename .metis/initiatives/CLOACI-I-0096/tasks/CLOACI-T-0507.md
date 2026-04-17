---
id: t4-reconciler-and-defaultrunner
level: task
title: "T4: Reconciler and DefaultRunner wiring — use runtime.unregister_* on package unload"
short_code: "CLOACI-T-0507"
created_at: 2026-04-17T02:36:06.544064+00:00
updated_at: 2026-04-17T02:36:06.544064+00:00
parent: CLOACI-I-0096
blocked_by: [CLOACI-T-0506]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0096
---

# T4: Reconciler and DefaultRunner wiring — use runtime.unregister_* on package unload

## Parent Initiative

CLOACI-I-0096 — Runtime Registry Unification

## Objective

Migrate the reconciler, `DefaultRunner`, and any other callers of the global static registries to use Runtime instead. After T4, no code path outside the macros themselves touches `global_task_registry()` / `register_task_constructor()` / `computation_graph::global_registry::*`.

The reconciler's package-unload path is the big one: it currently does a bespoke dance through `TaskRegistrar::unregister_package_tasks`, `global_workflow_registry().remove()`, and the CG global registry. With Runtime unified, all of these become `runtime.unregister_*(key)` calls driven by the loaded package's manifest.

## Acceptance Criteria

- [ ] `DefaultRunner` owns a `Runtime` and hands it to the executor and scheduler.
- [ ] `ThreadTaskExecutor` and `TaskScheduler` take `&Runtime` for task lookup (not globals).
- [ ] `PackageLoader::load_package` registers tasks/workflows/triggers/CGs/stream backends into the Runtime it was handed.
- [ ] Reconciler's unload path calls `runtime.unregister_task` / `unregister_workflow` / `unregister_trigger` / `unregister_computation_graph` / `unregister_stream_backend` for every entry the package contributed.
- [ ] `TaskRegistrar::unregister_package_tasks` is either gone or becomes a thin wrapper around `runtime.unregister_task` loops.
- [ ] `angreal cloacina integration` passes — in particular `test_registry_dynamic_loading`, reconciler tests, and the server soak.
- [ ] `cloaca` Python bindings still load and unload packages correctly (covered by `angreal cloaca test`).

## Implementation Notes

### Runtime threading

`DefaultRunner` currently holds a bunch of services. Add `runtime: Runtime` as a field. Construct with `Runtime::new()` in the public constructors. Expose `&Runtime` to:
- Executor threads (task lookup by namespace)
- Scheduler loop (workflow / trigger lookup)
- Reactive scheduler (CG + stream backend lookup for packaged CGs)
- Reconciler (register on load, unregister on unload)

### Reconciler unload

In `registry/reconciler/loading.rs`'s unload path:

```rust
for task_ns in package.tasks() {
    runtime.unregister_task(&task_ns);
}
for wf in package.workflows() {
    runtime.unregister_workflow(wf);
}
for trig in package.triggers() {
    runtime.unregister_trigger(trig);
}
for cg in package.computation_graphs() {
    runtime.unregister_computation_graph(cg);
}
for sb in package.stream_backends() {
    runtime.unregister_stream_backend(sb);
}
```

Replaces the existing mix of `TaskRegistrar::unregister_package_tasks`, `global_workflow_registry().write().remove(...)`, and CG/stream backend global removal.

### What stays

The `#[ctor]` emission from T2 still populates the old global statics. Callers in tests/examples that directly read those globals keep working until T5 deletes them. T4 is about moving *our own* engine code off the globals, not users' code.

## Status Updates

*To be added during implementation*
