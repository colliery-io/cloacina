---
id: packaged-trigger-workflow-example
level: task
title: "Packaged trigger workflow example and demo"
short_code: "CLOACI-T-0276"
created_at: 2026-03-28T02:16:59.756681+00:00
updated_at: 2026-03-28T12:59:19.793533+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Packaged trigger workflow example and demo

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Create an example project and angreal demo task demonstrating a packaged workflow with triggers — both Rust and Python variants — so users can see how to declare and use triggers in packages.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example under `examples/features/` showing a Rust packaged workflow with a trigger definition in its manifest
- [ ] Example or tutorial showing a Python packaged workflow with `@cloaca.trigger` + manifest trigger definition
- [ ] Angreal demo task (`angreal demos packaged-triggers` or similar) that builds and runs the example
- [ ] Example includes comments explaining the manifest trigger fields

## Implementation Notes

### Pattern to follow
- Existing `examples/features/event-triggers/` for the library-API trigger pattern
- Existing `examples/features/packaged-workflows/` for the packaging pattern
- This example combines both: a trigger defined in a package manifest

### Depends on
- T-0272, T-0273 (at minimum — the manifest and reconciler must work)

## Status Updates

**2026-03-28**: Implementation complete, all tests and demos pass.

### Changes made:

1. **`examples/features/packaged-triggers/`** — New Rust cdylib example:
   - `Cargo.toml` — cdylib + rlib crate with packaged workflow macros
   - `build.rs` — cloacina-build for PyO3 rpath
   - `src/lib.rs` — `file_pipeline` packaged workflow with 3 tasks (validate, transform, archive), extensive doc comments explaining ManifestV2 trigger fields with a complete field reference table

2. **`examples/tutorials/python/08_packaged_triggers.py`** — New Python tutorial:
   - Part 1: Trigger poll simulation showing `TriggerResult.skip()`/`TriggerResult.fire(ctx)`
   - Part 2: Triggered workflow execution via `DefaultRunner`
   - Part 3: ManifestV2 trigger declaration explanation with field reference
   - Runs successfully via `angreal demos python-tutorial-08`

3. **`.angreal/demos/demos_utils.py`** — Added `packaged-triggers` to excluded dirs (library, not binary)

4. **`crates/cloacina/pyproject.toml`** — Added `extension-module` to maturin features (fix for wheel builds after T-0272's Cargo.toml change)

### Bonus fix:
- `pyproject.toml` needed `extension-module` in maturin features after we moved it to an opt-in Cargo feature in T-0272. Without this, Python wheel builds produced broken extensions (GIL not held error). This also fixed pre-existing failures in tutorials 01-07.
