---
id: migrate-rust-packaging-pipeline
level: task
title: "Migrate Rust packaging pipeline from PackageManifest v1 to ManifestV2"
short_code: "CLOACI-T-0286"
created_at: 2026-03-28T22:44:52.433699+00:00
updated_at: 2026-03-29T11:46:14.965295+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Migrate Rust packaging pipeline from PackageManifest v1 to ManifestV2

## Objective

Unify the manifest format: make the Rust packaging pipeline (`compile` → `manifest` → `archive`) emit `ManifestV2` instead of `PackageManifest` (v1). Then remove v1 types and the v1 fallback path in `FilesystemWorkflowRegistry`.

### Tech Debt Impact
- **Current problems**: Two manifest formats coexist — `PackageManifest` (v1) for Rust build pipeline, `ManifestV2` for Python packages and trigger/cron declarations. The `FilesystemWorkflowRegistry` has a v1 fallback path. The reconciler has separate code paths for each format.
- **Benefits of fixing**: Single manifest format everywhere. Rust packages get trigger and cron declarations in their manifest. Simpler reconciler. Delete ~200 lines of v1 types and fallback code.
- **Risk**: Low — v1 is only used internally in the build pipeline. No external consumers.

### Priority
- P2 — Nice to have. Not blocking anything, but accumulates complexity.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `packaging/manifest.rs` generates `ManifestV2` instead of `PackageManifest`
- [ ] `packaging/archive.rs` writes `ManifestV2` as `manifest.json` in `.cloacina` archives
- [ ] `packaging/debug.rs` reads `ManifestV2` for archive inspection
- [ ] `PackageManifest`, `PackageInfo`, `LibraryInfo`, `TaskInfo` (v1 types) deleted from `packaging/types.rs`
- [ ] `FilesystemWorkflowRegistry::peek_v1_manifest()` fallback removed
- [ ] All existing packaging tests updated to use `ManifestV2`
- [ ] Pre-built example packages rebuilt with new format
- [ ] All unit and integration tests pass

## Files affected
- `crates/cloacina/src/packaging/types.rs` — delete v1 types
- `crates/cloacina/src/packaging/manifest.rs` — generate ManifestV2
- `crates/cloacina/src/packaging/archive.rs` — write ManifestV2
- `crates/cloacina/src/packaging/debug.rs` — read ManifestV2
- `crates/cloacina/src/packaging/mod.rs` — update re-exports
- `crates/cloacina/src/packaging/tests.rs` — update tests
- `crates/cloacina/src/registry/workflow_registry/filesystem.rs` — remove v1 fallback
- `crates/cloacina/tests/integration/packaging*.rs` — update tests

## Status Updates

**2026-03-29**: Complete. All v1 types deleted, pipeline emits ManifestV2, all tests pass.
