---
id: t-01b-migrate-rust-callers-remove
level: task
title: "T-01b: Migrate Rust callers + remove bundled CG form"
short_code: "CLOACI-T-0539"
created_at: 2026-04-24T15:08:10.182883+00:00
updated_at: 2026-04-25T15:02:30.939823+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

### 2026-04-24 — post-migration example verification + macro FFI scoping fix

Migration commit `689b688` landed earlier. Verified the migrated examples build and run end-to-end before doing the bundled-form removal.

**Bug found:** `examples/features/computation-graphs/packaged-graph` (only crate that enables `feature = "packaged"`) failed to compile:

```
error[E0425]: cannot find type `PackagedMarketMakerReactor` in this scope
  --> src/lib.rs:71:23
   |
71 |     trigger = reactor(PackagedMarketMakerReactor),
```

Root cause in `crates/cloacina-macros/src/computation_graph/codegen.rs`: the `ByReactor` codegen branch references the user-supplied `#type_path` directly inside the generated `pub mod _ffi { … }` block (for `ffi_accumulator_entries_expr` / `ffi_reaction_mode_expr`). From inside that nested module, a bare ident like `PackagedMarketMakerReactor` doesn't resolve. Tutorials 07–10 didn't catch this because they don't enable the `packaged` feature, so `_ffi` is `#[cfg]`-gated out.

**Fix:** emit a private type alias at the outer scope where the user's path resolves correctly, and reference it via `super::Alias` from inside `_ffi`:

```rust
let alias_ident = format_ident!("__CGTriggerReactor_{}", mod_name);
let trigger_alias = quote! {
    #[doc(hidden)] #[allow(non_camel_case_types)]
    type #alias_ident = #type_path;
};
// emitted alongside __cloacina_check_reactor_binding_<mod> via the
// type_binding_check token tree (already at outer scope).

let ffi_accs = quote! {
    <super::#alias_ident as #cg_path::Reactor>::ACCUMULATORS …
};
let ffi_mode = quote! {
    <super::#alias_ident as #cg_path::Reactor>::REACTION_MODE …
};
```

**Verification after fix:**
- `cargo check --workspace --all-features` — green (only pre-existing warnings).
- `cargo build` on `examples/features/computation-graphs/packaged-graph` (with `feature = "packaged"`) — green.
- `cargo build --bins` on `examples/performance/computation-graph` (main + cg-bench) — green.
- `angreal demos tutorials rust 07` — graph fires, mid-price + spread output prints correctly.
- `angreal demos tutorials rust 08` — accumulators + reactor pipeline, 3 fires across 3 events as expected.
- `angreal demos tutorials rust 09` — full reactive pipeline, 4 fires, TRADE/WAIT signals as designed.
- `angreal demos tutorials rust 10` — routing demo, 7 fires across enum-dispatch outputs.

Bundled-form parser/codegen still in place (intentional). Macro fix is uncommitted on top of `689b688`. Next: commit the fix, then proceed to step 2 of the kickoff plan (delete bundled-form parse/IR/desugar + emit compile-error diagnostic on `react = ...`).

### 2026-04-24 — bundled-form removal landed + full test suites green

Step 2 of the kickoff plan. Commit `b2b7b1f` removes the bundled-form path end-to-end:

- **Macro (`cloacina-macros`)**:
  - Parser: dropped `react = when_any|when_all(...)` clause, `ReactionCriteria`/`ReactionMode` types, conflict detection. The `react` ident is still recognized so a hard migration error fires pointing at CLOACI-I-0101 (instead of a generic "unknown field").
  - IR/codegen: dropped `TriggerSpec::Bundled`, the synthesized `__Reactor_<graph>` struct + inventory entry, and the macro-time bundled-form subset check (split form's const-eval check still emitted).
  - `#[reactor]` attribute: dropped the reserved-prefix guard on `name = "..."` (no synthesized name to defend against).
- **Runtime (`cloacina` + `cloacina-computation-graph`)**:
  - `Runtime::user_reactor_names()` removed — no `__Reactor_*` producer remains, so the dunder filter has no purpose. `reactor_names()` is the single accessor.
  - Doc comments on `ReactorRegistration`, `ReactorEntry`, and the `Reactor` trait section dropped bundled-form references.
- **Tests**:
  - Macro tests: dropped bundled parse coverage, react/trigger conflict, unknown-reaction-mode, duplicate-react, reserved-prefix. Added `test_error_react_form_removed` (asserts diagnostic + I-0101 reference) and `test_error_duplicate_trigger`.
  - Integration `test_cloaci_t_0538_runtime_reactor_registry_shape` simplified to drop the dunder-filter assertion.
- **Net**: 71 insertions, 333 deletions across 8 files.

Compile-error diagnostic on the old form (live):
```
the bundled `#[computation_graph(react = ...)]` form has been removed.
Declare a standalone `#[reactor]` and reference it via
`#[computation_graph(trigger = reactor(MyReactor), ...)]` —
see initiative CLOACI-I-0101 for migration guidance.
```

**Acceptance criteria — full run results:**

- [x] Every in-tree CG declaration uses split form (689b688).
- [x] Bundled-form desugar removed from `cloacina-macros` (b2b7b1f).
- [x] Old form produces a compile error pointing at I-0101 docs (verified by `test_error_react_form_removed`).
- [x] `cargo check --workspace --all-features` — green (only pre-existing warnings).
- [x] `angreal test unit` — **32 + 701 passed, 0 failed, 1 ignored**.
- [x] `angreal test macros` — all macro validation scenarios passed (4/4 expected-failure builds + macro lib tests).
- [x] `angreal test integration --backend sqlite` — Rust 6 passed (282 backend-filtered out), Python sqlite 27/27, "All integration tests passed!".
- [x] `angreal test integration --backend postgres` — **Rust 276 passed, 0 failed, 6 ignored, 6 filtered**; Python postgres 27/27; "All integration tests passed!".
- [x] No new cargo warnings introduced.

Three commits on the branch land T-0539:
- `689b688` — caller migration to split form.
- `31a8ca3` — macro fix: split-form trigger reactor scoping inside packaged FFI module (uncovered by `examples/features/computation-graphs/packaged-graph`).
- `b2b7b1f` — bundled-form removal + migration diagnostic.

Ready for phase transition to completed.

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
