---
id: packaged-trigger-workflow-example
level: task
title: "Packaged trigger workflow example and demo"
short_code: "CLOACI-T-0276"
created_at: 2026-03-28T02:16:59.756681+00:00
updated_at: 2026-03-28T02:16:59.756681+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Packaged trigger workflow example and demo

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Create an example project and angreal demo task demonstrating a packaged workflow with triggers — both Rust and Python variants — so users can see how to declare and use triggers in packages.

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

*To be added during implementation*
