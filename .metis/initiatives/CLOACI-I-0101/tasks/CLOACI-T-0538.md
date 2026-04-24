---
id: t-01a-cg-macro-split-internals-new
level: task
title: "T-01a: CG macro split internals (new declaration + type binding + tests)"
short_code: "CLOACI-T-0538"
created_at: 2026-04-24T15:08:02.717131+00:00
updated_at: 2026-04-24T15:08:02.717131+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-01a: CG macro split internals (new declaration + type binding + tests)

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Land the `#[computation_graph]` macro changes that enable the split declaration model from CLOACI-S-0011 and I-0101, without touching any in-tree caller yet. The bundled form keeps working through this task so CI stays green; the new form is validated end-to-end against the existing runtime via added tests. No external behavior changes; no migration; no removal of the old path.

## Acceptance Criteria

- [ ] `#[computation_graph]` accepts a new `trigger = reactor("name")` clause and emits a declaration that references the reactor by name.
- [ ] `#[computation_graph]` accepts no trigger clause at all (trigger-less declaration compiles and is registered).
- [ ] Compile-time type binding: when `trigger = reactor("name")` is present, macro expansion produces a type assertion that the reactor's firing output matches the graph's `entry_type`. Mismatch is a compile error with a readable message.
- [ ] The bundled form (`#[computation_graph]` with reactor + accumulators inside) continues to compile and run — it should emit the same runtime artifacts it does today. Migration of in-tree callers happens in T-01b.
- [ ] Unit tests: macro expansion for each of the three forms (bundled — existing; split with trigger — new; trigger-less — new).
- [ ] Integration tests: standalone CG using the new `trigger = reactor(...)` form fires via reactor end-to-end; trigger-less CG compiles and is present in the graph registry (even if nothing invokes it yet).
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal cloacina unit` green.
- [ ] `angreal cloacina integration --backend sqlite` green end-to-end (full suite, not just new tests).

## Implementation Notes

### Technical Approach

1. Extend the `#[computation_graph]` macro parser in `crates/cloacina-macros` to accept:
   - `trigger = reactor("name")` as an optional top-level clause.
   - A form with no reactor/accumulators/criteria clauses at all (trigger-less).
2. Update the macro's internal IR (`computation_graph::graph_ir`) to carry `Option<TriggerBinding>`. When the bundled form is used, the bundled inline reactor translates to a synthesized `trigger` binding + a separately emitted reactor declaration. This is the narrow "temporary compat path" mentioned in the initiative — it keeps the old form working by desugaring it into the new form, not by shipping two code paths.
3. Emit the type-binding assertion with `const _: () = { ... assert_type_eq!(...) };` at expansion so mismatches surface as compile errors.
4. Extend the runtime graph registry to accept a trigger-less graph: the graph's compiled function is still registered by name, but it is not bound to any reactor subscription — that binding happens later when a subscriber (a reactor or workflow task) references it.
5. Keep `ComputationGraphScheduler::load_graph` compatible with both old-style and new-style declarations (both desugar to the same internal representation by this point).

### Key Files

- `crates/cloacina-macros/src/computation_graph/parser.rs` — add new clauses.
- `crates/cloacina-macros/src/computation_graph/graph_ir.rs` — IR updates.
- `crates/cloacina-macros/src/computation_graph/codegen.rs` — emission changes.
- `crates/cloacina/src/computation_graph/scheduler.rs` — registry additions for trigger-less graphs.
- `crates/cloacina-macros/tests/` + `crates/cloacina/tests/integration/computation_graph.rs` — new unit + integration tests.

### Dependencies

- None. This is the foundation task on the I-0101 branch.

### Risk Considerations

- Macro IR changes ripple through downstream codegen. Keep the IR evolution minimal — additive fields with `Option<>`, no renames of existing ones. Removing old fields happens in T-01b.
- Type binding via `assert_type_eq!` requires the reactor's firing-output type to be visible at the graph's macro-expansion point. If users put the reactor and graph in different modules without a shared import, the assertion may fail to compile even when types are correct. Document the import pattern in the error message.
- Keep the bundled-form desugar surgical — it should produce byte-identical runtime artifacts to today's bundled emission. Snapshot-test the macro output if practical.

## Status Updates

*To be added during implementation.*
