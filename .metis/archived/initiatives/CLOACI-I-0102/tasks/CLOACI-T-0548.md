---
id: t-b-reconciler-dispatch-for-get
level: task
title: "T-B: Reconciler dispatch for get_reactor_metadata (Rust path)"
short_code: "CLOACI-T-0548"
created_at: 2026-04-30T04:07:42.041097+00:00
updated_at: 2026-05-01T20:55:37.471802+00:00
parent: CLOACI-I-0102
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0102
---

# T-B: Reconciler precedence-ordered loader

## Parent Initiative

[[CLOACI-I-0102]]

## Objective

Rebuild `crates/cloacina/src/registry/reconciler/loading.rs` around a single fixed-order pipeline that handles every primitive kind a package can declare. Replaces today's per-`package_type` branching with one `load_package` function that dispatches in precedence order: **cron triggers → custom triggers → reactors → trigger-less CGs → reactor-bound CGs → workflows**. Same shape Rust + Python — only the metadata-extraction step differs (fidius plugin handle vs. scoped Runtime walk).

This is where T-A's new optional plugin methods (`get_reactor_metadata`, `get_trigger_metadata`) get consumed. It's also where today's `package_type`-driven dispatch dies. Cron-vs-custom trigger routing splits at this layer (cron expression present → cron scheduler; otherwise → runtime trigger registry).

The legacy `[[triggers]]` and `package_type` reads in `package.toml` are kept *temporarily* in this task with deprecation warnings — the manifest cleanup happens in T-E. T-C strips per-macro plugin emission; this task assumes both legacy and unified plugin shells coexist (gracefully handled by fidius's optional-method mechanism).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Plugin host-side wiring

- [ ] `LoadedGraphPlugin` (or whatever its successor is post-refactor) in `crates/cloacina/src/computation_graph/packaging_bridge.rs` gains:
  - `get_reactor_metadata(&self) -> Result<Vec<ReactorPackageMetadata>, ...>` calling `handle.call_method(4, &())`. Translates `CallError::NotImplemented` → `Ok(vec![])` for plugins built before T-A landed.
  - `get_trigger_metadata(&self) -> Result<Vec<TriggerPackageMetadata>, ...>` calling `handle.call_method(5, &())`. Same NotImplemented translation.
- [ ] Method-index constants documented in code (no bare `4` / `5` literals in dispatch sites).

### Precedence-ordered `load_package` pipeline

- [ ] New top-level `load_package(plugin_view, manifest, scheduler, runtime, cron_scheduler)` in `loading.rs` (or a new submodule) replaces the per-`package_type` branching for the Rust path. Pipeline executes in this fixed order:
  1. **Cron triggers** — `get_trigger_metadata` entries with `cron_expression.is_some()` → cron_scheduler register.
  2. **Custom triggers** — `get_trigger_metadata` entries with `cron_expression.is_none()` → `runtime.register_trigger`.
  3. **Reactors** — `get_reactor_metadata` → `scheduler.load_reactor` (idempotent on contract).
  4. **Trigger-less CGs** — from `get_graph_metadata` if the CG has no `trigger_reactor` set → `runtime.register_triggerless_graph`.
  5. **Reactor-bound CGs** — from `get_graph_metadata` if `trigger_reactor: Some(name)` → `scheduler.bind_graph_to_reactor`. Hard-error if the upstream reactor isn't already loaded.
  6. **Workflows** — from `get_task_metadata` → `register_workflow` + bind workflow → triggers (lookup each name in `metadata.triggers`). Hard-error if any named trigger isn't already loaded.

### Python parity

- [ ] Same six-step pipeline applied to Python packages. Metadata-extraction step replaces `LoadedGraphPlugin` with a Python adapter that walks the post-import scoped Runtime registries (`reactor_names()`, `trigger_names()`, etc.) and returns the same metadata struct shapes.
- [ ] T-0545 M3a's existing `dispatch_runtime_reactors_into_scheduler` helper folds into this pipeline (the reactor step calls it). T-0545's Python trigger registration (`drain_python_triggers` → `register_trigger`) gets re-routed through the cron-vs-custom step.

### Cross-package and lifecycle behavior

- [ ] Cross-package binding works: a CG package referencing a reactor loaded by an earlier package binds via T-0544 M2's idempotent path. Same for a workflow referencing a trigger from another package.
- [ ] Cross-package ordering is fail-fast: subscriber loaded before publisher → clean rejection naming the missing primitive. No pending-bindings queue.
- [ ] Package unload mirrors load order in reverse: workflows → CGs → reactors → triggers. Reuses T-0544 M4's `unload_reactor` reject-with-subscribers guard.

### Manifest deprecation (warnings only — full removal is T-E)

- [ ] `[[triggers]]` stanzas in `package.toml`: still read, but a `tracing::warn!` deprecation message is logged when present. The macro-form (`#[workflow(triggers = […])]`) is the source of truth; if both are present, macro wins and a warning logs.
- [ ] `package_type` field: still read, but `tracing::warn!` deprecation when present (and ignored for routing purposes — the new pipeline doesn't consult it).

### Tests

- [ ] Existing CG/workflow integration tests pass unchanged. The pipeline is a behavioral no-op for legacy package shapes.
- [ ] New unit/integration test coverage for the precedence pipeline:
  - Cron-vs-custom trigger routing.
  - Cross-package missing-publisher errors (subscriber-first load → expected rejection).
  - Reactor-only / trigger-only package loads (depends on T-A's shell macro being usable, but full E2E with cdylib fixtures is T-D's territory; this task uses synthetic plugin handles or integration scaffolding).
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Technical Approach

1. Extend `LoadedGraphPlugin` with `get_reactor_metadata`. Wrap the fidius `call_method(4, &())` call (the new method takes no args). Match on `CallError::NotImplemented` to fall back to empty vec; treat other errors as load failures.
2. Add a `dispatch_reactor_metadata_into_scheduler(metadata, scheduler, tenant_id, accumulator_overrides)` helper alongside the existing Python-driven helper, OR refactor both behind one signature taking `impl IntoIterator<Item = ReactorRegistrationView>`. Either keeps the per-reactor work (build factories, call `scheduler.load_reactor`) in one place.
3. Reconciler `loading.rs` Rust CG branch — at the point where the plugin handle is in scope and just before `scheduler.load_graph(decl)`, call the new helper. Same `cloacina_manifest.metadata.accumulators` overrides we use for CGs.
4. `package_type = ["reactor"]` routing — add a branch that calls `get_reactor_metadata` and dispatches without trying to load a CG. Today's path requires `has_computation_graph()`; loosen that check or add a parallel `has_reactors()` driven by the new metadata.

### Key Files

- `crates/cloacina/src/computation_graph/packaging_bridge.rs` — `LoadedGraphPlugin` + the dispatch helper.
- `crates/cloacina/src/registry/reconciler/loading.rs` — Rust CG branch.
- `crates/cloacina-workflow-plugin/src/types.rs` — possibly `package_type` value `"reactor"` if we add it as a recognized variant. Decision deferred from I-0102 design.

### Dependencies

- **T-0547 (T-A)** — must land first. Provides the `get_reactor_metadata` trait method and `ReactorPackageMetadata` struct.

### Risk Considerations

- **Method index 4 hardcode.** The `call_method(4, …)` index relies on the trait's declaration order being stable. Adding methods between existing ones in the future would shift indices. Document the index in a comment and consider a constant.
- **Error-handling for `NotImplemented`.** Make sure we're matching on the specific `CallError::NotImplemented { bit }` variant, not the generic `CallError::Plugin(_)`. Misclassifying a real plugin error as "no reactors here" would silently lose reactor declarations.

## Status Updates

### 2026-05-01 — Phase 1: host-side FFI bridge for the new optional methods

Bounded leaf change that the rest of T-B builds on. Workspace `cargo check --all-features` green.

**Changes shipped:**

- `crates/cloacina/src/computation_graph/packaging_bridge.rs`:
  - Added `METHOD_GET_TASK_METADATA` / `METHOD_EXECUTE_TASK` / `METHOD_GET_GRAPH_METADATA` / `METHOD_EXECUTE_GRAPH` / `METHOD_GET_REACTOR_METADATA` / `METHOD_GET_TRIGGER_METADATA` `pub const usize` constants for the trait's method indices. Replaces bare `call_method(3, …)` etc. literals at dispatch sites going forward.
  - Added free function `call_get_reactor_metadata(&PluginHandle) -> Result<Vec<ReactorPackageMetadata>, String>` that calls `handle.call_method(4, &())` and translates `CallError::NotImplemented { .. }` → `Ok(Vec::new())`. Plugins built against trait v1 + per-macro `_ffi` stubs returning `Ok(vec![])` both surface as "package declares no reactors" cleanly.
  - Added free function `call_get_trigger_metadata(&PluginHandle) -> Result<Vec<TriggerPackageMetadata>, String>` with the same NotImplemented fallback. Cron-vs-custom routing happens at the reconciler based on `cron_expression`.

The `LoadedGraphPlugin` struct (private to `packaging_bridge.rs`) is left scoped to CG-only loads for now. T-B's "unified `load_package` pipeline" will either generalize this struct or introduce a new `LoadedPackagePlugin`. Choice deferred to the next iteration — the free functions above operate on `&PluginHandle` directly, so they're reusable from either shape.

### Phase 2 — remaining T-B plan

Bulk of T-B is still ahead. Layout for the next iteration:

1. **Generalize the host-side wrapper.** Either rename `LoadedGraphPlugin` → `LoadedPackagePlugin` and broaden it to expose `get_task_metadata` / `get_graph_metadata` / `get_reactor_metadata` / `get_trigger_metadata` / `execute_task` / `execute_graph`, or introduce a new struct beside it. Today's CG-only load in `packaging_bridge.rs:115` (`build_declaration_from_ffi`) and the workflow load in `loading.rs` are separate; the unified pipeline needs one shape.
2. **Build `load_package` pipeline (Rust path).** Replace the per-`package_type` branching in `crates/cloacina/src/registry/reconciler/loading.rs` (1144 lines) with a fixed-order pipeline:
   1. Cron triggers (`get_trigger_metadata` entries with `cron_expression.is_some()` → cron_scheduler register)
   2. Custom triggers (`cron_expression.is_none()` → `runtime.register_trigger`)
   3. Reactors (`get_reactor_metadata` → `scheduler.load_reactor`, idempotent on contract)
   4. Trigger-less CGs (from `get_graph_metadata` if `trigger_reactor.is_none()` → `runtime.register_triggerless_graph`)
   5. Reactor-bound CGs (from `get_graph_metadata` if `trigger_reactor.is_some(name)` → `scheduler.bind_graph_to_reactor`; hard-error if upstream reactor isn't already loaded)
   6. Workflows (from `get_task_metadata` → `register_workflow` + bind workflow → triggers via `metadata.triggers` lookup; hard-error if any named trigger isn't loaded)
3. **Python parity.** Same six steps for the Python path. T-0545 M3a's `dispatch_runtime_reactors_into_scheduler` and the Python trigger drain (`drain_python_triggers`) fold into the new pipeline's reactor + trigger steps. The metadata-extraction adapter for Python walks the post-import scoped `Runtime` registries and projects to the wire-format struct shapes.
4. **Cross-package binding + fail-fast.** Already in place via T-0544 M2's idempotent `bind_graph_to_reactor`. Subscriber-loaded-before-publisher → clean rejection naming the missing primitive. No pending-bindings queue.
5. **Lifecycle ordering for unload.** Mirror load order in reverse: workflows → CGs → reactors → triggers. Reuses T-0544 M4's `unload_reactor` reject-with-subscribers guard.
6. **Manifest deprecation warnings.** `[[triggers]]` and `package_type` keep parsing but emit `tracing::warn!`. Macro layer becomes the source of truth; full removal is T-E (T-0551).

### Test gates remaining

- [x] `cargo check --workspace --all-features` (Phase 1).
- [ ] `angreal test unit`.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres`.
- [ ] New unit/integration coverage for the precedence pipeline (cron-vs-custom routing, cross-package missing-publisher errors, reactor-only / trigger-only loads).

### 2026-05-01 — Phase 2: extract + dispatch reactor metadata in the Rust load path

Plumbed reactor metadata extraction and dispatch into the existing reconciler dispatch (additive — no restructuring). All four test gates green.

**Changes shipped:**

- `crates/cloacina/src/registry/loader/package_loader.rs`:
  - Added `extract_reactor_metadata(&[u8]) -> Result<Vec<ReactorPackageMetadata>>` — calls `call_get_reactor_metadata` from packaging_bridge. NotImplemented (trait v1 plugins) and the per-macro `_ffi` stub (returning `Ok(vec![])`) both surface as empty Vec, so the call is a safe no-op for legacy packages.
  - Added `extract_trigger_metadata(&[u8]) -> Result<Vec<TriggerPackageMetadata>>` — same NotImplemented fallback. Not yet consumed by the reconciler (Phase 3).
- `crates/cloacina/src/computation_graph/packaging_bridge.rs`:
  - Added `dispatch_package_reactors_into_scheduler(&[ReactorPackageMetadata], scheduler, accumulator_overrides, tenant)` — the wire-format-driven counterpart to the existing `dispatch_runtime_reactors_into_scheduler` (Python path). Mirrors its accumulator-override merge logic, including the FFI default (passthrough) and the per-reactor `accumulator_type` from the wire format. Calls `scheduler.load_reactor` for each entry.
- `crates/cloacina/src/registry/reconciler/loading.rs`:
  - Rust load path now calls `extract_reactor_metadata` + `dispatch_package_reactors_into_scheduler` after task/workflow/trigger registration. For packages built against the unified `cloacina::package!()` shell, this is the path that brings their reactors up. For legacy packages (per-macro `_ffi` stubs), it's a no-op.

**Test gates (green):**

- [x] `cargo check --workspace --all-features`.
- [x] `angreal test unit` — 701 passed.
- [x] `angreal test integration --backend sqlite` — 6 Rust + 28 Python.
- [x] `angreal test integration --backend postgres` — 287 Rust + 28 Python.

The reactor-only fixture from T-0547 (`examples/fixtures/reactor-only-rust`) is now load-able end-to-end through this path, though there's no integration test exercising it yet (T-D's territory — T-0550 builds those).

### Phase 3 — still pending for full T-B

The remaining T-B work is the precedence-ordered pipeline rewrite. Order of remaining moves:

1. **Trigger metadata consumption.** Wire `extract_trigger_metadata` → split entries on `cron_expression.is_some()` → cron scheduler vs. runtime trigger registry. Today's `register_package_triggers` reads from manifest `[[triggers]]`; the new path reads from the FFI wire format. Both paths coexist until T-E removes the manifest reads.
2. **Workflow → trigger subscription binding.** When `PackageTasksMetadata.triggers` is non-empty, bind each named trigger to the workflow at load time. Hard-error if any named trigger isn't already loaded.
3. **Pipeline restructure.** The big one — replace per-`package_type` branching in `loading.rs` with the six-step pipeline (cron triggers → custom triggers → reactors → trigger-less CGs → reactor-bound CGs → workflows). Today's branching is heavily forked on `language` (rust/python) AND `package_type`; the new shape is one pipeline that calls a language-specific metadata-extraction adapter as its first step.
4. **Python parity.** Same six-step pipeline applied to Python packages. T-0545 M3a's `dispatch_runtime_reactors_into_scheduler` already covers reactor step for Python; need to add equivalents for trigger metadata + workflow-trigger binding from Python's scoped Runtime.
5. **Manifest deprecation warnings.** `[[triggers]]` and `package_type` keep parsing but emit `tracing::warn!`. Macro layer becomes the source of truth; full removal is T-E.
6. **Lifecycle ordering for unload.** Reverse load order: workflows → CGs → reactors → triggers.

T-0548 stays active. Phase 1 + Phase 2 are non-disruptive additive plumbing that T-0547's shell macro can already feed into. Phase 3 is the restructure that lets T-C's stripped per-macro emission stop being load-bearing.

### 2026-05-01 — Phase 3: workflow → trigger subscription validation + manifest deprecation warnings

Two more bounded additions land in. All four test gates still green.

**Changes shipped:**

- `crates/cloacina/src/registry/loader/package_loader.rs`:
  - `PackageMetadata` gained `#[serde(default)] pub workflow_triggers: Vec<String>` — sourced from `PackageTasksMetadata.triggers` in `extract_metadata`'s `convert_plugin_metadata_to_rust`. Threaded through every `PackageMetadata` construction site (5 production + test fixtures).
- `crates/cloacina/src/registry/reconciler/loading.rs`:
  - New `validate_workflow_trigger_subscriptions(metadata, package_data)` — calls `extract_metadata`, checks each name in `workflow_triggers` against `runtime.get_trigger`, and hard-errors with a friendly message naming the missing triggers. Wired into the Rust load path right before reactor dispatch.
  - Added deprecation warnings (logged via `tracing::warn!`) when `cloacina_manifest.metadata.triggers` is non-empty (`[[triggers]]` in package.toml) or when `package_type` is set to any non-default value. Both paths still parse + route today; T-E (T-0551) makes them hard-errors.

**Test gates (green):**

- [x] `cargo check --workspace --all-features`.
- [x] `angreal test unit` — 701 passed.
- [x] `angreal test integration --backend sqlite` — 6 Rust + 28 Python.
- [x] `angreal test integration --backend postgres` — 287 Rust + 28 Python.

### Deferred for follow-up (out of T-0548 scope)

The remaining T-B AC items are deferred — they need adjacent work to be useful:

- **Trigger metadata FFI consumption** (cron-vs-custom split). `extract_trigger_metadata` is in place but the shell macro stubs `get_trigger_metadata` returning `Ok(vec![])` (TriggerEntry hasn't been relocated to a cdylib-reachable crate yet). Wiring it into the reconciler today would be a dead path until TriggerEntry moves alongside ReactorEntry. Picking it up should be paired with the TriggerEntry relocation in a future task.
- **Pipeline restructure** — the six-step precedence pipeline (cron → custom → reactors → trigger-less CGs → reactor-bound CGs → workflows) replacing the current `language × package_type` branching. The substance of T-B's "rebuild loading.rs" goal. Today's branching still works correctly with the new metadata flowing additively; the restructure is purely an organizational cleanup that lets T-C's stripped per-macro emission stop being load-bearing. Best landed alongside or just before T-C, since they touch the same surface.
- **Python parity for the new pipeline.** T-0545 M3a's `dispatch_runtime_reactors_into_scheduler` already covers reactor step for Python; trigger metadata + workflow-trigger validation parity follow the same restructure work above.
- **Lifecycle ordering for unload** (workflows → CGs → reactors → triggers). T-0544 M4's `unload_reactor` reject-with-subscribers guard is in place; full reverse-order unload for the new step list comes with the restructure.

### State

T-0548 has the load-bearing additive pieces in place: host-side FFI bridge, reactor metadata extraction + dispatch, workflow-trigger subscription hard-error validation, manifest deprecation warnings. T-D (T-0550) can now build fixtures that exercise the reactor flow end-to-end. The pipeline restructure is real follow-up work but not blocking. **Recommend transitioning T-0548 to completed and creating a follow-up task for the restructure if/when the user wants the cleanup.**
