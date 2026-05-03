---
id: reconciler-load-package-precedence
level: task
title: "Reconciler load_package precedence-pipeline restructure"
short_code: "CLOACI-T-0554"
created_at: 2026-05-03T13:26:01.000000+00:00
updated_at: 2026-05-03T13:26:01.000000+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#tech-debt"


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

*To be added during implementation.*
