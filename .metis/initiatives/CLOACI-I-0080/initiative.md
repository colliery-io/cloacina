---
id: computation-graph-packaging-build
level: initiative
title: "Computation Graph Packaging — Build, Load, and Execute via Reconciler"
short_code: "CLOACI-I-0080"
created_at: 2026-04-05T16:10:21.412415+00:00
updated_at: 2026-04-05T18:18:52.630826+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: computation-graph-packaging-build
---

# Computation Graph Packaging — Build, Load, and Execute via Reconciler Initiative

## Context

The computation graph system works in embedded mode (tutorials 07-10 prove this). But the full packaging and server-mode execution path has never been implemented or tested.

**Critical finding from code review:** The existing workflow packaging does NOT use `#[ctor]`. Packaged workflows use **fidius FFI** — the `#[workflow]` macro generates FFI exports under `#[cfg(feature = "packaged")]`, and the reconciler loads the `.so` via `fidius_host::load_library()` + `PluginHandle::call_method()`. The `#[ctor]` path is embedded-mode only.

This means computation graph packaging needs the same fidius FFI approach:

### Workflow packaging flow (existing, working):
```
#[workflow(features=["packaged"])] macro
  → generates _ffi module with CloacinaPlugin impl
  → get_task_metadata() returns PackageTasksMetadata
  → execute_task(request) runs task in plugin's own tokio runtime
  → fidius_plugin_registry!() emits C ABI symbol
  → cargo build --lib → cdylib with fidius registry
  → .cloacina archive (bzip2 tar)
  → reconciler loads via fidius_host::load_library()
  → PackageLoader::extract_metadata() → PluginHandle::call_method(0)
  → TaskRegistrar registers DynamicLibraryTask in GLOBAL_TASK_REGISTRY
  → Workflow assembled from host registry → GLOBAL_WORKFLOW_REGISTRY
```

### Computation graph packaging (needs to be built):
```
#[computation_graph(features=["packaged"])] macro
  → extends CloacinaPlugin impl with get_graph_metadata() + execute_graph()
  → same fidius_plugin_registry!()
  → cargo build --lib → cdylib
  → .cloacina archive
  → reconciler loads via same fidius path
  → metadata indicates "computation_graph" type
  → reconciler routes to ReactiveScheduler::load_graph()
  → Accumulators spawned, reactor spawned, events flow
```

### Design decision: Single plugin interface (Option B)

Rather than a separate `CloacinaComputationGraphPlugin` trait, we extend `CloacinaPlugin` with two additional methods that have default "not supported" implementations:

```rust
#[fidius::plugin_interface(version = 2, buffer = PluginAllocated)]
pub trait CloacinaPlugin: Send + Sync {
    // Existing workflow methods
    fn get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError>;
    fn execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, PluginError>;

    // New computation graph methods (default: not supported)
    fn get_graph_metadata(&self) -> Result<GraphPackageMetadata, PluginError> {
        Err(PluginError::NotImplemented("get_graph_metadata"))
    }
    fn execute_graph(&self, request: GraphExecutionRequest) -> Result<GraphExecutionResult, PluginError> {
        Err(PluginError::NotImplemented("execute_graph"))
    }
}
```

One plugin interface. One load path. The `CloacinaMetadata` (package.toml) gains a `package_type` list field (`["workflow"]`, `["computation_graph"]`, or `["workflow", "computation_graph"]`) so the reconciler knows which methods to call. A package can contain both — the reconciler iterates the list and routes to each scheduler.

**Version bump**: interface version goes from 1 → 2. Old plugins (version 1) work fine — they just don't implement the new methods.

Subsumes backlog items: T-0380 (reconciler wiring).

## Goals & Non-Goals

**Goals:**
- Extend `CloacinaPlugin` trait with `get_graph_metadata()` and `execute_graph()` (default not-supported)
- Bump fidius interface version to 2
- Update `#[computation_graph]` codegen to emit FFI plugin impl under `#[cfg(feature = "packaged")]`
- Extend `CloacinaMetadata` (package.toml) with `package_type` field and computation graph metadata
- Extend `PackageLoader` to call graph methods when metadata indicates computation graph
- Wire reconciler → ReactiveScheduler for computation graph packages
- Bridge from FFI graph metadata to `ComputationGraphDeclaration` + `AccumulatorFactory`
- Build an example packaged computation graph
- End-to-end test: build → load → spawn → execute (Rust and Python)
- Python computation graph packaging: `language = "python"` + `package_type = ["computation_graph"]`, entry module with `@node` + `@passthrough_accumulator` decorators, loaded via reconciler's existing Python path
- Example packaged Python computation graph

**Non-Goals:**
- WebSocket endpoints (already exist from I-0071)
- DAL persistence (later)
- Hot reload / zero-downtime updates (later)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `CloacinaPlugin` extended with `get_graph_metadata()` and `execute_graph()` with default impls
- [ ] Interface version bumped to 2
- [ ] New FFI types: `GraphPackageMetadata`, `GraphExecutionRequest`, `GraphExecutionResult`
- [ ] `CloacinaMetadata` gains `package_type: Vec<String>` field (e.g., `["workflow"]`, `["computation_graph"]`)
- [ ] `#[computation_graph]` macro generates FFI plugin impl under `#[cfg(feature = "packaged")]`
- [ ] Accumulator macros emit metadata for packaging
- [ ] `PackageLoader` calls `get_graph_metadata()` when package_type includes computation_graph
- [ ] Reconciler routes computation graph metadata to ReactiveScheduler
- [ ] ReactiveScheduler spawns accumulators + reactor from loaded package
- [ ] `execute_graph()` called via FFI for each graph execution
- [ ] Example packaged computation graph
- [ ] Python computation graph package: `language = "python"`, entry module imports, decorators register graph + accumulators
- [ ] Python package loaded via reconciler's existing Python import path → routes to ReactiveScheduler
- [ ] Example packaged Python computation graph
- [ ] End-to-end test (Rust): load package → graph spawns → push events → graph executes → verify output
- [ ] End-to-end test (Python): load Python package → graph spawns → push events → verify
- [ ] Existing workflow packages continue to work (backward compatible)
- [ ] All existing tests pass

## Implementation Plan

1. **Extend plugin interface** — add `get_graph_metadata()` + `execute_graph()` to `CloacinaPlugin`, new FFI types, bump version to 2
2. **Extend package metadata** — `CloacinaMetadata` gains `package_type` + computation graph fields
3. **Macro codegen** — `#[computation_graph]` emits `_ffi` module with plugin impl under `#[cfg(feature = "packaged")]`
4. **Accumulator macro codegen** — accumulator macros emit metadata for FFI `get_graph_metadata()` response
5. **PackageLoader extension** — detect package_type, call appropriate methods
6. **Reconciler routing** — route computation graph packages to ReactiveScheduler
7. **AccumulatorFactory bridge** — create factories from FFI-loaded graph metadata
8. **Rust example** — packaged market maker
9. **Python packaging** — extend reconciler's Python import path for computation graphs, entry module pattern
10. **Python example** — packaged Python computation graph
11. **End-to-end tests** — Rust and Python full cycle
12. **Backward compatibility test** — verify old version 1 workflow packages still load
