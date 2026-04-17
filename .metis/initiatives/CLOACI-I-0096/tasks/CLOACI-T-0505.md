---
id: t2-flip-macro-codegen-from-ctor-to
level: task
title: "T2: Flip macro codegen from #[ctor] to inventory::submit!"
short_code: "CLOACI-T-0505"
created_at: 2026-04-17T02:36:03.933789+00:00
updated_at: 2026-04-17T02:36:03.933789+00:00
parent: CLOACI-I-0096
blocked_by: [CLOACI-T-0504]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0096
---

# T2: Flip macro codegen from #[ctor] to inventory::submit!

## Parent Initiative

CLOACI-I-0096 — Runtime Registry Unification

## Objective

Change the macro codegen so that `#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]`, and any stream backend registration macros emit `inventory::submit!` entries instead of `#[ctor::ctor]` functions. Inventory collects entries in a linker section and exposes them via `inventory::iter::<Entry>()`, read lazily after `main()` — so the ordering bug that killed I-0095 cannot recur.

T2 adds the inventory emission **alongside** the existing `#[ctor]` emission. Nothing reads inventory yet; that's T3. Keeping both registration paths live lets us land T2 without breaking anything.

## Acceptance Criteria

- [ ] `inventory = "0.3"` (or latest) added to `crates/cloacina` as a regular dependency and re-exported for use in generated code.
- [ ] Define one `inventory::collect!` entry type per namespace (or a single enum variant) in `crates/cloacina`: `TaskEntry`, `WorkflowEntry`, `TriggerEntry`, `ComputationGraphEntry`, `StreamBackendEntry`.
- [ ] `crates/cloacina-macros/src/tasks.rs` (or `workflow_attr.rs` wherever `#[task]` lives) emits `inventory::submit! { cloacina::TaskEntry { namespace: ..., constructor: ... } }` in addition to the existing `#[ctor]`.
- [ ] Same in `workflow_attr.rs`, `trigger_attr.rs`, and `computation_graph/codegen.rs` for their respective entry types.
- [ ] Trybuild or direct compile tests confirm the generated code compiles for all four macros.
- [ ] `inventory::iter::<TaskEntry>` returns the expected entries in a smoke test (one `#[task]` in a test crate → one entry visible).

## Implementation Notes

### Entry shape

Each entry is a `Copy` + `Sync` zero-sized-like struct. Constructors are function pointers, not closures (to satisfy inventory's `'static` requirement).

```rust
pub struct TaskEntry {
    pub namespace_fn: fn() -> TaskNamespace,
    pub constructor: fn() -> Arc<dyn Task>,
}
inventory::collect!(TaskEntry);
```

Note `namespace_fn` not `namespace: TaskNamespace` — `TaskNamespace` contains `String` so it cannot be in a const/static. Defer namespace construction.

### Macros

For `#[task]`, the existing ctor body looks like:
```rust
#[ctor::ctor]
fn __register() {
    cloacina::register_task_constructor(ns, || Arc::new(MyTask));
}
```

New emission (alongside existing):
```rust
inventory::submit! {
    cloacina::TaskEntry {
        namespace_fn: || <ns construction>,
        constructor: || Arc::new(MyTask),
    }
}
```

Same pattern for the other four macros.

### Scope

Do **not** remove `#[ctor]` emission in T2. Do **not** wire Runtime to read inventory in T2. Both happen in T3. T2 is strictly additive so the full test suite keeps passing.

## Status Updates

*To be added during implementation*
