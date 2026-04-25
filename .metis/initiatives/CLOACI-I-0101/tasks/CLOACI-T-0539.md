---
id: t-01b-migrate-rust-callers-remove
level: task
title: "T-01b: Migrate Rust callers + remove bundled CG form"
short_code: "CLOACI-T-0539"
created_at: 2026-04-24T15:08:10.182883+00:00
updated_at: 2026-04-24T23:15:37.830169+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-01b: Migrate Rust callers + remove bundled CG form

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Migrate every in-tree Rust user of the bundled `#[computation_graph]` form to the new split form (`#[reactor]` declaration + `#[computation_graph(trigger = reactor(TypePath))]`, where `TypePath` is the unit-struct type emitted by `#[reactor]`). Then remove the bundled-form desugar from the macro, leaving only the split form on main. After this task lands, the bundled form is unreachable in the tree and in the public macro surface.

## Acceptance Criteria

## Acceptance Criteria

- [ ] Every in-tree Rust CG declaration uses the split form: a separate `#[reactor]` declaration for each reactor, plus `#[computation_graph(trigger = reactor(TypePath))]` for each CG that subscribes to one (type-path form per T-0538's compile-time binding decision).
- [ ] Files touched by the migration (non-exhaustive; verified during the task):
  - `examples/**` — any Rust example declaring a CG
  - `crates/cloacina/tests/integration/computation_graph.rs`
  - `crates/cloacina/src/computation_graph/**` — internal examples/tests
  - Any CG declarations in `crates/cloacina-server/tests/**`
  - `crates/cloacina-computation-graph` if it carries sample declarations
- [ ] The bundled-form desugar added in T-01a is removed from `cloacina-macros`. Attempting to use the old form now produces a compile error pointing at the migration docs.
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal cloacina unit` green (both backends).
- [ ] `angreal cloacina integration --backend postgres` and `--backend sqlite` green end-to-end.
- [ ] `angreal cloacina macros` green.
- [ ] `angreal cloacina soak` quick run green (optional; include if soak isn't already exercised in integration).
- [ ] No cargo warnings introduced by the migration (unused imports, dead code, etc.).

## Implementation Notes

### Technical Approach

1. Grep for every macro invocation that uses the bundled form (`#[computation_graph(...)]` with inline reactor/accumulator clauses) and rewrite each into two declarations: `#[reactor]` for the reactor + its accumulators + its criteria, and `#[computation_graph(trigger = reactor(TypePath))]` for the graph topology.
2. Migrate the corresponding tests so they reference the new declaration surface.
3. Delete the bundled-form parse path in the macro parser, the IR shape that supports it, and the code-gen desugar that T-01a added.
4. Add a compile-error diagnostic at the macro level: if a user tries the old syntax, the error message says "the bundled `#[computation_graph]` form has been removed; declare `#[reactor]` and `#[computation_graph(trigger = reactor(TypePath))]` separately — see initiative CLOACI-I-0101."

### Key Files (expected)

- `crates/cloacina-macros/src/computation_graph/parser.rs`
- `crates/cloacina-macros/src/computation_graph/graph_ir.rs`
- `crates/cloacina-macros/src/computation_graph/codegen.rs`
- All in-tree CG declarations (see Acceptance Criteria for paths).

### Dependencies

- T-01a must have landed first. The split form must exist before we migrate to it.

### Risk Considerations

- Migration churn is mechanical but wide — touching many files. Single atomic PR; no half-migrated states.
- The macro's diagnostic quality matters here: the compile-error message is the user's first signal of the breaking change. Write it well.

## Status Updates

### 2026-04-24 — kickoff
- T-0538 committed as `5f773dc` on branch `i-0101-cg-reactor-decouple`.
- Migration surface enumerated via grep:
  - **Examples (7)**: `examples/features/computation-graphs/packaged-graph/src/lib.rs`, `examples/tutorials/computation-graphs/library/{07-computation-graph,08-accumulators,09-full-pipeline,10-routing}/src/main.rs`, `examples/performance/computation-graph/src/{main,bench}.rs`.
  - **Integration tests (1)**: `crates/cloacina/tests/integration/computation_graph.rs` (lines 48, 100, 688 use bundled form).
  - **Macro test fixtures**: `crates/cloacina-macros/src/computation_graph/parser.rs` test bodies use bundled `react = ...` literals.
- Plan:
  1. Migrate every caller to split form (`#[reactor]` + `#[computation_graph(trigger = reactor(TypePath))]`), keeping bundled-form parser path live so `cargo check` stays green throughout.
  2. Run `angreal cloacina unit` and `angreal cloacina macros` to confirm no regressions.
  3. Delete bundled-form parse/IR/desugar in macros; emit compile-error diagnostic on `react = ...` keyword pointing to I-0101.
  4. Final pass: `cargo check --workspace --all-features`, integration tests both backends.
