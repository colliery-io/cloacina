---
id: update-all-examples-and-tutorials
level: task
title: "Update all examples and tutorials to use cloacina-build"
short_code: "CLOACI-T-0269"
created_at: 2026-03-26T17:33:48.529014+00:00
updated_at: 2026-03-26T22:33:45.339625+00:00
parent: CLOACI-I-0050
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0050
---

# Update all examples and tutorials to use cloacina-build

## Parent Initiative

[[CLOACI-I-0050]]

## Objective

Update every example and tutorial binary to use `cloacina-build` in their `build.rs` so Python workflows work correctly on macOS. Also update any documentation references to the old cloaca bindings pattern.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

### 2026-03-26 — Complete

**Examples updated (19 total):**
- Added `[build-dependencies] cloacina-build` to all 19 example Cargo.toml files
- Created `build.rs` calling `cloacina_build::configure()` in all 19 example directories
- Covers: 10 feature examples, 6 Rust tutorials, 3 performance benchmarks

**Python tutorials:**
- Python tutorials (7 files) still import `cloaca` — this is correct, the `cloaca` module name is preserved via `ensure_cloaca_module()` in cloacina core

**Documentation:**
- Updated `docs/content/reference/repository-structure.md`:
  - Directory layout now shows all current crates (cloacina-build, cloacinactl, cloacina-testing, cloacina-workflow)
  - Replaced "Bindings / cloaca-backend" section with "Python Support" describing native PyO3 embedding
  - Removed `cd bindings/cloaca-backend && maturin develop` build instructions
- Generated docs in `docs/static/api/` still reference old cloaca-backend — will be regenerated on next docs build

**Test results:** `angreal cloacina all` — 585 tests, 0 failures
