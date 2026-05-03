---
id: reconciler-load-package-precedence
level: task
title: "Reconciler load_package precedence-pipeline restructure"
short_code: "CLOACI-T-0554"
created_at: 2026-05-03T13:26:01+00:00
updated_at: 2026-05-03T14:12:49.012550+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/active"


exit_criteria_met: false
initiative_id: NULL
---

# Reconciler load_package precedence-pipeline restructure

## Objective

I-0102 follow-up. T-B (T-0548) Phase 3 deferred this. Replace the per-`language × package_type`-style branching in `crates/cloacina/src/registry/reconciler/loading.rs` (~1100 lines) with a single fixed-order pipeline that handles every primitive a package can declare in precedence order:

1. **Cron triggers** — `get_trigger_metadata` entries with `cron_expression.is_some()` → cron scheduler register.
2. **Custom triggers** — `cron_expression.is_none()` → `runtime.register_trigger`.
3. **Reactors** — `get_reactor_metadata` → `scheduler.load_reactor` (idempotent on contract).
4. **Trigger-less CGs** — `get_graph_metadata` if `trigger_reactor.is_none()` → `runtime.register_triggerless_graph`.
5. **Reactor-bound CGs** — `get_graph_metadata` if `trigger_reactor.is_some(name)` → `scheduler.bind_graph_to_reactor`. Hard-error if upstream reactor isn't already loaded.
6. **Workflows** — `get_task_metadata` → `register_workflow` + bind workflow → triggers (lookup each name in `metadata.triggers`). Hard-error if any named trigger isn't already loaded.

Same shape Rust + Python — only the metadata-extraction step differs (fidius `PluginHandle` vs. scoped `Runtime` walk).

## Backlog Item Details

### Type
- [x] Tech Debt — organizational cleanup; behavior-equivalent today

### Priority
- [x] P3 — Low (nice to have, no behavior change)

### Technical Debt Impact
- **Current Problems**: Today's branching is heavily forked on `language` (rust/python) AND on what's effectively `has_computation_graph` / `has_workflow` derived from the manifest. New primitive kinds (e.g., the trigger flow restored by T-0553) end up bolted on rather than plugged in. The reactor metadata extraction was added additively in T-B Phase 2; it works but doesn't fit the prior shape cleanly.
- **Benefits of Fixing**: Single point of dispatch makes future primitive additions trivial. Mirrored Rust/Python pipelines reduce duplicated logic. Cross-package contract mismatch errors get a cleaner home (currently `dispatch_package_reactors_into_scheduler` has no place to validate subscriber accumulator names against a bound reactor).
- **Risk Assessment**: Medium — high blast radius (~1100 LOC touched) but covered by integration tests on both backends. Restructure should be behavior-equivalent for legacy packages.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

### Pipeline structure (Rust path)

- [ ] New top-level `load_package(plugin_view, manifest, scheduler, runtime, cron_scheduler)` function in `loading.rs` (or a new submodule) replaces the per-branch dispatch. Pipeline executes the six steps above in fixed order.
- [ ] Each step is its own helper (`load_cron_triggers`, `load_custom_triggers`, `load_reactors`, `load_triggerless_cgs`, `load_reactor_bound_cgs`, `load_workflows`) for unit-testability.

### Python parity

- [ ] Same six-step pipeline applied to Python packages. Metadata-extraction step replaces `LoadedGraphPlugin` with a Python adapter that walks the post-import scoped Runtime registries (`reactor_names()`, `trigger_names()`, etc.) and returns the same metadata struct shapes.
- [ ] T-0545 M3a's existing `dispatch_runtime_reactors_into_scheduler` helper folds into this pipeline (the reactor step calls it). Python trigger registration (`drain_python_triggers` → `register_trigger`) gets re-routed through the cron-vs-custom step.

### Cross-package and lifecycle behavior

- [ ] Cross-package binding works: a CG package referencing a reactor loaded by an earlier package binds via T-0544 M2's idempotent path. Same for a workflow referencing a trigger from another package.
- [ ] Cross-package ordering is fail-fast: subscriber loaded before publisher → clean rejection naming the missing primitive. No pending-bindings queue.
- [ ] Cross-package contract mismatch: subscriber declares incompatible accumulator names → load fails with a clear error naming offending package + missing accumulator(s).
- [ ] Package unload mirrors load order in reverse: workflows → CGs → reactors → triggers. Reuses T-0544 M4's `unload_reactor` reject-with-subscribers guard.

### Test gates

- [ ] All existing CG/workflow integration tests pass unchanged. The pipeline is a behavioral no-op for legacy package shapes.
- [ ] New unit/integration coverage for the precedence pipeline:
  - Cron-vs-custom trigger routing.
  - Cross-package missing-publisher errors (subscriber-first load → expected rejection).
  - Cross-package contract mismatch errors.
  - Reactor-only / trigger-only / mixed package loads.
- [ ] `cargo check --workspace --all-features` green.
- [ ] `angreal test unit` green.
- [ ] `angreal test integration --backend sqlite` + `--backend postgres` green.

## Implementation Notes

### Dependencies

- I-0102 closure (T-0546–T-0551) — done.
- **T-0552 (TriggerEntry relocation)** — not strictly required, but the trigger steps (1, 2) only do useful work once trigger metadata flows. Reasonable to do this either way; the pipeline stays valid even when those steps are no-ops.
- **T-0553 (daemon trigger restoration + T-D fixtures)** — natural pairing. T-0553's cross-package contract mismatch test fits more cleanly once this pipeline lands.

### Technical Approach

1. Extract one helper per step (`load_cron_triggers`, etc.) from the current branched code. Each takes the plugin view + scheduler/runtime/cron-scheduler handles + tenant context.
2. Wire them in the fixed precedence order at the top of `load_package`.
3. Remove the per-language branching that gates "is this CG or workflow" — instead, each step is a no-op when the package doesn't declare that primitive (e.g., `get_reactor_metadata` returns empty Vec).
4. For Python parity, write a Python adapter that walks the scoped Runtime and produces the same wire-format struct shapes that the Rust path's FFI extraction produces. The pipeline body becomes language-independent.
5. Lift cross-package contract validation out of `dispatch_package_reactors_into_scheduler` (which doesn't currently validate subscriber accumulators) into the reactor-bound CG step.

### Risk Considerations

- **Behavior equivalence vs. legacy packages.** Existing in-tree fixtures rely on today's dispatch order. The restructure must be a behavioral no-op for them — the existing CG/workflow integration tests are the safety net.
- **Python adapter shape.** The metadata struct shapes are wire-format types from `cloacina-workflow-plugin`. The Python adapter generates them from in-memory Runtime state, not from FFI. Need to think about whether to introduce a unifying trait or just have two metadata-extraction adapters that feed the same downstream pipeline.
- **Diff size.** ~1100-line file restructure. Plan for a careful, well-staged commit history (per-step helper extraction first, then pipeline assembly, then deletion of the old branching code).

## Status Updates

### 2026-05-03 — Orientation + scoped plan

Current `loading.rs` (~1176 lines) `load_package` is ~540 LOC with three branches: Rust path, Python workflow path, Python CG path, plus a separate Step 7 CG routing that forks on language again. Today's effective ordering: tasks → workflows → reactors → CG, which doesn't match the target precedence pipeline (cron/custom triggers → reactors → CGs → workflows).

**Scoped plan for this session:**

Phase 1:
- Define `PackageLoadView` carrying wire-format metadata.
- Extract per-step helper methods on `RegistryReconciler` (six helpers).
- Rewrite Rust path's `load_package` body to call helpers in fixed precedence order.
- Python paths kept as-is for this iteration; the helpers are language-agnostic so a future Python adapter can wire them in.

Phase 2 (deferred):
- Python metadata-extraction adapter producing the same `PackageLoadView`.
- Cross-package contract mismatch validation in `load_reactor_bound_cgs`.
- Reverse-order unload pipeline.

### 2026-05-03 — Phase 1 done

Rust path now runs through the precedence-ordered pipeline. All four test gates green (cargo check, unit 701, integration sqlite, integration postgres 290 + 28 Python).

**Changes shipped:**

- `crates/cloacina/src/registry/reconciler/loading.rs`:
  - New `PackageLoadView` struct carrying `tasks: PackageMetadata`, `triggers: Vec<TriggerPackageMetadata>`, `reactors: Vec<ReactorPackageMetadata>`, `graph: Option<GraphPackageMetadata>`.
  - New `build_view_rust(library_data) -> PackageLoadView` extracts all four metadata kinds via fidius FFI (existing `extract_metadata` / `extract_trigger_metadata` / `extract_reactor_metadata` / `extract_graph_metadata`).
  - Six step helpers added on `RegistryReconciler`:
    - `step_load_cron_triggers` — filters cron-shaped trigger entries; logs count (no-op pending T-0553's daemon restoration).
    - `step_load_custom_triggers` — filters non-cron entries and validates each against the runtime registry (current behavior preserved).
    - `step_load_reactors` — uses `dispatch_package_reactors_into_scheduler`.
    - `step_load_triggerless_cgs` — no-op (TriggerlessGraphEntry inventory walk happens at task-invocation time, not reconciler time).
    - `step_load_reactor_bound_cgs` — handles the (single) graph from `view.graph`; merges manifest accumulator overrides; `build_declaration_from_ffi`; `scheduler.load_graph`. Took over the Step 7 Rust branch.
    - `step_load_workflows` — wraps the existing `register_package_tasks` + `register_package_workflows` + `validate_workflow_trigger_subscriptions` calls.
  - Rust path of `load_package` rewritten to call the six helpers in fixed precedence order: cron triggers → custom triggers → reactors → trigger-less CGs → reactor-bound CG → workflows.
  - The original Step 7 CG routing now only handles the **Python** CG path; Rust CG handling moved into the unified pipeline. Detection: if `rust_graph_name.is_some()` from the pipeline, Step 7 short-circuits.

**Behavior equivalence:** verified by all integration tests passing unchanged. The Rust load order shifted from "tasks → workflows → triggers → reactors → CG" to "triggers → reactors → CG → workflows", but legacy fixtures don't depend on the prior order (validation that fails on missing triggers still runs, accumulator dispatch still works, CG registers before workflow constructor needs it). Step 5 (reactor-bound CG) now happens BEFORE step 6 (workflow registration) — fine because the CG's reactor binding doesn't reference the workflow.

### Phase 2 — still deferred

Python metadata-extraction adapter, cross-package contract mismatch validation, reverse-order unload pipeline. Python paths in `load_package` kept as-is (they don't yet route through the unified pipeline).

The structural restructure goal of T-0554 — "single dispatch point for the precedence-ordered pipeline" — is achieved for the Rust path. Python parity is a clean follow-up.
