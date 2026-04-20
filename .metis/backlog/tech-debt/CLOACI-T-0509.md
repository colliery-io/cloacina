---
id: finish-i-0096-cleanup-remove-ctor
level: task
title: "Finish I-0096 cleanup — remove #[ctor] emission, ctor dep, and global_*_registry modules"
short_code: "CLOACI-T-0509"
created_at: 2026-04-17T13:48:55.493373+00:00
updated_at: 2026-04-20T11:07:58.262949+00:00
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

# Finish I-0096 cleanup — remove #[ctor] emission, ctor dep, and global_*_registry modules

## Objective

Completes the deferred half of **T-0508** from I-0096. The initiative delivered inventory-based `Runtime::new()` seeding, so the ordering-bug fix and the unified register/unregister surface are already on main. This task removes the remaining `#[ctor]` emission, the `ctor` workspace dependency, and the process-global static registries that back the `register_*_constructor` / `global_*_registry` APIs.

The reason this was split out of I-0096 (PR #70): a non-trivial set of integration tests reads those globals directly (`global_workflow_registry().read().contains_key(...)`, `is_trigger_registered(...)`, etc.). Every one needs to switch to reading via `Runtime::new()` before the globals can go. Plumbing a `Runtime` into the Python bindings' dynamic-registration path is also part of the cleanup.

## Type
- [x] Tech Debt

## Priority
- [x] P2 — No functional impact (ordering bug already fixed), but removes dead-code paths and a redundant registration mechanism. Do before 1.0.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[ctor::ctor]` emission removed from `workflow_attr.rs`, `trigger_attr.rs`, `computation_graph/codegen.rs`. Inventory is the only registration path.
- [ ] `ctor` dependency removed from workspace, `cloacina`, and `cloacina-macros` Cargo.toml.
- [ ] Integration tests updated to read registry state via `Runtime::new()` instead of the global registries. Specific spots:
  - `tests/integration/workflow/basic.rs:34`
  - `tests/integration/workflow/macro_test.rs:60, 102, 147`
  - `tests/integration/unified_workflow.rs:94, 112, 125, 140` (uses `is_trigger_registered`)
  - `tests/integration/trigger_packaging.rs` (6+ uses of `is_trigger_registered`)
  - `tests/integration/python_package.rs:349`
- [ ] Python bindings (`python/workflow.rs`, `python/loader.rs`, `python/bindings/runner.rs`) accept a `Runtime` handle and register dynamically into it instead of the globals.
- [ ] Delete `register_task_constructor` + `global_task_registry` (`task.rs`), `register_workflow_constructor` + `global_workflow_registry` (`workflow/registry.rs`), `register_trigger_constructor` + `global_trigger_registry` + `is_trigger_registered` + `deregister_trigger` (`trigger/registry.rs`), `computation_graph/global_registry.rs`, stream backend globals in `computation_graph/stream_backend.rs`.
- [ ] `Runtime::seed_from_globals()` is deleted (no more globals to seed from).
- [ ] Reconciler calls `seed_from_globals` on package load are replaced with explicit `runtime.register_*` using the metadata the reconciler already has.
- [ ] `angreal cloacina all`, `angreal cloaca test`, server soak short run all pass.

## Implementation Notes

### Recommended order

1. Rewrite the integration tests that read globals → use `Runtime::new()` instead. This is the tedious but mechanical step. Do it first so nothing depends on the globals for test assertions.
2. Plumb `Runtime` into Python bindings. The Python runner already has a DefaultRunner which owns a Runtime — expose it and have the Python workflow-register helpers push into it.
3. Rewrite `reconciler::load_package` to call `runtime.register_*` using the package's declared metadata instead of relying on `#[ctor]` side-effects + `seed_from_globals`. The reconciler already has the task namespaces, workflow name, trigger names, and graph name — it just has to invoke register.
4. Delete `#[ctor]` emission from macros and the `ctor` dep.
5. Delete the global registry modules and the `register_*_constructor` APIs.
6. Delete `Runtime::seed_from_globals()`.

### Reference

See PR #70 (I-0096) for context — the deferred T-0508 portion was a single commit that removed `#[ctor]` + the dep and then had to be reverted when the test sprawl became apparent. That commit is a useful starting point for the macro/Cargo.toml side of the cleanup.

## Status Updates

*To be added during implementation*
