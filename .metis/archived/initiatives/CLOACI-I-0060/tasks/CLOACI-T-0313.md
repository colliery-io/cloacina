---
id: create-cloacina-plugin-api
level: task
title: "Create cloacina-plugin-api interface crate with #[plugin_interface] trait and shared types"
short_code: "CLOACI-T-0313"
created_at: 2026-03-31T23:39:05.592557+00:00
updated_at: 2026-04-01T01:18:01.435585+00:00
parent: CLOACI-I-0060
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0060
---

# Create cloacina-plugin-api interface crate with #[plugin_interface] trait and shared types

## Parent Initiative

[[CLOACI-I-0060]]

## Objective

Create a new `cloacina-plugin-api` crate that defines the fidius plugin interface for cloacina's packaged workflow system. This is the single source of truth for the FFI contract — both the plugin (cdylib) and the host (cloacina engine) depend on this crate.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New crate `crates/cloacina-plugin-api/` with Cargo.toml depending on `fidius` (facade crate)
- [ ] `#[plugin_interface(version = 1, buffer = PluginAllocated)]` trait `CloacinaPlugin` with two methods:
  - `get_task_metadata(&self) -> Result<PackageTasksMetadata, PluginError>` — returns workflow/task metadata
  - `execute_task(&self, request: TaskExecutionRequest) -> Result<TaskExecutionResult, PluginError>` — runs a task by name with serialized context
- [ ] Shared serde types: `PackageTasksMetadata`, `TaskMetadataEntry`, `TaskExecutionRequest`, `TaskExecutionResult` — these cross the FFI boundary via fidius wire format
- [ ] `CloacinaMetadata` struct — host-defined metadata schema for `PackageManifest<CloacinaMetadata>` (workflow name, task graph, trigger definitions, description, author)
- [ ] Re-exports `fidius::plugin_impl`, `fidius::PluginError`, and `fidius_core::package::{PackageManifest, PackageHeader}` for plugin author convenience
- [ ] Crate compiles as a dependency of both cdylib plugins and the host binary
- [ ] No dependency on `fidius-host` (that's host-only)
- [ ] Unit tests for serde round-trip of all shared types
- [ ] Unit tests for `CloacinaMetadata` deserialization from TOML

## Implementation Notes

### Files to create
- `crates/cloacina-plugin-api/Cargo.toml`
- `crates/cloacina-plugin-api/src/lib.rs` — trait + re-exports
- `crates/cloacina-plugin-api/src/types.rs` — shared serde types

### Shared types design
`PackageTasksMetadata` replaces the current `cloacina_ctl_package_tasks` C struct:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageTasksMetadata {
    pub workflow_name: String,
    pub package_name: String,
    pub package_description: Option<String>,
    pub package_author: Option<String>,
    pub workflow_fingerprint: Option<String>,
    pub graph_data_json: Option<String>,
    pub tasks: Vec<TaskMetadataEntry>,
}
```

`TaskExecutionRequest` / `TaskExecutionResult` replace the raw buffer passing:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRequest {
    pub task_name: String,
    pub context_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionResult {
    pub success: bool,
    pub context_json: Option<String>,
    pub error: Option<String>,
}
```

### Depends on
- fidius crate published or available via path dependency to `../fides/fidius`

## Status Updates

*To be added during implementation*
