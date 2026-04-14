---
id: extend-runtime-to-cover-all-global
level: initiative
title: "Extend Runtime to cover all global registries"
short_code: "CLOACI-I-0095"
created_at: 2026-04-13T12:06:36.329354+00:00
updated_at: 2026-04-14T12:40:01.875899+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: extend-runtime-to-cover-all-global
---

# Extend Runtime to cover all global registries Initiative

## Context

The `Runtime` struct provides isolated scoping for tasks, workflows, and triggers — enabling tests to run without stomping on global state. But 6 additional global static registries are NOT covered by `Runtime`, forcing 161 `#[serial]` annotations across 24 test files and making computation graph tests structurally unable to run in parallel.

Supersedes backlog task CLOACI-T-0484 (same objective, elevated to initiative due to scope).

## Review Finding References

EVO-002, EVO-008 (from architecture review `review/10-recommendations.md` REC-014)

## Goals & Non-Goals

**Goals:**
- Unify Runtime to a single lookup path — `new()` snapshots globals, no delegation/fallback
- Extend `Runtime` to encompass computation graph and stream backend registries
- Enable `ReactiveScheduler` to use `Runtime` instead of global statics
- Reduce `#[serial]` annotations across test files

**Non-Goals:**
- Full crate decomposition (separate effort, AR-2 from review)
- Moving Python ephemeral registries into Runtime (different lifecycle — scoped to builder context, drained on exit, not long-lived)
- DAL trait abstraction (CLOACI-T-0482)

## Current State: Global Registry Inventory

### Covered by Runtime (3):
1. `GLOBAL_TASK_REGISTRY` — `task.rs:637`
2. `GLOBAL_WORKFLOW_REGISTRY` — `workflow/registry.rs:36`
3. `GLOBAL_TRIGGER_REGISTRY` — `trigger/registry.rs:36`

### Target for Runtime (2 — high value):
4. `GLOBAL_COMPUTATION_GRAPH_REGISTRY` — `cloacina-computation-graph/src/lib.rs:289`
5. `GLOBAL_REGISTRY` (Stream Backend) — `computation_graph/stream_backend.rs:138`

### Out of scope — Python ephemeral (4):
6. `NODE_REGISTRY` — `python/computation_graph.rs:62` (drained on builder exit)
7. `ACCUMULATOR_REGISTRY` — `python/computation_graph.rs:101` (drained on builder exit)
8. `GRAPH_EXECUTORS` — `python/computation_graph.rs:463` (long-lived but Python-specific)
9. `PYTHON_TRIGGER_REGISTRY` — `python/trigger.rs:37` (drained by reconciler)

## Detailed Design

### Design principle: single code path

The current Runtime has two modes (`new()` = isolated, `from_global()` = delegate to globals) which means test and production code exercise different lookup paths. This is wrong — tests should exercise the same code as production.

**New model:**
- `#[ctor]` continues to push into global statics (unavoidable — runs before `main()`)
- `Runtime::new()` **snapshots** all globals into local maps at creation time. No delegation, no fallback. Lookups only check local maps.
- `Runtime::empty()` creates a completely empty runtime for test isolation
- Remove `use_globals` flag entirely — one lookup path for everyone
- Dynamic package loading (reconciler): after `dlopen` triggers `#[ctor]`, reconciler calls `runtime.register_*()` directly or `runtime.sync_from_global()` to pick up new entries
- Result: `get_task()`, `get_workflow()`, etc. have exactly one code path

### Phase 1: Unify Runtime lookup path + add CG/stream fields
- Remove `use_globals` flag from `RuntimeInner`
- `Runtime::new()` snapshots all 5 global registries (tasks, workflows, triggers, CGs, stream backends) into local maps
- Add `Runtime::empty()` for test isolation (no snapshot)
- Rename `from_global()` → deprecate or remove (replaced by `new()`)
- Add `computation_graphs: RwLock<HashMap<String, ComputationGraphConstructor>>` field
- Add `stream_backends: RwLock<StreamBackendRegistry>` field
- Add `register_computation_graph()`, `get_computation_graph()`, `register_stream_backend()`, `get_stream_backend()` methods
- All `get_*()` methods: check local maps only, no fallback branch
- Update all callers of `from_global()` → `new()`
- Update reconciler to call `runtime.register_*()` after loading packages (instead of relying on global fallback)

### Phase 2: Wire ReactiveScheduler + reconciler to use Runtime for CG lookup
- `load_graph()` looks up constructors via `runtime.get_computation_graph()` instead of `global_computation_graph_registry()`
- Reconciler calls `runtime.register_computation_graph()` after loading CG packages
- Stream backend lookup goes through Runtime

### Phase 3: Update `#[computation_graph]` macro codegen + remove `#[serial]`
- Currently generates `#[ctor]` that calls `register_computation_graph_constructor()` (global)
- Keep `#[ctor]` for binary-embedded graphs (same as tasks/workflows)
- `Runtime::new()` snapshot picks them up automatically
- Tests using `Runtime::empty()` + local registration bypass globals
- Remove `#[serial]` from tests that only needed it for CG/stream backend globals
- Verify parallel execution

## Alternatives Considered

- **Move everything to Runtime at once**: Rejected — Python ephemeral registries have a fundamentally different lifecycle (scoped to builder context). Forcing them into Runtime would add complexity without value.
- **Skip Runtime, just add mutexes**: Rejected — doesn't solve the `#[serial]` problem, just prevents data races. Tests still can't isolate.
- **Full crate decomposition first**: Rejected — too expensive as a prerequisite. Runtime extension is a pragmatic step that delivers value independently.

## Implementation Plan

Phase 1 → Phase 2 → Phase 3, sequentially. Phase 1 is the largest (unifying the lookup path + adding fields). Phase 2 wires CG lookup through Runtime. Phase 3 removes `#[serial]`. Full test suite (`angreal cloacina all`) validated at each step.
