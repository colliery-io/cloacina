---
id: extend-runtime-to-cover-all-global
level: initiative
title: "Extend Runtime to cover all global registries"
short_code: "CLOACI-I-0095"
created_at: 2026-04-13T12:06:36.329354+00:00
updated_at: 2026-04-13T12:06:36.329354+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


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
- Extend `Runtime` to encompass computation graph and stream backend registries
- Enable `ReactiveScheduler` to use `Runtime` instead of global statics
- Make `#[ctor]` auto-registration opt-in so tests can bypass globals
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

### Phase 1: Add computation graph + stream backend fields to Runtime
- Add `computation_graphs: RwLock<HashMap<String, ComputationGraphConstructor>>` to `Runtime`
- Add `stream_backends: RwLock<StreamBackendRegistry>` to `Runtime`
- Add `register_computation_graph()`, `get_computation_graph()`, `register_stream_backend()` methods
- `Runtime::from_global()` delegates to global registries on miss (same pattern as tasks/workflows)

### Phase 2: Update ReactiveScheduler to use Runtime
- `ReactiveScheduler` currently receives an `EndpointRegistry` (not global) but graph loading goes through `GLOBAL_COMPUTATION_GRAPH_REGISTRY` via the reconciler
- Change `load_graph()` to accept a `&Runtime` reference for graph constructor lookup
- Update reconciler to pass Runtime through

### Phase 3: Update `#[computation_graph]` macro codegen
- Currently generates `#[ctor]` that calls `register_computation_graph_constructor()` (global)
- Add opt-in: `#[ctor]` registers to a thread-local staging area (like tasks/workflows)
- `Runtime::from_global()` consumes staged registrations
- Tests using `Runtime::new()` bypass globals entirely

### Phase 4: Remove `#[serial]` annotations
- Identify tests that are serial only because of computation graph / stream backend globals
- Convert to use `Runtime::new()` with local registrations
- Verify parallel execution doesn't break

## Alternatives Considered

- **Move everything to Runtime at once**: Rejected — Python ephemeral registries have a fundamentally different lifecycle (scoped to builder context). Forcing them into Runtime would add complexity without value.
- **Skip Runtime, just add mutexes**: Rejected — doesn't solve the `#[serial]` problem, just prevents data races. Tests still can't isolate.
- **Full crate decomposition first**: Rejected — too expensive as a prerequisite. Runtime extension is a pragmatic step that delivers value independently.

## Implementation Plan

Phase 1 → Phase 2 → Phase 3 → Phase 4, sequentially. Each phase is independently testable and shippable. Phase 3 (macro codegen) carries the most risk and should be done carefully with full test suite validation at each step.
