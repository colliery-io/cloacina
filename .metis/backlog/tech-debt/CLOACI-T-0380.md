---
id: post-mvp-wire-reconciler-to
level: task
title: "Post-MVP: Wire reconciler to ReactiveScheduler (load_graph on package load)"
short_code: "CLOACI-T-0380"
created_at: 2026-04-05T12:37:01.718445+00:00
updated_at: 2026-04-05T12:37:01.718445+00:00
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

# Post-MVP: Wire reconciler to ReactiveScheduler (load_graph on package load)

## Objective

Thread `ReactiveScheduler` through `RegistryReconciler` so that after a `.cloacina` package is loaded, the reconciler calls `list_registered_graphs()` to detect new computation graph registrations and routes them to `ReactiveScheduler::load_graph()`. On unload, calls `unload_graph()`.

## Context (from I-0071)

The global `ComputationGraphRegistry` and `#[ctor]` codegen are in place (T-0376). `list_registered_graphs()` returns all registered graph names. The missing piece is the reconciler calling this after package load and having a reference to the ReactiveScheduler to call `load_graph()`. This requires refactoring `RegistryReconciler` ownership to accept an optional `Arc<ReactiveScheduler>`.

### Key files
- `crates/cloacina/src/registry/reconciler/loading.rs` — add graph detection after package load
- `crates/cloacina/src/registry/reconciler/mod.rs` — add ReactiveScheduler field

## Acceptance Criteria

- [ ] `RegistryReconciler` accepts optional `Arc<ReactiveScheduler>` at construction
- [ ] After loading a package, reconciler checks `list_registered_graphs()` for new entries
- [ ] New graphs routed to `ReactiveScheduler::load_graph()`
- [ ] On package unload, `ReactiveScheduler::unload_graph()` called
- [ ] Integration test with real package loading
