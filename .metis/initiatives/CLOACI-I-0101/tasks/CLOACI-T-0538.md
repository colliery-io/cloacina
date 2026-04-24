---
id: t-01a-cg-macro-split-internals-new
level: task
title: "T-01a: CG macro split internals (new declaration + type binding + tests)"
short_code: "CLOACI-T-0538"
created_at: 2026-04-24T15:08:02.717131+00:00
updated_at: 2026-04-24T15:16:21.525613+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0101
---

# T-01a: CG macro split internals (new declaration + type binding + tests)

## Parent Initiative

[[CLOACI-I-0101]]

## Objective

Land the `#[computation_graph]` macro changes that enable the split declaration model from CLOACI-S-0011 and I-0101, without touching any in-tree caller yet. The bundled form keeps working through this task so CI stays green; the new form is validated end-to-end against the existing runtime via added tests. No external behavior changes; no migration; no removal of the old path.

## Acceptance Criteria

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

### 2026-04-24 — Discovery + design log (before first code edit)

Starting the task. Read the existing macro and runtime code to pin down what "split the declaration" has to actually do at the IR and runtime level. Captured the plan below so the next edit session can pick up without re-reading the same files.

**Current bundling (before I-0101):**

- `#[computation_graph]` takes `react = when_any(...)` + `graph = {...}`. Applied to a module. Produces a single `ComputationGraphDeclaration` struct containing `name`, `accumulators: Vec<AccumulatorDeclaration>`, and `reactor: ReactorDeclaration { criteria, strategy, graph_fn }`.
- The graph's compiled function lives *inside* `ReactorDeclaration`. The scheduler's `load_graph` takes the whole bundle and spawns accumulators + reactor as one unit. There is no separation between "reactor runtime" and "graph runtime" today — they're one thing.
- Accumulators are declared separately via `#[stream_accumulator]` / `#[passthrough_accumulator]` / etc. The `react = when_any(alpha, beta)` clause references them by ident and tells the runtime which accumulators to spawn as the reactor's inputs.
- Key files: `crates/cloacina-macros/src/computation_graph/{parser,graph_ir,codegen}.rs`, `crates/cloacina/src/computation_graph/{scheduler,reactor}.rs`.

**Target shape (post I-0101):**

- Reactor becomes standalone: `#[reactor(name = "X", accumulators = [alpha, beta], criteria = when_any(alpha, beta), strategy = latest)]`. Produces a `ReactorDeclaration` (restructured) that knows its accumulators and firing criteria but has no graph reference.
- Graph becomes standalone: `#[computation_graph(name = "G", trigger = reactor("X"), graph = {...})]` (with trigger) or `#[computation_graph(name = "G", graph = {...})]` (trigger-less). Produces a `ComputationGraphDeclaration` that carries the compiled fn and, optionally, the name of a reactor to subscribe to.
- Runtime split: `load_reactor(ReactorDeclaration)` spawns accumulators + reactor loop, publishes firings. `load_graph(ComputationGraphDeclaration)` registers the compiled fn under its name and, if `trigger = reactor("X")` is set, subscribes to reactor X's firings.
- Firing payload: reactor publishes an `InputCache` (the same shape today's `graph_fn(&InputCache)` consumes). Type binding between reactor and graph is "graph's entry nodes must only reference accumulator names that reactor X declares." That's a compile-time check at macro expansion.

**Scope reality check for T-0538:**

This task spans more than just `#[computation_graph]` parsing — to land the new form end-to-end and keep the bundled form green through the transition, it needs:

1. New `#[reactor]` attribute macro (parser + IR + codegen — emits standalone `ReactorDeclaration`).
2. New clauses on `#[computation_graph]`: `trigger = reactor("name")` accepted instead of `react = when_any(...)`, plus the trigger-less form.
3. Desugar path inside the macro: `react = ...` bundled input expands into the same runtime artifacts the split form produces — synthesized reactor declaration under a derived name, plus a graph declaration with `trigger = reactor(derived_name)`. This is how the bundled form keeps working during T-0538 without duplicate emission paths in the backend.
4. Runtime split: `ComputationGraphDeclaration` loses `reactor: ReactorDeclaration`, gains `trigger_reactor: Option<String>` + `graph_fn: CompiledGraphFn` (moved out of the reactor decl). `ReactorDeclaration` stops carrying `graph_fn`. Scheduler gets a `load_reactor` that parallels `load_graph`.
5. Compile-time type binding between the reactor's accumulator set and the graph's entry-node signature.
6. Unit + integration tests for each new form.

Decision: keep the task scope as originally written (T-0538 lands the macro internals + runtime split, with bundled form still working). Size holds at **M** but toward the upper end — likely 4–6 days of focused work. Splitting further into T-0538a (new `#[reactor]` macro) and T-0538b (graph split) was tempting but doesn't buy much — they can't land independently because bundled-form desugar needs both sides present.

**Intended next-session starting point:**

- File: `crates/cloacina-macros/src/computation_graph/parser.rs`. Add a `trigger: Option<TriggerBinding>` field to `ParsedTopology`; parse `trigger = reactor("name")` as an alternative to `react`. Validate exclusivity (exactly one of `react` or `trigger` per declaration; the trigger-less form has neither but also requires no accumulators to be referenced in the graph's entry nodes — that check defers until type-binding).
- Then: sketch the new `#[reactor]` attribute macro in a new module `crates/cloacina-macros/src/reactor/`. Parser + IR first; codegen can be a thin wrapper that emits a static `ReactorDeclaration` initializer.
- Runtime refactor lands alongside to keep the workspace compiling.
- Last: test coverage and ensure bundled-form tests still pass via the desugar.

No code edits landed this session — only design discovery. The initiative branch `i-0101-cg-reactor-decouple` is created but carries no non-Metis commits.
