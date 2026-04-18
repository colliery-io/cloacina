---
id: runtime-registry-unification
level: initiative
title: "Runtime Registry Unification — Eliminate #[ctor] Dependency"
short_code: "CLOACI-I-0096"
created_at: 2026-04-14T23:54:12.881539+00:00
updated_at: 2026-04-18T01:38:50.490394+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: runtime-registry-unification
---

# Runtime Registry Unification — Eliminate #[ctor] Dependency Initiative

## Context

Supersedes CLOACI-I-0095 (reverted). The goal of I-0095 was to unify Runtime into a single lookup path by snapshotting global registries at construction time. This failed because **`#[ctor]` initialization ordering across compilation units is not guaranteed on macOS** — `Runtime::new()` was called before some `#[ctor]`-registered tasks were visible in the global registry, producing an empty snapshot.

### What we learned (I-0095 post-mortem)

1. **`#[ctor]` ordering is unreliable.** The `#[ctor]` attribute (from the `ctor` crate, backed by `__attribute__((constructor))`) runs constructors before `main()`, but the ORDER across compilation units is platform-dependent and not guaranteed. On macOS with the integration test binary, `Runtime::new()` saw 0 tasks in globals even though 12 tasks were registered via `#[ctor]` moments later.

2. **Snapshot-at-construction doesn't work.** Because of (1), `Runtime::new()` cannot reliably snapshot all registered constructors. The old `from_global()` delegation worked because it reads globals lazily on every lookup, by which time all `#[ctor]`s have completed.

3. **The dual-path (`new()` vs `from_global()`) exists for a reason.** It's not just about test isolation — it's about `#[ctor]` timing. The delegation path handles late registration; the isolated path handles test independence.

4. **The real fix is removing `#[ctor]` entirely.** If tasks/workflows/CGs are registered explicitly (not via linker magic), then Runtime can snapshot reliably because registration happens at a well-defined point in program execution.

### Current state of global registries

- **Tasks, Workflows, Triggers**: covered by Runtime (3 registries), use `from_global()` delegation
- **Computation Graphs, Stream Backends**: NOT in Runtime (2 registries), accessed via global statics directly
- **Python ephemeral**: out of scope (4 registries, different lifecycle)
- **159 `#[serial]` annotations**: ~20 are registry-related (removable), ~139 are DB/process-related (separate concern)

## Goals & Non-Goals

**Goals:**
- Remove `#[ctor]` dependency from task, workflow, trigger, and computation graph registration
- Replace with explicit registration into Runtime at a well-defined point
- Extend Runtime to cover computation graph and stream backend registries
- Enable `Runtime::new()` to produce a complete, reliable snapshot (single code path)
- Reduce `#[serial]` annotations for registry-related tests

**Non-Goals:**
- Fixing DB-related `#[serial]` (shared fixture pattern — separate initiative)
- Python ephemeral registries (different lifecycle)
- Full crate decomposition

## Detailed Design

### Phase 1: Explicit registration model for embedded mode

Currently, `#[task]` and `#[workflow]` macros generate `#[ctor]` functions that call `register_task_constructor()` / `register_workflow_constructor()`. These push into global statics at binary load time.

**New model:**
- Macros generate a `pub fn register(runtime: &Runtime)` method instead of `#[ctor]`
- A top-level `register_all(runtime: &Runtime)` function is generated that calls all individual `register()` methods
- `DefaultRunner::new()` and `DefaultRunner::with_config()` call `register_all()` explicitly
- For backward compatibility: `#[ctor]` can still push into globals, but `Runtime::new()` doesn't snapshot — it starts empty and requires explicit registration
- `from_global()` becomes a convenience that calls `sync_from_global()` to copy whatever is in globals

### Phase 2: Extend Runtime with CG and stream backend fields

Same as I-0095 Phase 1 but without the snapshot:
- Add `computation_graphs` and `stream_backends` fields
- Add `register_computation_graph()`, `get_computation_graph()`, etc.
- ReactiveScheduler and reconciler use Runtime for CG lookup

### Phase 3: Update `#[computation_graph]` macro

- Generate `register(runtime: &Runtime)` instead of `#[ctor]`
- Reconciler calls explicit registration after loading packages

### Phase 4: Remove `#[serial]` for registry tests

- Tests create `Runtime::empty()` and explicitly register what they need
- No globals involved — fully parallel

## Alternatives Considered

- **Snapshot at construction (I-0095 approach)**: Failed. `#[ctor]` ordering is unreliable.
- **Lazy delegation (current `from_global()`)**: Works but means tests and prod exercise different code paths.
- **Thread-local staging area**: Considered but adds complexity. `#[ctor]` still has ordering issues.
- **Keep `#[ctor]` + add `sync_from_global()`**: Partial fix. The globals still exist as a parallel registry.

## Implementation Plan

### Updated design (2026-04-16)

Runtime becomes a single flat unified registry holding all 5 namespaces:
tasks, workflows, triggers, computation graphs, and stream backends. Every
entry is individually register-able and **unregister-able** by key. This is a
change from the earlier draft — workflows and triggers were treated as static
(register once, live forever), but the same dynamic lifecycle that CGs need
(add/remove at runtime, package unload) applies to workflows and triggers too.
Unifying the lifecycle simplifies the reconciler and enables hot-swap in
embedded mode.

Replace `#[ctor]` with the `inventory` crate: macros emit `inventory::submit!`
entries collected in a linker section and read lazily after `main()`. This
preserves the "annotate and it works" UX for embedded mode while eliminating
the ordering bug that sank I-0095. `Runtime::new()` iterates inventory at
construction to seed itself; the seed is just a convenience — nothing prevents
callers from `unregister_*` on any entry afterwards.

The `from_global()` / `use_globals` split goes away. Single code path: create
Runtime → seed from inventory (automatic) → optionally register/unregister
explicitly. Reconciler's package-unload path switches from a parallel "unregister
from global + registrar cleanup" dance to a uniform `runtime.unregister_*(key)`
for every namespace.

This is a pre-1.0 breaking change for embedded users: the macros still emit
auto-registration entries, but users that constructed `Runtime::new()` expecting
it to be truly empty (rather than inventory-seeded) will see a behavior change.
Documented in T5.

### Task breakdown

- **T1** Runtime unification — extend Runtime with CG + stream backend fields; add `register_*` and `unregister_*` for all 5 namespaces; remove `from_global()` / `use_globals`.
- **T2** Macro codegen flip — `#[task]`, `#[workflow]`, `#[trigger]`, `#[computation_graph]`, and the stream backend registration macro emit `inventory::submit!` instead of `#[ctor]`.
- **T3** Runtime seeding from inventory — `Runtime::new()` iterates inventory and populates. Drop the global static registries entirely.
- **T4** Reconciler + DefaultRunner wiring — package-unload paths switch to `runtime.unregister_*`; DefaultRunner constructs Runtime → seed → hand to executor.
- **T5** Cleanup & docs — remove `ctor` dep, remove `register_*_constructor` globals + `global_*_registry` modules, document the new contract and breaking change.

Dependency order is T1 → T2 → T3 → T4 → T5.
