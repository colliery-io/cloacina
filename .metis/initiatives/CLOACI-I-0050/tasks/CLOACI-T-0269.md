---
id: update-all-examples-and-tutorials
level: task
title: "Update all examples and tutorials to use cloacina-build"
short_code: "CLOACI-T-0269"
created_at: 2026-03-26T17:33:48.529014+00:00
updated_at: 2026-03-26T17:33:48.529014+00:00
parent: CLOACI-I-0050
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0050
---

# Update all examples and tutorials to use cloacina-build

## Parent Initiative

[[CLOACI-I-0050]]

## Objective

Update every example and tutorial binary to use `cloacina-build` in their `build.rs` so Python workflows work correctly on macOS. Also update any documentation references to the old cloaca bindings pattern.

## Acceptance Criteria

- [ ] Every example `Cargo.toml` has `cloacina-build` as a build-dependency
- [ ] Every example has a `build.rs` calling `cloacina_build::configure()`
- [ ] Python tutorial examples updated to use native cloacina Python (not cloaca bindings)
- [ ] All Rust tutorials compile and run (`angreal demos tutorial-*`)
- [ ] All Python tutorials compile and run (`angreal demos python-tutorial-*`)
- [ ] Documentation updated to reference `cloacina-build` instead of cloaca bindings pattern
- [ ] `angreal cloacina all` passes

## Implementation Notes

### Technical Approach
1. List all example/tutorial Cargo.toml files under `examples/` and `tutorials/`
2. Add `cloacina-build` as `[build-dependencies]` to each
3. Create or update `build.rs` in each with `cloacina_build::configure()`
4. Update Python tutorial examples to import from cloacina directly
5. Update docs (`docs/content/`) references to the old cloaca pattern
6. Run all demos to verify

### Dependencies
T-0267 (PyO3 move) and T-0268 (cloaca removal) should be completed first.

## Status Updates

*To be added during implementation*
