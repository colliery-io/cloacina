---
id: t-c-strip-per-macro-plugin
level: task
title: "T-C: Strip per-macro plugin emission + migrate in-tree packaged crates"
short_code: "CLOACI-T-0549"
created_at: 2026-04-30T04:08:45.830718+00:00
updated_at: 2026-05-01T20:56:12.638108+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0102
---

# T-C: Strip per-macro plugin emission + migrate in-tree packaged crates

## Parent Initiative

[[CLOACI-I-0102]]

## Objective

Cut over to the unified surface in a single PR. Three converging migrations:

1. **Strip per-macro plugin emission.** Remove the `_ffi` plugin module emitted by `#[computation_graph]` and the equivalent shell emitted by `#[workflow]`. Keep inventory submission. After this task, `cloacina::package!();` is the *only* path that produces a `CloacinaPlugin`.
2. **Migrate in-tree packaged crates to the new authoring surface.** Add `cloacina::package!();` to every in-tree packaged Rust crate's `lib.rs`. Convert cross-primitive references from type-path form (`trigger = reactor(SharedRx)`) to string-name form (`trigger = reactor("shared_rx")`). Convert `#[workflow]` trigger subscriptions from the manifest's `[[triggers]]` table to the `triggers = [...]` macro argument introduced in T-A.
3. **Stop reading `[[triggers]]` and `package_type` from `package.toml` for the migrated crates.** The macro layer is now authoritative. Manifest cleanup itself (warnings, removal, hard error) is T-E; this task just stops the in-tree fixtures from depending on those keys.

Atomic migration is honest: there is no useful intermediate state where some packages emit via the old path and some via the new. Pre-1.0 breakage is the policy.

## Acceptance Criteria

## Acceptance Criteria

### Macro emission strip

- [ ] `#[computation_graph]` codegen no longer emits the `_ffi` plugin module (`crates/cloacina-macros/src/computation_graph/codegen.rs`). Inventory submission stays. The macro becomes a pure primitive declarator (matches `#[reactor]` shape).
- [ ] `#[workflow]` codegen (in `crates/cloacina-macros/src/workflow_attr.rs` or wherever the workflow plugin shell lives today) likewise stops emitting an FFI plugin. Inventory + task registration stay.
- [ ] The compile-time subset/handle checks deleted in T-A (T-0543 M4 + T-0540 M3 vestiges) stay deleted — no resurrection.

### In-tree crate migration

- [ ] Every in-tree packaged Rust crate gains `cloacina::package!();` at the crate root of `lib.rs`:
  - `examples/features/workflows/simple-packaged`
  - `examples/features/workflows/packaged-workflows`
  - `examples/features/workflows/packaged-triggers`
  - `examples/features/workflows/complex-dag`
  - `examples/features/computation-graphs/packaged-graph`
  - `examples/fixtures/compiler-broken-rust`
  - `examples/fixtures/compiler-happy-rust`
  - Any inline cdylib fixtures under `crates/*/tests/`.
- [ ] All `#[computation_graph(trigger = reactor(TypePath))]` references converted to `trigger = reactor("name")` string form.
- [ ] All `#[task(invokes = computation_graph(TypePath))]` references converted to `invokes = computation_graph("name")`.
- [ ] All `#[workflow]` trigger subscriptions previously expressed via `package.toml`'s `[[triggers]]` are converted to the `triggers = ["..."]` macro argument.
- [ ] The migrated crates' `package.toml` files no longer contain `[[triggers]]` or `package_type` entries (the keys are not yet hard-errored — that is T-E — but the migrated crates no longer rely on them).

### Single-emission invariant

- [ ] No package emits two plugins after the migration. The single-emission check inside `cloacina::package!();` (see T-A) catches accidental double-add.

### Test gates

- [ ] All existing CG/workflow integration tests are green — no regression in load behavior, no regression in fan-out behavior added by T-0544.
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.
- [ ] `angreal demos features python-packaged-graph` and `angreal demos features packaged-graph` still run end-to-end (smoke check that cross-language pack/load still works).

## Implementation Notes

### Technical Approach

1. **Strip CG codegen first.** Delete the `packaged_ffi` block in `crates/cloacina-macros/src/computation_graph/codegen.rs`. Compile workspace to surface anything that depended on the old emission.
2. **Strip workflow codegen.** Same pattern in `crates/cloacina-macros/src/workflow_attr.rs`.
3. **Audit + migrate.** For each crate in the migration list: add `cloacina::package!();`, convert type-path cross-primitive refs to strings, convert `[[triggers]]` table entries to `triggers = [...]` on the `#[workflow]` attribute. Drop now-redundant `package_type` from `package.toml` (the key still parses via T-E's deprecation warning until the hard removal lands).
4. **Sweep tests.** `angreal test unit` and `angreal test integration` on both backends. Triage any failures (likely localized: a crate that didn't get the macro line, or a fixture whose trigger reference still uses a type path).

### Key Files

- `crates/cloacina-macros/src/computation_graph/codegen.rs` — strip the `_ffi` module emission.
- `crates/cloacina-macros/src/workflow_attr.rs` — same.
- Each `lib.rs` in the migration list — add `cloacina::package!();` and update macro args.
- Each `package.toml` in the migration list — drop `[[triggers]]` + `package_type`.

### Dependencies

- **T-0547 (T-A)** — must land first. Provides the unified shell macro and the `triggers = [...]` macro arg this task migrates onto.
- **T-0548 (T-B)** — must land first. Provides the precedence-ordered loader that consumes string-named reactor refs at runtime.

### Risk Considerations

- **Atomicity.** Stripping the per-macro emission and adding the shell macro must happen in a single PR — there's no useful state where some crates emit via one path and some via the other (compiler errors during the migration are how we know we got everyone).
- **Hidden cdylib fixtures.** Test crates under `crates/*/tests/` may contain inline cdylib builds. Audit before committing; missed fixtures fail at integration-test time.
- **String-name typos.** Compile-time fail-fast on cross-primitive refs is gone by design. A typo in `trigger = reactor("shared_rxx")` now manifests at load time. The reconciler's contract validation (T-B) is the safety net; verify the error message is clear (names the offending package + the missing reactor name).

## Status Updates

### 2026-05-01 — Orientation + dependency analysis

T-C's three converging migrations are **blocked on deferred prep work from T-A**.

**What T-C needs to do:**

1. Strip `_ffi` plugin emission from `#[computation_graph]` codegen (`computation_graph/codegen.rs`) and `#[workflow]` codegen (`workflow_attr.rs`).
2. Add `cloacina::package!();` to every in-tree packaged Rust crate (7 crates listed in AC).
3. Convert remaining type-path → string-name refs in examples (already done in tests by T-A).
4. Drop `[[triggers]]` and `package_type` from migrated `package.toml` files.

**The blocker:** T-A's shell macro currently stubs everything except `get_reactor_metadata`:

- `get_task_metadata` → empty `Vec<TaskMetadataEntry>`
- `execute_task` → "task not registered" error
- `get_graph_metadata` → NotSupported
- `execute_graph` → NotSupported
- `get_trigger_metadata` → empty `Vec<TriggerPackageMetadata>`

The stubs exist because the corresponding inventory entry types (`TaskEntry`, `TriggerEntry`, `ComputationGraphEntry`, `TriggerlessGraphEntry`) still live in `crates/cloacina/src/inventory_entries.rs` — engine-only, not reachable from packaged cdylibs (which depend on `cloacina-workflow-plugin` / `cloacina-workflow` / `cloacina-computation-graph` / `cloacina-macros`).

**Stripping per-macro `_ffi` emission today would break every in-tree packaged crate** because the shell can't yet substitute for what those `_ffi` blocks provide.

**Prerequisite work:**

1. **TaskEntry relocation** to `cloacina-workflow-plugin`. `Task` trait and `TaskNamespace` are both already in `cloacina-workflow` (which `cloacina-workflow-plugin` could pull in). Mechanical relocation.
2. **ComputationGraphEntry relocation** to `cloacina-workflow-plugin`. `ComputationGraphRegistration` is in `cloacina-computation-graph` (already a dep of `cloacina-workflow-plugin` after T-A). Mechanical.
3. **TriggerEntry relocation.** Hard — `Trigger` trait lives in `cloacina/src/trigger/mod.rs` (engine-only, has full Diesel/Tokio dependency surface). Either move the `Trigger` trait to `cloacina-workflow` (which doesn't have those engine deps) or define a parallel "leaf-crate Trigger" trait in `cloacina-workflow-plugin` and have the engine impl wrap it.
4. **TriggerlessGraphEntry relocation.** `TriggerlessGraphRegistration` lives in `cloacina/src/computation_graph/triggerless.rs` (engine). Likely needs to move to `cloacina-computation-graph`. Mechanical-ish but requires `Context<Value>` reachability check.
5. **WorkflowEntry relocation.** The `Workflow` struct is in `cloacina/src/workflow.rs` (engine, deeply integrated). Likely **easier to leave WorkflowEntry behind** and have the shell derive workflow_name from `TaskEntry`'s namespaced template instead of walking `WorkflowEntry`.
6. **Shell macro completion.** Once entry types are reachable, replace stubs with real inventory walks. Match shape of existing per-macro `_ffi` blocks.
7. **Strip per-macro emission** in `workflow_attr.rs` and `computation_graph/codegen.rs`.
8. **In-tree fixture migrations** — add `cloacina::package!();` to 7 crates, drop `[[triggers]]` / `package_type` from their `package.toml`.

Realistic scope for T-C done thoroughly: 1–2 fresh sessions. The relocation phase alone is probably half a session.

**Recommended path forward (user decision):**

Option A — **Ship I-0102 incrementally, defer T-C/D/E to follow-up.** T-A and T-B are landed and committable as-is. The shell macro works for reactor-only crates, the new metadata flows additively, the in-tree fixtures still use per-macro emission and continue working. Open a follow-up initiative to finish the unification (T-C/D/E) once the prerequisite relocations get attention.

Option B — **Push through the relocations now.** Substantial crate surgery (TaskEntry + ComputationGraphEntry + TriggerlessGraphEntry move; possibly `Trigger` trait too), then careful migration of fixtures.

### 2026-05-01 — Phase 1: relocate TaskEntry + ComputationGraphEntry

Mechanical relocation of two more inventory entry types from `cloacina/src/inventory_entries.rs` to `cloacina-workflow-plugin/src/inventory_entries.rs`. All four test gates green.

**Changes shipped:**

- `crates/cloacina-workflow-plugin/Cargo.toml`: added `cloacina-workflow = { path = ..., default-features = false }` dep so the leaf crate can reference `Task` and `TaskNamespace`.
- `crates/cloacina-workflow-plugin/src/inventory_entries.rs`:
  - Added `pub struct TaskEntry { namespace: fn() -> TaskNamespace, constructor: fn() -> Arc<dyn Task> }` with `inventory::collect!`.
  - Added `pub struct ComputationGraphEntry { name: &'static str, constructor: fn() -> ComputationGraphRegistration }` with `inventory::collect!`.
- `crates/cloacina-workflow-plugin/src/lib.rs`: re-exports both at the crate root.
- `crates/cloacina/src/inventory_entries.rs`: local definitions of `TaskEntry` and `ComputationGraphEntry` deleted. Re-exports from `cloacina_workflow_plugin::{TaskEntry, ComputationGraphEntry}` so existing engine paths (`crate::TaskEntry`, `cloacina::TaskEntry`, etc.) keep resolving.

The relocation is invisible to existing macro emissions (`cloacina::TaskEntry` and `crate::TaskEntry` paths resolve through the re-export). Test gates (workspace check + unit + integration sqlite) verify nothing regressed.

### Phase 2 — still pending

To complete T-C's main goal (strip per-macro emission and migrate fixtures), the next iteration needs to:

1. **Update macro emission paths to be cdylib-friendly.** `workflow_attr.rs`'s `task_inventory_entries` emits `cloacina::TaskEntry`, `cloacina::Task`, `cloacina::Context`, `cloacina::TaskError`, `cloacina::TaskNamespace`, `cloacina::retry::RetryPolicy`. In packaged mode (cdylibs that don't depend on `cloacina`), these don't resolve. Switch the emission to `cloacina_workflow_plugin::TaskEntry` (after this Phase 1 relocation) and `cloacina_workflow::{Task, Context, TaskError, TaskNamespace, retry::RetryPolicy}` (already cdylib-reachable).
2. **Un-gate `task_inventory_entries` from `cfg(not(feature = "packaged"))`** so packaged cdylibs collect their tasks too. Same for the workflow macro's similar gates.
3. **Complete `cloacina::package!()` shell macro** with real bodies for `get_task_metadata` (walk TaskEntry, derive workflow_name from namespace template), `execute_task` (look up task by id, dispatch to constructor), `get_graph_metadata` + `execute_graph` (walk ComputationGraphEntry — single entry per cdylib invariant).
4. **Strip per-macro `_ffi` emission** in `workflow_attr.rs` and `computation_graph/codegen.rs`. Compile workspace to surface dependents.
5. **Migrate in-tree fixtures**: add `cloacina::package!();` to each, drop `[[triggers]]` / `package_type` from package.toml. List in AC.
6. **TriggerEntry + TriggerlessGraphEntry relocation** can wait until a fixture exercises them.

Phase 1 is committable on its own (mechanical move with no behavior change, all tests green). Phase 2 is the substantive cut.
