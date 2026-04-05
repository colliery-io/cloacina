---
id: reconciler-routing-detect
level: task
title: "Reconciler routing — detect computation graph packages, route to Reactive Scheduler"
short_code: "CLOACI-T-0376"
created_at: 2026-04-05T00:33:02.321215+00:00
updated_at: 2026-04-05T01:51:46.669989+00:00
parent: CLOACI-I-0071
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0071
---

# Reconciler routing — detect computation graph packages, route to Reactive Scheduler

## Objective

Extend the reconciler to detect computation graph declarations in loaded packages and route them to the Reactive Scheduler (T-0375) instead of the Unified Scheduler. Packages use the same `.cloacina` format and `#[ctor]` registration mechanism — the reconciler inspects the global registry to determine whether a package contains workflow/trigger registrations (→ Unified Scheduler) or computation graph registrations (→ Reactive Scheduler).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[computation_graph]` macro registers the compiled graph in a global computation graph registry (parallel to workflow/task registry)
- [ ] Global registry: `ComputationGraphRegistry` with `register()` and `list()`, populated by `#[ctor]` in the macro-generated code
- [ ] Reconciler's `reconcile()` checks the computation graph registry after loading a package
- [ ] If computation graph registrations found → call `ReactiveScheduler::load_graph(declaration)`
- [ ] If workflow registrations found → existing path (Unified Scheduler)
- [ ] A package can contain both — both schedulers receive their respective declarations
- [ ] On package unload → `ReactiveScheduler::unload_graph(name)` called
- [ ] Unit test: load package with computation graph → Reactive Scheduler's `load_graph` called
- [ ] Unit test: load package with workflow only → Reactive Scheduler not called

## Implementation Notes

### Files
- `crates/cloacina-macros/src/computation_graph/codegen.rs` — add `#[ctor]` registration to generated code
- `crates/cloacina/src/computation_graph/` — new `global_registry.rs` for `ComputationGraphRegistry`
- `crates/cloacina/src/registry/reconciler/loading.rs` — add computation graph detection after package load
- `crates/cloacina/src/registry/reconciler/mod.rs` — wire ReactiveScheduler into reconciler

### Design
The `#[ctor]` function registered by the computation graph macro pushes a `ComputationGraphDeclaration` into the global registry. The reconciler reads this registry the same way it reads the task/workflow registries after loading a `.so`.

### Dependencies
T-0375 (Reactive Scheduler must exist to receive declarations)

## Status Updates

- 2026-04-04: Core building blocks complete. GlobalComputationGraphRegistry implemented with register/list/deregister. Codegen emits #[ctor::ctor] registration (cfg(not(test))) that pushes ComputationGraphRegistration into the global registry. Re-exported from cloacina:: for macro access. Reconciler can detect new graphs via list_registered_graphs(). Actual reconciler wiring (threading ReactiveScheduler through RegistryReconciler) deferred — requires deeper refactor of reconciler ownership model. The API surface is ready for integration.
