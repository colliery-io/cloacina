---
id: t5-cleanup-remove-ctor-dep-global
level: task
title: "T5: Cleanup — remove ctor dep, global_*_registry modules, document breaking change"
short_code: "CLOACI-T-0508"
created_at: 2026-04-17T02:36:07.299628+00:00
updated_at: 2026-04-17T02:36:07.299628+00:00
parent: CLOACI-I-0096
blocked_by: [CLOACI-T-0507]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0096
---

# T5: Cleanup — remove ctor dep, global_*_registry modules, document breaking change

## Parent Initiative

CLOACI-I-0096 — Runtime Registry Unification

## Objective

Delete the now-vestigial `#[ctor]` emission and the global static registries. After T4 nothing in the engine reads them, so removing them is safe. Document the Runtime lifecycle contract and the breaking change for embedded users.

## Acceptance Criteria

- [ ] `#[ctor]` emission removed from `tasks.rs`, `workflow_attr.rs`, `trigger_attr.rs`, `computation_graph/codegen.rs`. Only `inventory::submit!` remains.
- [ ] `ctor` dependency removed from `crates/cloacina/Cargo.toml` and from `cloacina-macros/Cargo.toml` if also unused.
- [ ] Global registry modules deleted:
  - `register_task_constructor` + `global_task_registry` in `crates/cloacina/src/task.rs`
  - `register_workflow_constructor` + `global_workflow_registry` in `crates/cloacina/src/workflow/mod.rs` (or registry.rs)
  - `register_trigger_constructor` + `global_trigger_registry` in `crates/cloacina/src/trigger/mod.rs`
  - `crates/cloacina/src/computation_graph/global_registry.rs`
  - Stream backend globals in `crates/cloacina/src/computation_graph/stream_backend.rs`
- [ ] Any leftover `#[serial]` attributes attached solely to protect registry-level races are removed. DB- and process-level `#[serial]` stays.
- [ ] `crates/cloacina` docs updated: module-level docs on `runtime.rs` explain the inventory-seeded lifecycle and `Runtime::empty()` for isolation.
- [ ] Changelog / release note drafted explaining the breaking change (removal of `Runtime::from_global()`, new `Runtime::empty()`, inventory-seeding behavior).
- [ ] Full test suite passes: `angreal cloacina all`, `angreal cloaca test`, `angreal cloacina ws-integration`, `angreal cloacina soak` (short run), `angreal cloacina server-soak` (short run).

## Implementation Notes

### What NOT to remove

- `TaskRegistrar` itself — still useful as a helper that knows how to iterate a loaded package's namespaces and call `runtime.unregister_*`. Drop any methods that touched the old globals and keep the rest.
- `inventory::collect!` declarations — those are live.

### Serial cleanup

Per the initiative notes, ~20 of 159 `#[serial]` annotations were registry-related. Find them with `rg '#\[serial]'` and inspect; drop the ones whose sole purpose was preventing concurrent global-registry mutation. Leave DB and process ones alone.

### Breaking change note

Short-form for CHANGELOG:

> `Runtime::from_global()` and the process-global task/workflow/trigger/
> computation-graph/stream-backend registries are removed. `Runtime::new()`
> now seeds itself from macro-generated `inventory` entries, matching the
> previous `from_global()` behavior for typical embedded apps. Tests that
> need a blank slate should use `Runtime::empty()`. This is a pre-1.0
> breaking change motivated by macOS `#[ctor]` ordering bugs that caused
> silent registration drops.

## Status Updates

### 2026-04-17: Scope split — deferred to T-0509

Attempted the full cleanup during I-0096 (see PR #70) but reverted: a
non-trivial set of integration tests reads the process-global registries
directly (`global_workflow_registry().read().contains_key(...)`,
`is_trigger_registered(...)`, etc.). Rewriting them to read via `Runtime::new()`
is straightforward but broader in scope than the initiative should hold. The
ordering-bug fix the initiative was really about is already delivered through
T-0505/6/7 (inventory-based seeding).

Remaining work tracked as **CLOACI-T-0509** in the tech-debt backlog. When
I-0096 closes, this task is effectively complete-in-spirit — the docs update
and breaking-change note are still satisfied by the PR body and the runtime
module docs. T-0509 handles the physical code cleanup.
