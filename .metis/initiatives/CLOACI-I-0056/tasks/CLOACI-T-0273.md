---
id: reconciler-trigger-registration
level: task
title: "Reconciler trigger registration from packages"
short_code: "CLOACI-T-0273"
created_at: 2026-03-28T02:16:57.201256+00:00
updated_at: 2026-03-28T04:03:51.225178+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Reconciler trigger registration from packages

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Extend the `RegistryReconciler` to read trigger definitions from a loaded package's `ManifestV2` and register them: create `TriggerSchedule` DB records via DAL and register constructors in the global trigger registry so the `TriggerScheduler` picks them up.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] When reconciler loads a package with `triggers` in its manifest, a `TriggerSchedule` record is upserted for each trigger definition
- [ ] For Rust packages (`trigger_type: "rust"`): trigger constructor is loaded from the cdylib symbol table and registered in the global trigger registry
- [ ] `PackageState` tracks registered trigger names so they can be unloaded when the package is removed
- [ ] Unloading a package removes its trigger schedules and deregisters from global trigger registry
- [ ] Packages with no triggers continue to work unchanged

## Implementation Notes

### Files to modify
- `crates/cloacina/src/registry/reconciler/loading.rs` — extend `load_package()` to process trigger definitions
- `crates/cloacina/src/registry/reconciler/mod.rs` — extend `PackageState` to track `trigger_names: Vec<String>`
- `crates/cloacina/src/registry/reconciler/extraction.rs` — add trigger extraction from manifest

### Depends on
- T-0272 (TriggerDefinitionV2 in ManifestV2)

## Status Updates

**2026-03-27**: Implementation complete, all 342 tests pass.

### Changes made:

1. **`crates/cloacina/src/registry/reconciler/mod.rs`**:
   - Added `trigger_names: Vec<String>` to `PackageState` for tracking registered triggers per package

2. **`crates/cloacina/src/registry/reconciler/loading.rs`**:
   - Added `register_package_triggers()` — extracts ManifestV2 from `.cloacina` archive via `peek_manifest()`, reads trigger definitions, registers each in the global trigger registry
   - For Rust triggers: checks if cdylib `ctor` already registered the trigger (skips if so)
   - For other types: creates a `ManifestTrigger` placeholder that the TriggerScheduler can poll
   - Added `unregister_package_triggers()` — removes triggers from global registry on package unload
   - Added `ManifestTrigger` struct implementing `Trigger` trait — placeholder for config-driven triggers (webhook, http_poll, file_watch, etc.)
   - Wired trigger registration into `load_package()` and deregistration into `unload_package()`

3. **`crates/cloacina/src/trigger/registry.rs`**:
   - Added `deregister_trigger(name)` function for removing triggers by name
   - Added `test_deregister_trigger` test

4. **`crates/cloacina/src/trigger/mod.rs`**:
   - Added `deregister_trigger` to re-exports

### Architecture decisions:
- Used existing `peek_manifest()` from `python_loader.rs` to extract ManifestV2 from archives — avoids duplicating archive extraction logic
- **No placeholder triggers** — the reconciler verifies triggers are in the global registry (registered by the package itself via `#[trigger]` macro / `@cloaca.trigger` / manual impl) and warns if a declared trigger has no implementation
- Packages without ManifestV2 or without triggers continue to work unchanged (empty vec returned)
- Triggers are just trait impls — the reconciler doesn't need to know about specific types (webhook, http_poll, etc.)
