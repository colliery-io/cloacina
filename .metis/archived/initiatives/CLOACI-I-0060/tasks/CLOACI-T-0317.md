---
id: migrate-packaged-examples-and
level: task
title: "Migrate packaged examples and integration tests to fidius plugin system"
short_code: "CLOACI-T-0317"
created_at: 2026-03-31T23:39:39.038701+00:00
updated_at: 2026-04-01T04:03:20.320270+00:00
parent: CLOACI-I-0060
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0060
---

# Migrate packaged examples and integration tests to fidius plugin system

## Parent Initiative

[[CLOACI-I-0060]]

## Objective

Rebuild all packaged workflow examples with the new fidius-based plugin output, update integration tests, verify `fidius inspect` works on generated dylibs, and add an ABI drift detection test.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] All packaged examples compile and produce fidius-compatible dylibs:
  - `examples/features/packaged-workflows/`
  - `examples/features/simple-packaged/`
  - `examples/features/packaged-triggers/`
  - `examples/features/complex-dag/`
- [ ] `fidius inspect` on each dylib shows `CloacinaPlugin` interface with correct method count and hash
- [ ] Example Cargo.tomls depend on `cloacina-plugin-api` instead of raw `cloacina-macros` + `cloacina-workflow`
- [ ] `dal::workflow_registry` integration tests pass with fidius-loaded packages
- [ ] `registry_workflow_registry_tests` pass
- [ ] Packaging inspection tests pass
- [ ] Update docs: `docs/content/explanation/ffi-system.md` updated to describe fidius-based architecture
- [ ] Update tutorial: `docs/content/tutorials/07-packaged-workflows.md` updated with new dependency setup
- [ ] All CI checks pass (unit, integration, macro tests, both backends, both platforms)

## Implementation Notes

### Depends on
- T-0314 (macro generates fidius plugins)
- T-0315 (host loads via fidius)
- T-0316 (legacy code removed)

### Note
ABI drift detection and validation tests are covered separately in T-0318.

## Status Updates

*To be added during implementation*
