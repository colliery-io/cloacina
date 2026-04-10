---
id: extend-cloacinaplugin-with-graph
level: task
title: "Extend CloacinaPlugin with graph methods, FFI types, and package metadata"
short_code: "CLOACI-T-0398"
created_at: 2026-04-05T17:13:24.293776+00:00
updated_at: 2026-04-05T17:28:31.222389+00:00
parent: CLOACI-I-0080
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0080
---

# Extend CloacinaPlugin with graph methods, FFI types, and package metadata

## Objective

Extend the existing `CloacinaPlugin` fidius trait with two new methods for computation graph support. Define the FFI types that cross the plugin boundary. Extend `CloacinaMetadata` (package.toml schema) with `package_type` list and computation graph fields. Bump interface version from 1 → 2.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `CloacinaPlugin` gains `get_graph_metadata()` → `Result<GraphPackageMetadata, PluginError>` with default not-supported impl
- [ ] `CloacinaPlugin` gains `execute_graph(GraphExecutionRequest)` → `Result<GraphExecutionResult, PluginError>` with default not-supported impl
- [ ] Interface version bumped to 2 in `#[fidius::plugin_interface]`
- [ ] `GraphPackageMetadata` type: graph_name, accumulator declarations (name, type, config), reaction_mode, accumulator_names
- [ ] `GraphExecutionRequest` type: cache entries as `HashMap<String, String>` (JSON-serialized values)
- [ ] `GraphExecutionResult` type: success, terminal_outputs_json, error
- [ ] `AccumulatorDeclarationEntry` type: name, accumulator_type (passthrough/stream/polling/batch), config map
- [ ] `CloacinaMetadata` gains `package_type: Vec<String>` field (default `["workflow"]` for backward compat)
- [ ] `CloacinaMetadata` gains optional `graph_name`, `reaction_mode` fields for CG packages
- [ ] Existing workflow serde round-trip tests still pass
- [ ] New serde round-trip tests for graph metadata types
- [ ] Existing packaged workflow examples still compile

## Implementation Notes

### Files
- `crates/cloacina-workflow-plugin/src/lib.rs` — extend trait, bump version
- `crates/cloacina-workflow-plugin/src/types.rs` — new FFI types, extend CloacinaMetadata

### Key risk
Bumping `#[fidius::plugin_interface(version = 2)]` — need to verify fidius supports version negotiation so old v1 plugins still load. If not, may need to keep version 1 and use method count detection instead.

### Dependencies
None — foundational task.

## Status Updates

- 2026-04-05: Complete. Extended CloacinaPlugin with get_graph_metadata() + execute_graph() (kept version=1 for backward compat — host just doesn't call methods 2/3 on old plugins). New FFI types added. CloacinaMetadata extended with package_type list (default ["workflow"]), workflow_name→Option<String>, graph_name, reaction_mode, input_strategy. Workflow macro generates stub impls for new methods. python_loader.rs updated for Option<String>. 15 plugin tests pass. Packaged workflow example compiles.
