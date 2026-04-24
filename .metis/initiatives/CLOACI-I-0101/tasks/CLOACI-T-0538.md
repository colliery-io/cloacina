---
id: t-01a-cg-macro-split-internals-new
level: task
title: "T-01a: CG macro split internals (new declaration + type binding + tests)"
short_code: "CLOACI-T-0538"
created_at: 2026-04-24T15:08:02.717131+00:00
updated_at: 2026-04-24T23:13:00.851546+00:00
parent: CLOACI-I-0101
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


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

### 2026-04-24 — Ralph restart: scope check-in before committing (no edits landed)

Resumed Ralph on T-0538. Re-read the existing macro surface (`parser.rs`, `graph_ir.rs`, `codegen.rs`), the runtime types (`cloacina_computation_graph::ComputationGraphRegistration`, `Runtime::register_computation_graph`, `inventory_entries::ComputationGraphEntry`), and the reconciler/packaging paths that consume `accumulator_names` + `reaction_mode`. The prior session's target-shape notes hold: landing the acceptance criteria end-to-end needs a standalone `#[reactor]` declaration mechanism, a split `ComputationGraphRegistration` (trigger by name + compiled_fn), a new `load_reactor` runtime path, and a compile-time type-binding story (still design-open — there is no shared type today, only string accumulator names).

**Open architectural decisions that T-0538 forces but I-0101/S-0011 haven't settled in code:**

1. **Does T-01a introduce a `#[reactor]` attribute macro, or does the CG macro keep emitting the reactor under a derived name via bundled-form desugar?** The design log leans toward the former; the AC doesn't require it by name. Introducing `#[reactor]` adds a macro, an IR, a codegen path, and an inventory entry (`ReactorEntry`) — real scope. Without it, the new `trigger = reactor("X")` form can't meaningfully reference anything the user writes themselves (there's no syntax to declare reactor X standalone).

2. **How is compile-time type binding expressed?** AC says `assert_type_eq!(reactor firing output, graph entry_type)`. Today there is no distinct type — reactor fires `InputCache` and graph consumes `InputCache`. The binding is semantic (accumulator name set), not structural. Options: (a) emit a `const` in the reactor's expansion that enumerates accumulator names + a structural newtype, and have the graph macro reference it; (b) defer to runtime check at registration. (a) is the spirit of the AC; (b) is a shortcut.

3. **Registration shape.** Split `ComputationGraphRegistration` into `{ graph_fn, entry_accumulators: Vec<String>, trigger_reactor: Option<String> }` and add a parallel `ReactorRegistration { name, accumulator_names, reaction_mode }`? Packaging bridge + reconciler/loading both consume the current flat shape; they ripple.

4. **Does the bundled form keep `accumulator_names`/`reaction_mode` on the emitted registration (for packaging metadata) even after it desugars to trigger-by-name?** Packaging FFI depends on those fields being in `GraphPackageMetadata`. Probably yes, with the understanding that they're redundant for the new-style path.

**Scope reality for a single landing:**

The AC is tight — it asks for the macro changes, the runtime split, the type-binding assertion, *and* a green end-to-end integration test on the new form. Based on the file surface (macros: 4 files; runtime cg: 10 files; reconciler + packaging_bridge + inventory_entries consumers), landing all of it cleanly is 4–6 focused days with real risk of touching Python FFI and packaged-graph paths too. That's the right size for an initiative slice, not a single Ralph iteration.

**Proposed split (pending user approval):**

- **T-0538a (this task, renamed)** — parser + IR extensions only. `#[computation_graph]` accepts `trigger = reactor("X")` and no-trigger forms, alongside today's `react = ...`. Runtime registration stays on the current flat shape; the new forms synthesize defaults so emission stays additive. Unit-tests for all three parser paths. No runtime behavior change yet; bundled form is byte-identical.
- **T-0538b (new, follow-on)** — introduce `#[reactor]` macro + `ReactorRegistration` + `load_reactor` runtime path. Graph's `trigger = reactor("X")` binds at load time via name. First form where reactor and graph are declared separately.
- **T-0538c (new, follow-on)** — compile-time type binding (`assert_type_eq!` between the reactor's firing output const and graph entry_type) + integration test covering the new form end-to-end.

Alternative: keep T-0538 monolithic but commit to a 4–6 day landing with incremental PRs to the `i-0101-cg-reactor-decouple` branch.

**Decision needed from user before I continue:**

- Split T-0538 into a/b/c, or land monolithic?
- On the type-binding question (item 2 above): (a) structural const emitted by the reactor macro, or (b) runtime check at registration?
- Is it OK for bundled-form emission to keep carrying `accumulator_names` + `reaction_mode` redundantly (needed for packaging FFI), or do we want to move those into a parallel `ReactorRegistration` even for the bundled desugar path?

Stopping here instead of locking in a multi-day direction unilaterally. Will resume on the next Ralph iteration once the user picks a lane.

### 2026-04-24 — Direction locked; starting implementation

User decisions:
- **Single ticket**, landed across multiple commits on `i-0101-cg-reactor-decouple`.
- **Compile-time type binding** — "fail fast" system philosophy. Runtime-only check was rejected.
- **Explicit split-form** `#[reactor]` attribute macro is in scope (not just bundled desugar).
- Syntax: `#[reactor(name = "X", accumulators = [...], criteria = when_any(...))] pub struct X;` — attribute on unit struct. Graph references by type path: `trigger = reactor(X)`.
- Bundled form desugars to a synthesized struct named `__Reactor_<graphname>` (double-underscore prefix for operational filtering).

**Implementation plan (in order, each a committable milestone):**

1. `Reactor` trait + `ReactionMode` const + `ReactorRegistration` in `cloacina-computation-graph`. `ReactorEntry` inventory in `cloacina`. `Runtime::register_reactor` / `get_reactor` scaffolding. No macro yet.
2. New `#[reactor]` attribute macro (parser + codegen). Emits struct, trait impl, inventory entry. Unit tests for expansion.
3. Graph parser extended for `trigger = reactor(TypePath)` and trigger-less form. Bundled-form desugar path. `ComputationGraphRegistration` grows `entry_accumulators: Vec<String>` + `trigger_reactor: Option<String>`; keep `accumulator_names`/`reaction_mode` for packaging FFI continuity.
4. Compile-time check: graph emits a `const _: () = { ... }` block that const-evals a subset-check between graph entry accumulators and `<ReactorType as Reactor>::ACCUMULATORS`. Readable panic message on mismatch.
5. Runtime: `load_reactor` path. Graph registration with `trigger_reactor: Some(name)` binds at load. Trigger-less graphs register fn-only.
6. Integration tests: split-form graph fires end-to-end; trigger-less graph is present in the registry.
7. Workspace check + `angreal cloacina unit` + `angreal cloacina integration --backend sqlite` green.

Starting iteration 1 now.

### 2026-04-24 — Milestones 1 + 2 landed locally (not yet committed)

**M1: Reactor foundation types.**
- `cloacina-computation-graph/src/lib.rs`: added `ReactionMode` enum (`WhenAny`/`WhenAll` + `as_str`/`Display`), `Reactor` trait with `NAME`/`ACCUMULATORS`/`REACTION_MODE` consts, `ReactorRegistration { name, accumulator_names, reaction_mode }`, `ReactorConstructor` alias.
- `cloacina/src/inventory_entries.rs`: added `ReactorEntry { name, constructor }` + `inventory::collect!`.
- `cloacina/src/runtime.rs`: added `reactors: RwLock<HashMap<String, ReactorConstructor>>` to `RuntimeInner`, seeded in `seed_from_inventory`, with `register_reactor`/`unregister_reactor`/`get_reactor`/`reactor_names`/`user_reactor_names` (the last filters out `__Reactor_*` synth names).
- `cloacina/src/lib.rs`: re-exported `Reactor`, `ReactorRegistration`, `ReactorConstructor`, `ReactionMode as ComputationReactionMode`, `ReactorEntry`.

**M2: `#[reactor]` attribute macro.**
- `cloacina-macros/src/reactor_attr.rs` (new, ~460 LOC with tests): parser for `name = "..."`, `accumulators = [..]`, `criteria = when_any|when_all(..)`; validates non-empty name, rejects `__Reactor_` prefix for user-declared reactors, rejects duplicate accumulators, rejects criteria names not in the accumulators list, requires a unit struct target, rejects generics. Emits: target struct preserved, `impl cloacina_computation_graph::Reactor for <T>` with const values, `inventory::submit! { ReactorEntry { ... } }`.
- `cloacina-macros/src/lib.rs`: wired `reactor` as a new `#[proc_macro_attribute]`.
- 14 unit tests, all green.

Workspace `cargo check` green. Next: milestone 3 (graph parser extensions + desugar).

### 2026-04-24 — Milestones 3 + 4 landed locally (not yet committed)

**M3: Graph parser extensions + desugar + registration-shape change.**
- `cloacina-computation-graph/src/lib.rs`: `ComputationGraphRegistration` grew `entry_accumulators: Vec<String>` + `trigger_reactor: Option<String>`. Legacy `accumulator_names` + `reaction_mode` retained for packaging FFI + reconciler back-compat; documented.
- `cloacina/src/registry/reconciler/loading.rs`: the packaged-CG reconciler construction now populates the new fields (entry_accumulators = accumulator_names, trigger_reactor = None).
- `cloacina-macros/src/computation_graph/parser.rs`: `ParsedTopology` replaces its `react` field with `trigger: TriggerSpec` (`Bundled(ReactionCriteria) | ByReactor(syn::TypePath) | None`). New parser branches: `trigger = reactor(TypePath)` and no-trigger (neither `react` nor `trigger`). Mutually-exclusive validation. New tests cover all three forms plus error paths.
- `cloacina-macros/src/computation_graph/graph_ir.rs`: `GraphIR.react` → `GraphIR.trigger`. Added `entry_accumulators()` helper computing the unique cache-input set from entry nodes.
- `cloacina-macros/src/computation_graph/codegen.rs`: large refactor. Per-trigger branching produces the right tokens for `legacy_acc_names_expr` / `legacy_reaction_mode_expr` / `trigger_reactor_expr` + FFI block tokens + `synth_reactor_decl` (bundled-only) + `type_binding_check` (split-only). Synthesized bundled reactor is emitted as `pub struct __Reactor_<graphname>;` with full `Reactor` trait impl + `ReactorEntry` inventory submit. Graph registration now carries the new fields. Bundled-form macro-time sanity check asserts entry accumulators are a subset of the `react = ...` list.

**M4: Compile-time subset check (split form).**
- Emitted per split-form graph as a `const fn __cloacina_check_reactor_binding_<mod>()` that compares bytes of each entry accumulator string against `<ReactorType as Reactor>::ACCUMULATORS`. Invoked via `const _: () = __cloacina_check_reactor_binding_<mod>();` so a missing accumulator surfaces as a compile-time panic with a concat!'d message naming the graph.

**Tests:**
- `cloacina-macros` lib tests: 40 green (was 27; added parser split/trigger-less/conflict/unknown-kind tests + 14 reactor macro expansion tests).
- `cloacina` integration cg tests: 31 green — all 27 existing bundled-form tests unchanged, plus 4 new T-0538 tests:
  - `test_cloaci_t_0538_reactor_trait_constants` — verifies `<T as Reactor>::{NAME,ACCUMULATORS,REACTION_MODE}` constants.
  - `test_cloaci_t_0538_split_form_compiled_fn_runs` — split-form graph with `trigger = reactor(CloaciT0538SplitReactor)` expands, passes the const subset check, and runs against a populated `InputCache`.
  - `test_cloaci_t_0538_triggerless_form_compiled_fn_runs` — trigger-less `graph = {...}` form expands and runs.
  - `test_cloaci_t_0538_runtime_reactor_registry_shape` — `Runtime::register_reactor`/`get_reactor`/`user_reactor_names` handle user and synthesized `__Reactor_*` names, and `user_reactor_names` filters the dunder prefix.
- `cargo check --workspace --all-features` green.

**Scope note on inventory-in-tests.** The `#[computation_graph]` and `#[reactor]` macros gate their `inventory::submit!` blocks with `#[cfg(not(test))]` — the existing bundled-form design. That means in-process `Runtime::new().get_computation_graph(name)` won't observe macro-declared graphs from test binaries. Integration tests here prove shape and runtime surface via manual registration; end-to-end "fires via reactor" coverage is deferred to M5 scheduler work (below).

### Remaining work

**M5: Scheduler split.** Today `ComputationGraphScheduler::load_graph(ComputationGraphDeclaration)` takes a bundled shape (`accumulators: Vec<AccumulatorDeclaration>` + `reactor: ReactorDeclaration { graph_fn }`). To fire split-form graphs end-to-end we need:
- A `load_reactor(ReactorRegistration, accumulator_factories: ...)` that builds accumulators + the reactor loop, publishing its firing output on a named channel.
- A `bind_graph_to_reactor(graph_name, reactor_name)` (or implicit binding at graph load) that subscribes the graph fn to the reactor's firings.
- Trigger-less graphs: register the compiled fn by name only; the first consumer is T-02/T-03 (workflow tasks / Python decorators invoking CGs).

This is a real surface change and wants its own architectural check-in before coding.

**M6: End-to-end integration test.** Once M5 lands, add a test that declares a `#[reactor]` + `#[computation_graph(trigger = reactor(X), ...)]`, pushes an event to its accumulator via the registry, and asserts the graph fires exactly as today's bundled-form scheduler test does.

**M7: `angreal cloacina unit` + `angreal cloacina integration --backend sqlite` full runs.** Pending all above.

Stopping this iteration here for user review. M1–M4 are solid and ready for inspection.

### 2026-04-24 — Milestones 5 + 6 landed locally (not yet committed)

User directive for M5: "making a change now is the right thing if we're going to do it." Proceeded with scheduler API additions.

**M5: Scheduler split.**
- `cloacina/src/computation_graph/reactor.rs`: added `From<ReactionMode> for ReactionCriteria` (maps the `cloacina-computation-graph` enum into the scheduler enum).
- `cloacina/src/computation_graph/scheduler.rs`:
  - `ComputationGraphScheduler` grew a `triggerless_graphs: Arc<RwLock<HashMap<String, CompiledGraphFn>>>` field.
  - New `load_graph_split(graph_name, graph_fn, &ReactorRegistration, accumulators, tenant_id)`. Validates that every accumulator declared on the `ReactorRegistration` has a matching `AccumulatorDeclaration` supplied, then constructs a `ComputationGraphDeclaration` internally and delegates to `load_graph`. One reactor instance per graph for now (the linkage is preserved, just named rather than bundled); sharing one reactor across multiple graphs is future work for T-01b.
  - New trigger-less surface: `register_triggerless_graph` / `invoke_triggerless_graph` / `triggerless_graph_names` / `unregister_triggerless_graph`. These do not touch the reactor path — the compiled fn is kept by name, ready for T-02 workflow-task invocation / T-03 Python decorator parity.
  - `load_graph` and the existing bundled-form surface are untouched. Split-form and trigger-less callers get their own entry points.

**M6: Split-form end-to-end + trigger-less invocation integration tests.**
- `test_cloaci_t_0538_split_form_scheduler_end_to_end` — builds a `ReactorRegistration` matching `CloaciT0538SplitReactor`, wires it through `load_graph_split` with a `TestAccumulatorFactory`, pushes an event via the endpoint registry, polls until the graph fires, asserts fire_count == 1, and unloads cleanly.
- `test_cloaci_t_0538_triggerless_scheduler_invocation` — registers, lists, rejects duplicates, invokes via `invoke_triggerless_graph`, unregisters, confirms post-unregister invocation returns `None`.
- `test_cloaci_t_0538_split_missing_accumulator_fails` — confirms the validation error surfaces when the reactor declares an accumulator the caller forgot to supply.

**Test totals:**
- `cloacina-macros` lib: 40 green.
- `cloacina` integration cg suite: 34 green (27 pre-existing bundled-form + 7 new T-0538).
- `cargo check --workspace --all-features` green.

### AC coverage snapshot

- [x] `#[computation_graph]` accepts `trigger = reactor(TypePath)` and emits a declaration referencing the reactor by name.
- [x] `#[computation_graph]` accepts no trigger clause.
- [x] Compile-time type binding: split form emits `const fn __cloacina_check_reactor_binding_<mod>()` invoked at const eval; mismatches produce a compile error with a graph-named message via `concat!`.
- [x] Bundled form continues to compile and run; it desugars to a synthesized `__Reactor_<graphname>` reactor declaration (inventory entry with dunder-prefix name, filtered out by `Runtime::user_reactor_names`).
- [x] Unit tests: macro expansion for all three forms (parser + integration tests cover bundled, split, trigger-less).
- [x] Integration tests: split-form CG fires via reactor end-to-end; trigger-less CG compiles and is present in the registry (via `register_triggerless_graph`).
- [ ] `cargo check --workspace --all-features` green — **DONE** (local cargo).
- [ ] `angreal cloacina unit` green — **needs user to run (feedback_external_commands memory)**.
- [ ] `angreal cloacina integration --backend sqlite` green — **needs user to run**.

### What's left

Real-world verification via the angreal task suites (user preference is to run tests externally, not in-tool). After those land green, final wrap-up: commit, PR, Metis transition.

### 2026-04-24 — Angreal suites green; AC complete

User approved in-session runs.

- `angreal cloacina unit` — **701 passed, 0 failed, 1 ignored**. Includes the 40 cloacina-macros lib tests plus all cloacina unit tests.
- `angreal cloacina integration --backend sqlite` — **Rust integration suite 282 scenarios green**, **Python sqlite scenarios 27/27 passed**. Includes the 34 computation_graph integration tests (27 existing bundled-form + 7 new T-0538).

All acceptance criteria ticked:

- [x] `#[computation_graph]` accepts `trigger = reactor(TypePath)`.
- [x] `#[computation_graph]` accepts trigger-less form.
- [x] Compile-time type binding via const-eval subset check; mismatch is a compile error with a graph-named message.
- [x] Bundled form still compiles and runs; desugars to synthesized `__Reactor_<graphname>` reactor declaration.
- [x] Unit tests for macro expansion in all three forms.
- [x] Integration tests: split-form CG fires via reactor end-to-end; trigger-less CG is present in the registry.
- [x] `cargo check --workspace --all-features` green.
- [x] `angreal cloacina unit` green.
- [x] `angreal cloacina integration --backend sqlite` green.

Signaling ready for user review and phase transition.
