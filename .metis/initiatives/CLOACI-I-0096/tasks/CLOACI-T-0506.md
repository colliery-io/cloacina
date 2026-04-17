---
id: t3-runtime-new-seeds-from
level: task
title: "T3: Runtime::new() seeds from inventory — drop global static registries"
short_code: "CLOACI-T-0506"
created_at: 2026-04-17T02:36:04.995217+00:00
updated_at: 2026-04-17T02:36:04.995217+00:00
parent: CLOACI-I-0096
blocked_by: [CLOACI-T-0505]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0096
---

# T3: Runtime::new() seeds from inventory — drop global static registries

## Parent Initiative

CLOACI-I-0096 — Runtime Registry Unification

## Objective

Make `Runtime::new()` iterate the inventory entries from T2 and populate itself. This makes Runtime the single source of truth for registry state. Once complete, the global static registries (`global_task_registry()`, `global_workflow_registry()`, `global_trigger_registry()`, `computation_graph/global_registry.rs`, stream backend globals) can go away in T5.

This is where the breaking change lands: previously `Runtime::new()` returned an empty runtime and `Runtime::from_global()` was the seeded one. After T3, `Runtime::new()` is inventory-seeded. Users who want a truly empty runtime (for isolated tests that want *nothing* registered) use `Runtime::empty()`.

## Acceptance Criteria

- [ ] `Runtime::new()` iterates `inventory::iter::<TaskEntry>`, `WorkflowEntry`, `TriggerEntry`, `ComputationGraphEntry`, `StreamBackendEntry` and registers all of them into the local maps.
- [ ] Add `Runtime::empty()` for test isolation — literally zero entries.
- [ ] The existing `Runtime::new()` tests that asserted "empty on construction" are updated to use `Runtime::empty()`.
- [ ] The `from_global()` helper and `use_globals` field are gone (already dropped in T1 if feasible; otherwise removed here).
- [ ] `crates/cloacina`'s own unit tests and `angreal cloacina unit` pass.
- [ ] Integration tests (`angreal cloacina integration`) pass — in particular, any test that counted on late-registered `#[ctor]` globals being visible via fallback now works because inventory is read after `main()`, which covers every test harness.

## Implementation Notes

### Seeding logic in Runtime::new

```rust
pub fn new() -> Self {
    let rt = Self::empty();
    for entry in inventory::iter::<TaskEntry> {
        rt.register_task((entry.namespace_fn)(), || (entry.constructor)());
    }
    // … same for workflow/trigger/CG/stream backend
    rt
}
```

Note that the closures are zero-capture, stored in `Box<dyn Fn>`. Since inventory entries are `&'static`, the `entry` reference is safe to move into the closure (or we can just dereference the `fn` pointer directly).

### Breaking-change behavior

Old: `Runtime::new()` returned empty. `Runtime::from_global()` saw globals.
New: `Runtime::new()` sees everything inventory captured. `Runtime::empty()` returns empty.

For the I-0095 post-mortem case where `Runtime::new()` saw 0 tasks because `#[ctor]` hadn't fired yet: under inventory, this doesn't happen because iteration is lazy. Add a regression test that asserts non-empty Runtime from a crate that uses the macros.

### Watch out

- The `#[ctor]` path from T2 is still in place. It populates the old globals. The old globals still have callers that haven't migrated. Those callers continue to work until T4 swaps them to Runtime.
- Any code that reaches directly into `computation_graph::global_registry` or similar must continue to find entries there. The macros still emit the `#[ctor]` for those globals at this point.

## Status Updates

*To be added during implementation*
