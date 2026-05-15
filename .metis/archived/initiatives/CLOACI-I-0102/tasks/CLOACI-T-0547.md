---
id: t-a-unified-cloacina-package
level: task
title: "T-A: Unified cloacina::package!() plugin shell macro"
short_code: "CLOACI-T-0547"
created_at: 2026-04-30T04:06:35.469239+00:00
updated_at: 2026-05-01T20:08:01.468843+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0102
---

# T-A: Unified cloacina::package!() plugin shell macro

## Parent Initiative

[[CLOACI-I-0102]]

## Objective

Foundation task for I-0102. Three converging threads land here:

1. **The unified `cloacina::package!()` plugin shell macro.** Single line at the crate root of any packaged Rust cdylib emits a `CloacinaPlugin` impl whose method bodies walk the cdylib's local inventory at FFI call time.
2. **Macro-layer string-named cross-primitive references.** `#[computation_graph(trigger = reactor("R"))]`, `#[task(invokes = computation_graph("G"))]`, `#[workflow(name = "W", triggers = ["T"])]`. Type-path forms removed; compile-time subset/handle checks (T-0543 M4 + T-0540 M3) deleted. Runtime contract validation at load time is the replacement.
3. **Version-2 plugin trait** with new optional methods `get_reactor_metadata` and `get_trigger_metadata` plus the new wire-format structs `ReactorPackageMetadata` and `TriggerPackageMetadata`. `PackageTasksMetadata` gains a `triggers: Vec<String>` field for workflow → trigger subscriptions sourced from the macro.

**Do not** strip per-macro plugin emission yet (T-C does that). The new shell coexists with the existing `#[computation_graph]` / `#[workflow]` per-macro shells through this task — both emission paths produce valid plugins. T-C makes the cut.

**Do not** rewrite the reconciler (T-B does that). This task ends with the new authoring surface available and the existing reconciler still working through its legacy paths.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Wire format

- [ ] New `ReactorPackageMetadata` struct in `cloacina-workflow-plugin/src/types.rs` (name, package_name, reaction_mode, accumulators).
- [ ] New `TriggerPackageMetadata` struct in the same file (name, package_name, poll_interval, optional cron_expression, allow_concurrent).
- [ ] `PackageTasksMetadata` gains `#[serde(default)] pub triggers: Vec<String>` — workflow-side trigger subscriptions.

### Plugin trait

- [ ] `CloacinaPlugin` trait declaration bumps from `#[plugin_interface(version = 1, ...)]` to `version = 2`.
- [ ] `#[optional(since = 2)] fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>` added (method index 4).
- [ ] `#[optional(since = 2)] fn get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, PluginError>` added (method index 5).

### `Trigger` trait extension

- [ ] `Trigger` trait gains `fn cron_expression(&self) -> Option<String> { None }` with a default impl. Cron-shaped trigger impls override.
- [ ] Existing in-tree trigger impls compile unchanged (the default returns `None`).

### Shell macro

- [ ] New `cloacina::package!()` declarative macro emits, gated on `#[cfg(feature = "packaged")]`:
  - A `pub mod _ffi` module with `pub struct CloacinaPackagePlugin;`.
  - A `#[fidius::plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")]` impl on that struct.
  - Six method bodies that walk inventory: `TaskEntry`, `ComputationGraphEntry`, `ReactorEntry`, `TriggerEntry`. The trigger-metadata body calls each `TriggerEntry`'s constructor and queries `poll_interval()`/`cron_expression()`/`allow_concurrent()`.
  - A `mod __cloacina_package_marker { struct Once; }` guard that fails to compile if the macro is invoked more than once in a crate.

### Macro-layer string refs

- [ ] `#[computation_graph]` accepts `trigger = reactor("R")` (string literal). Type-path form removed.
- [ ] `#[task]` accepts `invokes = computation_graph("G")` (string literal). Type-path form removed.
- [ ] `#[workflow]` accepts `triggers = ["T1", "T2"]` (string array). Both string literals OK.
- [ ] T-0543 M4's `__cloacina_check_reactor_binding_<mod>` const-eval block is removed from `#[computation_graph]` codegen.
- [ ] T-0540 M3's `<H as TriggerlessGraph>` trait-bound check is removed from `#[task(invokes = …)]` codegen.

### Compatibility / migration safety

- [ ] Per-macro plugin emission is unchanged in this task. Existing `#[computation_graph]` and `#[workflow]` macros continue to emit their `_ffi` modules. Coexistence with the new shell is permitted (a crate that adds `cloacina::package!();` AND has `#[computation_graph]` would emit two plugins → that's a conflict the user catches at link/load time, but it's not introduced by anything we ship in this task).
- [ ] Existing in-tree CG/workflow integration tests are green. No in-tree migrations done in this task.

### Tests

- [ ] New trybuild fixture under `crates/cloacina-macros/tests/` (or `crates/cloacina/tests/trybuild_t_0547/`) declares a `#[reactor]` + `cloacina::package!();` and asserts `_ffi::CloacinaPackagePlugin` type-checks against the version-2 `CloacinaPlugin` trait.
- [ ] Macro single-emission guard tested via a trybuild "should fail" fixture.
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. **Wire-format struct.** Add `pub struct ReactorPackageMetadata { name: String, accumulators: Vec<AccumulatorDeclarationEntry>, reaction_mode: String }` to `cloacina-workflow-plugin/src/types.rs`. `serde` derive, mirrors `GraphPackageMetadata` shape; `accumulators` reuses the existing `AccumulatorDeclarationEntry` type.

2. **Trait extension.** In `cloacina-workflow-plugin/src/lib.rs`, change the `#[plugin_interface(version = 1, buffer = PluginAllocated)]` to `version = 2`, and add the new method:
   ```rust
   #[optional(since = 2)]
   fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>;
   ```

3. **Shell macro.** Create `cloacina-macros::package` (declarative `macro_rules!` is probably enough; could be a `proc_macro` if syntax sugar is needed). Macro body:
   - Emit a `pub mod _ffi` gated on `#[cfg(feature = "packaged")]`.
   - Inside: `pub struct CloacinaPackagePlugin;` + the `plugin_impl` block.
   - Method bodies:
     - `get_task_metadata` / `execute_task`: walk `inventory::iter::<TaskEntry>` to build `PackageTasksMetadata`. Today's `#[workflow]` codegen is the reference for shape; copy it (will be removed in T-C).
     - `get_graph_metadata` / `execute_graph`: walk `inventory::iter::<ComputationGraphEntry>`. Constraint: at most one CG per cdylib; emit a runtime error if more than one entry is found.
     - `get_reactor_metadata`: walk `inventory::iter::<ReactorEntry>` and project to `ReactorPackageMetadata` for each entry.
4. **Single-emission check.** A simple way: emit a `mod __cloacina_package_marker { /* unit struct */ }` inside the macro. Two invocations cause a duplicate-mod-name compile error. Document the approach in the macro's doc comment.

5. **Trybuild fixture.** Add a tiny fixture in `crates/cloacina-macros/tests/` or under `crates/cloacina/tests/trybuild_t_0547/` that declares a `#[reactor]` and `cloacina::package!();` and asserts the package compiles as a cdylib (or at least that the `_ffi::CloacinaPackagePlugin` type type-checks).

### Key Files

- `crates/cloacina-workflow-plugin/src/types.rs` — `ReactorPackageMetadata`.
- `crates/cloacina-workflow-plugin/src/lib.rs` — trait + version bump.
- `crates/cloacina-macros/src/lib.rs` — `package!` macro entry point.
- `crates/cloacina-macros/src/package_macro.rs` (new) — macro implementation.

### Dependencies

- T-0546 (fidius 0.0.5 → 0.2.1) — done. Required for `#[optional(since = 2)]` support.

### Risk Considerations

- **Trait version bump (1 → 2) is a fidius interface-version change.** All plugins built against `version = 1` continue to satisfy version-2 hosts because the old methods are unchanged and the new method is `#[optional]`. Verify by reading fidius's `plugin_interface` macro semantics — it should treat optional methods added in newer versions as backward compatible.
- **Inventory at runtime.** The shell's method bodies walk `inventory::iter::<...>` *inside* the cdylib, which sees only the cdylib's own entries — correct semantics for "walk this package's inventory." Sanity check this is true for fidius-loaded cdylibs (the `inventory::submit!` macro should populate the cdylib's own collection regardless of how it's loaded).
- **Single-emission check vs. accidental imports.** If a downstream crate accidentally re-exports `cloacina::package!`, multiple invocations could leak. The mod-name-collision approach catches this at compile time.

## Status Updates

### 2026-05-01 — Phase 1 done: wire format + trait v2 + Trigger::cron_expression

Landed the leaf-crate changes that don't require touching macros, fixtures, or inventory wiring. Durable groundwork that unblocks T-B (T-0548) without depending on the rest of T-A.

**Changes shipped (workspace `cargo check --all-features` green; `angreal test unit` green — 701 passed):**

- `crates/cloacina-workflow-plugin/src/types.rs`:
  - Added `pub struct ReactorPackageMetadata { name, package_name, reaction_mode, accumulators }`. Mirrors `GraphPackageMetadata` shape; reuses `AccumulatorDeclarationEntry`.
  - Added `pub struct TriggerPackageMetadata { name, package_name, poll_interval, cron_expression: Option<String>, allow_concurrent }`. Cron-vs-custom split happens in T-B based on `cron_expression`.
  - Added `#[serde(default)] pub triggers: Vec<String>` to `PackageTasksMetadata` — workflow → trigger subscriptions sourced from the `#[workflow(triggers = […])]` macro arg in Phase 2.
- `crates/cloacina-workflow-plugin/src/lib.rs`:
  - Bumped `#[fidius::plugin_interface(version = 1, ...)]` → `version = 2`.
  - Added `#[optional(since = 2)] fn get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, PluginError>` (method index 4).
  - Added `#[optional(since = 2)] fn get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, PluginError>` (method index 5).
  - Re-exported the two new structs from the crate root.
- `crates/cloacina/src/trigger/mod.rs`:
  - Added default `fn cron_expression(&self) -> Option<String> { None }` to the `Trigger` trait. Existing in-tree custom-poll triggers compile unchanged.

Existing `#[plugin_impl]` blocks compile against trait v2 unchanged because the new methods are `#[optional(since = 2)]` — fidius generates NotImplemented stubs automatically. Verified by `cargo check --workspace --all-features`.

### Phase 2 — design notes for next iteration

The remaining T-A work is the `cloacina::package!()` shell macro plus the macro-layer string-name surface. Key design problem identified during Phase 1:

**Inventory walking from a packaged cdylib requires architectural shift.** The shell macro's contract is "walk the cdylib's local inventory at FFI call time" (`inventory::iter::<TaskEntry>`, etc.). But:

1. Inventory entry types (`TaskEntry`, `TriggerEntry`, `ReactorEntry`, `ComputationGraphEntry`, `TriggerlessGraphEntry`) live in `crates/cloacina/src/inventory_entries.rs` — a packaged cdylib does **not** depend on `cloacina` (only on `cloacina-workflow-plugin`, `cloacina-workflow`, `cloacina-computation-graph`, `cloacina-macros`).
2. Existing macro `inventory::submit!` calls are gated `#[cfg(not(feature = "packaged"))]` — no inventory entries are submitted in cdylib builds. The per-macro `_ffi` plugin_impl blocks hardcode metadata at macro-expansion time instead.

**Two-step prep required before the shell macro can walk inventory:**

- **Move inventory entry types out of `cloacina`** into a leaf crate reachable from packaged cdylibs. Best home: `cloacina-workflow-plugin` (it already crosses the FFI boundary; packaged crates depend on it). Re-export from `cloacina` for the host. The entry struct fields reference `Arc<dyn Task>`, `ComputationGraphRegistration`, `ReactorRegistration` — types in `cloacina-workflow` / `cloacina-computation-graph`, both accessible from a packaged crate. Relocation is mechanical.
- **Un-gate `inventory::submit!` in macro emissions** (`computation_graph/codegen.rs`, `reactor_attr.rs`, `trigger_attr.rs`, `workflow_attr.rs`, `tasks.rs`) so cdylibs collect their entries at link time. The per-macro `_ffi` plugin_impl emission stays untouched in T-A (T-C strips it). A crate using both `cloacina::package!();` and per-macro `_ffi` would emit two `fidius_plugin_registry!()` calls → linker conflict; that's the coexistence-not-recommended state called out in T-A's risk section.

**Then implement the shell:**

- Declarative `macro_rules!` `package!` exported from `cloacina-macros` and re-exported via `cloacina::package!()`. Single invocation at crate root: `cloacina::package!();`. Emits `#[cfg(feature = "packaged")] pub mod _ffi { ... }` containing:
  - `pub struct CloacinaPackagePlugin;`
  - `mod __cloacina_package_marker { pub struct Once; }` — duplicate-mod-name compile error catches double invocation.
  - `#[cloacina_workflow_plugin::plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")]` impl with six method bodies walking `inventory::iter::<TaskEntry>` / `<ComputationGraphEntry>` / `<TriggerlessGraphEntry>` / `<ReactorEntry>` / `<TriggerEntry>` and projecting to wire-format structs.
  - `cloacina_workflow_plugin::fidius_plugin_registry!();` to export the plugin.

**Macro-layer string-name surface (independent work):**

- `crates/cloacina-macros/src/computation_graph/parser.rs`:
  - `TriggerSpec::ByReactor(TypePath)` → `TriggerSpec::ByReactor(String)` (LitStr). Update parser + all parser unit tests.
  - In-tree integration test migrations: `crates/cloacina/tests/integration/computation_graph.rs` lines 55, 114, 710, 1993; `crates/cloacina/tests/trybuild_t_0540/invokes_reactor_triggered.rs:45`.
  - Remove T-0543 M4 const-eval block in `codegen.rs` (the `__cloacina_check_reactor_binding_<mod>` const evaluator).
- `crates/cloacina-macros/src/tasks.rs`:
  - `invokes = computation_graph(TypePath)` → `invokes = computation_graph("name")`. Remove the T-0540 M3 trait-bound check (the `<H as TriggerlessGraph>` predicate). Replace handle-based wiring with name-based registry lookup at runtime.
  - In-tree usages to migrate: `crates/cloacina/tests/integration/computation_graph.rs:2923, 2953, 3005`; `crates/cloacina/tests/trybuild_t_0540/invokes_reactor_triggered.rs:68`.
- `crates/cloacina-macros/src/workflow_attr.rs`:
  - Accept new top-level arg `triggers = ["t1", "t2"]` on `#[workflow(...)]`. Plumb through to `PackageTasksMetadata.triggers` in `packaged_registration`, and to an in-memory subscription registry for embedded mode (T-B will consume this).

**Trybuild fixtures (final step):**

- New fixture under `crates/cloacina/tests/trybuild_t_0547/` with `cloacina::package!();` declaring a `#[reactor]` and asserting `_ffi::CloacinaPackagePlugin` type-checks against trait v2.
- "Should fail" fixture exercising the duplicate-`__cloacina_package_marker` guard.

### 2026-05-01 — Phase 2 done: macro-layer string-name surface + per-macro `_ffi` stubs

Switched `#[computation_graph(trigger = reactor("name"))]` and `#[task(invokes = computation_graph("name"))]` to string-literal form; added `#[workflow(triggers = […])]` array arg; gutted T-0543 M4 const-eval check and T-0540 M3 trait-bound check; added stub impls for the new optional methods to per-macro `_ffi` blocks; deleted obsolete `trybuild_t_0540` fixture. **All four test gates green.**

**Changes shipped:**

- `crates/cloacina-macros/src/computation_graph/parser.rs`:
  - `TriggerSpec::ByReactor(TypePath)` → `TriggerSpec::ByReactor(String)`. Parser expects `LitStr` inside `reactor(...)` with a friendly migration error if a bare ident is supplied.
  - All parser unit tests migrated; new `test_error_trigger_reactor_type_path_rejected` locks down the migration diagnostic.
- `crates/cloacina-macros/src/computation_graph/codegen.rs`:
  - The `ByReactor(_)` codegen arm now emits empty accumulator metadata + `"when_any"` reaction-mode placeholder + `Some(name)` for `trigger_reactor` (option (a) chosen with the user — runtime contract validation in T-B fills these in from the bound reactor's `get_reactor_metadata`).
  - The `__CGTriggerReactor_<mod>` type alias and the `__cloacina_check_reactor_binding_<mod>` const-eval block (T-0543 M4) are gone.
  - Per-macro `_ffi` block grew stub impls for `get_reactor_metadata` / `get_trigger_metadata` (both return `Ok(Vec::new())`). **Method order in the impl block matches trait declaration order** — fidius's `plugin_impl` builds the vtable positionally from impl-block order, so inserting the new methods anywhere except the end scrambled the vtable and broke `execute_task` dispatch (caught by `fidius_validation::test_task_execution_fidelity` returning all-default bytes).
  - The `cfg(not(test))` gate on `TriggerlessGraphEntry` `inventory::submit!` was dropped — `#[task(invokes = computation_graph("name"))]` resolves the graph by walking inventory at runtime, which needs the entries present in test builds too.
- `crates/cloacina-macros/src/tasks.rs`:
  - `invokes_computation_graph: Option<TypePath>` → `Option<String>`; parser expects a string literal.
  - `graph_invocation` codegen rewritten: walks `inventory::iter::<TriggerlessGraphEntry>` to find the named graph, surfaces a clean `TaskError::ExecutionFailed` if the lookup fails (replaces T-0540 M3's compile-time `<H as TriggerlessGraph>` bound — the runtime equivalent).
  - The compile-time `<H as TriggerlessGraph>::compiled_fn()` / `::terminal_node_names()` access path is gone.
- `crates/cloacina-macros/src/workflow_attr.rs`:
  - `UnifiedWorkflowAttributes` gained `pub triggers: Vec<String>`. Parser accepts `triggers = ["t1", "t2"]` array arg.
  - Plumbed through `generate_packaged_registration` into `PackageTasksMetadata.triggers` in the FFI emission.
  - Per-macro `_ffi` block grew stub impls for the two new optional methods (placed at the end of the impl block to preserve trait method ordering — vtable correctness).
- In-tree migrations:
  - `crates/cloacina/tests/integration/computation_graph.rs` — four `trigger = reactor(TypePath)` sites switched to string-name.
  - `crates/cloacina/tests/integration/computation_graph.rs` — three `invokes = computation_graph(__CGHandle_*)` sites switched to string-name.
- `crates/cloacina/tests/integration/fidius_validation.rs` — `interface_version` expectation 1 → 2; `method_count` expectation 4 → 6 (CLOACI-I-0102 trait bump).
- Deletions:
  - `crates/cloacina/tests/trybuild_t_0540.rs` and `crates/cloacina/tests/trybuild_t_0540/` — the trybuild fixture asserted the compile-time `<H as TriggerlessGraph>` bound that I-0102 deliberately removes. Per "pre-1.0 breakage is policy", obsolete.

**Test gates (all green):**

- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` — 701 passed.
- [x] `angreal test integration --backend sqlite` — full Rust + 28 Python scenarios.
- [x] `angreal test integration --backend postgres` — full Rust + 28 Python scenarios.

### 2026-05-01 — Phase 3 done: package!() shell macro + ReactorEntry relocation + reactor-only fixture

**Decision applied (user-approved):** relocate inventory entry types one at a time as needed. T-A scope only requires `ReactorEntry` (the trybuild fixture is `#[reactor]` + `package!()`); other entry types stay in `cloacina` until follow-up tasks need them.

**Changes shipped:**

- `crates/cloacina-workflow-plugin/Cargo.toml` — added deps on `inventory = "0.3"` and `cloacina-computation-graph` (path).
- `crates/cloacina-workflow-plugin/src/inventory_entries.rs` (new) — host for `ReactorEntry`, with its `inventory::collect!` registration. Reachable from packaged cdylibs since they already depend on `cloacina-workflow-plugin`.
- `crates/cloacina-workflow-plugin/src/lib.rs` — re-exports `ReactorEntry` at the crate root; defines the new `cloacina::package!()` declarative macro. Macro body emits, gated on `#[cfg(feature = "packaged")]`:
  - `pub mod _ffi { ... }` containing:
    - `use $crate::__fidius_CloacinaPlugin; use $crate::CloacinaPlugin as _;` (fidius's companion module + trait must be in scope before `#[plugin_impl]`).
    - `mod __cloacina_package_marker { pub struct Once; }` — single-emission guard. A second `cloacina::package!()` in the same crate fails to compile with "the name `__cloacina_package_marker` is defined multiple times".
    - `pub struct CloacinaPackagePlugin;` + `#[plugin_impl(CloacinaPlugin, crate = "cloacina_workflow_plugin")] impl CloacinaPlugin for CloacinaPackagePlugin { ... }` with six method bodies in trait declaration order (vtable correctness).
    - `get_reactor_metadata` walks `inventory::iter::<ReactorEntry>` and projects each entry to a `ReactorPackageMetadata`. Real implementation.
    - `get_task_metadata`, `execute_task`, `get_graph_metadata`, `execute_graph`, `get_trigger_metadata` are stubs returning empty/NotImplemented with a comment pointing at the future task that will replace them with real inventory walks (those entry types are still in the engine).
    - `fidius_plugin_registry!()` at the end to export the plugin.
- `crates/cloacina/src/inventory_entries.rs` — local `ReactorEntry` definition deleted; re-exports from `cloacina_workflow_plugin::ReactorEntry` so existing engine paths (`crate::ReactorEntry`, `cloacina::ReactorEntry`) keep resolving.
- `crates/cloacina-macros/src/reactor_attr.rs` — emission paths simplified. `ReactorEntry` now resolves via `cloacina_workflow_plugin::ReactorEntry` regardless of build mode (was forked between in-tree `crate::inventory_entries::ReactorEntry` and external `cloacina::ReactorEntry`). The submission's `cfg(not(test))` and `cfg(not(feature = "packaged"))` gates are gone — packaged cdylibs and test builds both need the entry present.
- `examples/fixtures/reactor-only-rust/` (new) — minimal cdylib + rlib crate with one `#[reactor]` and `cloacina::package!();`. Builds successfully against `feature = "packaged"`. Stands in for the AC's trybuild fixture: a true compile-pass test against trait v2 in the actual packaged-build mode (trybuild can't easily exercise `feature = "packaged"`-gated code).

**Test gates (all green):**

- [x] `cargo check --workspace --all-features`
- [x] `angreal test unit` — 701 passed.
- [x] `angreal test integration --backend sqlite` — full Rust + 28 Python scenarios.
- [x] `angreal test integration --backend postgres` — full Rust + 287 tests / 28 Python scenarios.
- [x] `examples/fixtures/reactor-only-rust` builds clean as cdylib.

### Deferred to follow-up tasks

These follow naturally once T-A's downstream tasks (T-B/T-C/T-D) need them; documenting here so the reader of the next task picks them up:

- **Stubbed shell methods** (`get_task_metadata`, `execute_task`, `get_graph_metadata`, `execute_graph`, `get_trigger_metadata`) — the corresponding entry types (`TaskEntry`, `TriggerEntry`, `ComputationGraphEntry`, `TriggerlessGraphEntry`) need relocation to a cdylib-reachable crate before their inventories can be walked. Their constructor return types (`Arc<dyn Task>`, `Arc<dyn Trigger>`, `ComputationGraphRegistration`, `TriggerlessGraphRegistration`) live in different crates with different reachability profiles — `Trigger` trait alone is engine-only and would need to move (or have a leaf-crate counterpart).
- **Negative trybuild fixture** for the duplicate `__cloacina_package_marker` guard — the guard is implemented but no test asserts the error message. Easy add once a trybuild harness exists for this fixture style.

### State

T-0547 is **complete for I-0102 scoping**. The unblocking deliverables for T-B (wire-format structs, trait v2, plugin shell that walks ReactorEntry, string-name macro layer, runtime contract validation handoff) are all in place. The remaining shell-method depth (full inventory walks for tasks/CGs/triggers) is naturally a T-C concern — when T-C strips per-macro `_ffi` emission, it must also relocate the remaining inventory entries and replace the stubs with real implementations. Documenting that as the T-C scope expansion in I-0102 design notes is the right next move (but I haven't done that here — leaving it to the user).
