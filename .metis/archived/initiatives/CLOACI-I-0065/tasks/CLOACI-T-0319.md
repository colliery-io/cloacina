---
id: update-cloacinametadata-schema-add
level: task
title: "Update CloacinaMetadata schema — add language, Python fields, remove old manifest types"
short_code: "CLOACI-T-0319"
created_at: 2026-04-01T12:33:56.270530+00:00
updated_at: 2026-04-01T12:33:56.270530+00:00
parent: CLOACI-I-0065
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0065
---

# Update CloacinaMetadata schema — add language, Python fields, remove old manifest types

## Parent Initiative
[[CLOACI-I-0065]]

## Objective
Update `CloacinaMetadata` in `cloacina-workflow-plugin` to serve as the unified schema for both Rust and Python source packages. This is the foundation all other tasks build on.

## Acceptance Criteria

## Acceptance Criteria
- [ ] `CloacinaMetadata` gains `language: String` field (required — "rust" or "python")
- [ ] Python-specific optional fields: `requires_python`, `entry_module`
- [ ] TOML deserialization tests for Rust metadata, Python metadata, and missing required fields
- [ ] `PackageManifest<CloacinaMetadata>` round-trip test (fidius `load_manifest` + `pack_package`)
- [ ] Add `fidius-core` dependency to cloacina crate for `pack_package`/`unpack_package`/`load_manifest`

## Implementation Notes
### Key file
`crates/cloacina-workflow-plugin/src/types.rs` — `CloacinaMetadata` struct

### Changes
```rust
pub struct CloacinaMetadata {
    pub workflow_name: String,
    pub language: String,           // NEW: "rust" or "python"
    pub description: Option<String>,
    pub author: Option<String>,
    // Python-only
    pub requires_python: Option<String>,  // NEW
    pub entry_module: Option<String>,     // NEW
    pub triggers: Vec<TriggerDefinition>,
}
```

### Depends on
Nothing — first task

## Status Updates
*To be added during implementation*
