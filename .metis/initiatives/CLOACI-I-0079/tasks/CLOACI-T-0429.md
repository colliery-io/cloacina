---
id: extract-computation-graph-types
level: task
title: "Extract computation graph types crate — decouple #[computation_graph] macro from full cloacina engine"
short_code: "CLOACI-T-0429"
created_at: 2026-04-06T22:24:22.193266+00:00
updated_at: 2026-04-06T22:26:22.575596+00:00
parent: CLOACI-I-0079
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0079
---

# Extract computation graph types crate — decouple #[computation_graph] macro from full cloacina engine

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0079]]

## Objective

The `#[computation_graph]` macro expands into code that references `cloacina::computation_graph::*`, forcing packaged CG plugins to depend on the full `cloacina` engine crate. This produces 60MB debug dylibs (vs 2.5MB for workflow dylibs) and causes the server to be SIGKILL'd when loading a CG package alongside a workflow package — the combined resident memory of the loaded CG dylib + the workflow compilation subprocess exceeds system limits.

Workflow packages don't have this problem because `#[workflow]` expands into code referencing `cloacina_workflow::*` — a thin dedicated crate. The CG macro needs the same pattern: a thin `cloacina-computation-graph` crate containing only the types the macro expansion needs.

## Problem

The macro in `crates/cloacina-macros/src/computation_graph/codegen.rs` generates code referencing these paths:

- `cloacina::computation_graph::InputCache` — cache for reactor input values
- `cloacina::computation_graph::types::serialize` — serialization helper
- `cloacina::computation_graph::SourceName` — accumulator source identifier
- `cloacina::computation_graph::GraphResult` — execution result type (Completed/Error)
- `cloacina::ComputationGraphRegistration` — registration struct for `ctor`

These are 5 types/functions. The full `cloacina` crate pulls in Diesel, Postgres, SQLite, PyO3, the entire DAL, the reconciler, the runner, tokio, etc. — none of which a packaged graph plugin needs.

## Acceptance Criteria

## Acceptance Criteria

- [ ] New crate `crates/cloacina-computation-graph/` containing `InputCache`, `SourceName`, `GraphResult`, `GraphError`, `serialize`/`deserialize`, and `ComputationGraphRegistration`
- [ ] `cloacina-macros` codegen references `cloacina_computation_graph::*` instead of `cloacina::computation_graph::*`
- [ ] `cloacina` re-exports from `cloacina-computation-graph` so embedded-mode code (`use cloacina::computation_graph::*`) is unchanged
- [ ] Packaged CG examples (`packaged-graph`, soak CG package) depend on `cloacina-computation-graph` instead of `cloacina`
- [ ] Packaged CG dylib size drops from ~60MB to ~2-5MB (debug build)
- [ ] Server can load CG + workflow packages simultaneously without being killed
- [ ] All existing tutorials, tests, and examples compile unchanged (embedded mode still uses `cloacina::computation_graph::*` via re-export)
- [ ] `angreal cloacina server-soak` passes with both CG and workflow packages

## Implementation Notes

### Parallel: `cloacina-workflow` for workflows

The workflow crate pattern is the model:
- `cloacina-workflow` contains `Context`, `Task`, `TaskError`, `#[task]`/`#[workflow]` attribute support types
- `cloacina-macros` workflow codegen references `cloacina_workflow::*`
- `cloacina` re-exports: `pub use cloacina_workflow;`
- Packaged workflow plugins depend on `cloacina-workflow` (2.5MB dylib), not `cloacina` (60MB)

### Types to extract

From `cloacina::computation_graph::types`:
- `InputCache`, `InputCacheSnapshot` — reactor input value cache
- `SourceName` — newtype for accumulator source names
- `GraphResult`, `GraphError` — execution result types
- `serialize()`, `deserialize()` — profile-aware serialization (JSON in debug, bincode in release)

From `cloacina` root:
- `ComputationGraphRegistration` — struct used by `ctor` for auto-registration

### What stays in `cloacina`

Everything the server/engine needs:
- `Accumulator` trait and runtime
- `Reactor`, `ReactiveScheduler`
- `EndpointRegistry`, auth policies
- `PackagingBridge` (FFI loading)
- DAL, checkpointing, health states

These are re-exported from `cloacina` which depends on `cloacina-computation-graph`.

### Macro changes

In `crates/cloacina-macros/src/computation_graph/codegen.rs`, the generated code currently emits:
```rust
cloacina::computation_graph::InputCache::new()
cloacina::computation_graph::types::serialize(&value)
cloacina::computation_graph::SourceName::new(source_name)
cloacina::computation_graph::GraphResult::completed(outputs)
```

After this change:
```rust
cloacina_computation_graph::InputCache::new()
cloacina_computation_graph::serialize(&value)
cloacina_computation_graph::SourceName::new(source_name)
cloacina_computation_graph::GraphResult::completed(outputs)
```

The macro already has the `is_test_env` pattern that switches between `crate::` and `cloacina::` paths — this would add a third case for the new crate name.

### Unblocks

- T-0404 (CG soak test) — server-soak can load CG + workflow packages without crash
- T-0405 (benchmarks) — meaningful benchmarks require production-sized dylibs

## Status Updates **[REQUIRED]**

**2026-04-06 — Types crate extracted and wired**
- Created `crates/cloacina-computation-graph/` with all types: InputCache, SourceName, GraphResult, GraphError, serialize/deserialize, CompiledGraphFn, ComputationGraphRegistration, global registry
- Added to workspace in Cargo.toml
- `cloacina` depends on and re-exports from `cloacina-computation-graph`
- types.rs and global_registry.rs in cloacina replaced with re-exports
- reactor.rs CompiledGraphFn replaced with re-export
- `cloacina-macros` codegen updated: external crates use `cloacina_computation_graph::*`, internal (cloacina crate) uses `crate::computation_graph::*`
- `packaged-graph` example switched from `cloacina` to `cloacina-computation-graph` dep
- Soak CG package in server_soak.py updated similarly
- `HOST_CRATES` in extraction.rs updated to include `cloacina-computation-graph`
- Dylib size: 60MB → 2.8MB (debug build)
- All workspace crates compile clean

**2026-04-06 — Server crash persists (separate root cause)**
- CG dylib now 2.8MB (same order as workflow's 2.5MB)
- Server still SIGKILL'd at "Step 5a: Registering tasks" when loading CG package after workflow
- Crash happens during `extract_task_metadata_from_library` — loading the CG dylib via fidius while workflow dylib was previously loaded/unloaded
- Root cause is NOT dylib size — it's a fidius/ctor/dlopen interaction issue when loading a second cdylib after the first was loaded and dropped
- This is a separate bug from the types crate extraction — filing as a new task
