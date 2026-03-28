---
id: reconciler-trigger-registration
level: task
title: "Reconciler trigger registration from packages"
short_code: "CLOACI-T-0273"
created_at: 2026-03-28T02:16:57.201256+00:00
updated_at: 2026-03-28T02:16:57.201256+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Reconciler trigger registration from packages

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Extend the `RegistryReconciler` to read trigger definitions from a loaded package's `ManifestV2` and register them: create `TriggerSchedule` DB records via DAL and register constructors in the global trigger registry so the `TriggerScheduler` picks them up.

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

*To be added during implementation*
