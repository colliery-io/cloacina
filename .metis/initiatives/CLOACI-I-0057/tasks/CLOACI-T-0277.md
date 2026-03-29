---
id: filesystemworkflowregistry
level: task
title: "FilesystemWorkflowRegistry — directory-backed WorkflowRegistry trait impl"
short_code: "CLOACI-T-0277"
created_at: 2026-03-28T15:30:03.830966+00:00
updated_at: 2026-03-29T00:33:18.213612+00:00
parent: CLOACI-I-0057
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0057
---

# FilesystemWorkflowRegistry — directory-backed WorkflowRegistry trait impl

## Parent Initiative

[[CLOACI-I-0057]]

## Objective

Implement `FilesystemWorkflowRegistry` — a `WorkflowRegistry` trait implementation backed by a directory of `.cloacina` files. This lets the existing `RegistryReconciler` work with filesystem-based package storage instead of database blobs.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `FilesystemWorkflowRegistry` implements `WorkflowRegistry` trait
- [ ] `list_workflows()` scans directory for `.cloacina` files, peeks each manifest for metadata
- [ ] `get_workflow(name, version)` reads archive bytes from disk, returns `LoadedWorkflow`
- [ ] `register_workflow()` copies a package file into the watch directory
- [ ] `unregister_workflow()` removes the file from disk
- [ ] Handles missing/corrupt files gracefully (logs warning, skips)
- [ ] Unit tests for scan, get, register, unregister, corrupt file handling

## Implementation Notes

### Files to create/modify
- `crates/cloacina/src/registry/workflow_registry/filesystem.rs` — new file
- `crates/cloacina/src/registry/workflow_registry/mod.rs` — add `filesystem` module

### Key design points
- Constructor takes `Vec<PathBuf>` for multiple watch directories
- `list_workflows()` scans all directories, uses `peek_manifest()` from `python_loader.rs` to extract metadata from each `.cloacina` archive
- For v1 packages (Rust cdylib with `PackageManifest`), fall back to extracting metadata from the archive's `manifest.json`
- Package ID derived from content fingerprint in the manifest
- Thread-safe: `&self` methods only

### Depends on
- Nothing — first task in the chain

## Status Updates

**2026-03-28**: Implementation complete, all tests pass.

### Changes:
- `filesystem.rs` — `FilesystemWorkflowRegistry` with multi-dir support, ManifestV2 + v1 fallback, deterministic UUID v5 IDs, 13 unit tests
- `mod.rs` — added `pub mod filesystem`
- `registry/mod.rs` — re-exported `FilesystemWorkflowRegistry`
- `Cargo.toml` — added `v5` feature to uuid crate
