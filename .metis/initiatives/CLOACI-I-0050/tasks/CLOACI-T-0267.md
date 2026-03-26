---
id: move-pyo3-into-cloacina-core-and
level: task
title: "Move PyO3 into cloacina core and create cloacina-build crate"
short_code: "CLOACI-T-0267"
created_at: 2026-03-26T17:33:45.556746+00:00
updated_at: 2026-03-26T17:38:59.978852+00:00
parent: CLOACI-I-0050
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0050
---

# Move PyO3 into cloacina core and create cloacina-build crate

## Parent Initiative

[[CLOACI-I-0050]]

## Objective

Move PyO3 bindings from `bindings/cloaca-backend` into `crates/cloacina` core so Python workflows run natively without a separate bindings crate. Create `crates/cloacina-build` helper crate that handles Python rpath configuration for downstream binaries (required on macOS).

## Acceptance Criteria

## Acceptance Criteria

- [ ] PyO3 added as a dependency in `crates/cloacina/Cargo.toml` (not feature-gated)
- [ ] Python executor, task runner, context bridge, and workflow modules ported from `bindings/cloaca-backend/src/` into `crates/cloacina/src/python/`
- [ ] `crates/cloacina-build` crate created with `configure()` function
- [ ] `cloacina-build` uses `pyo3-build-config` with `resolve-config` feature
- [ ] `cloacinactl` uses `cloacina-build` in its `build.rs`
- [ ] `cloacina-build` added to workspace members
- [ ] Python workflows execute natively through cloacina core
- [ ] `angreal cloacina all` passes

## Implementation Notes

### Technical Approach
1. Port Python modules from `bindings/cloaca-backend/src/` into `crates/cloacina/src/python/`:
   - `runner.rs` — Python task execution
   - `task.rs` — Python task wrapper
   - `context.rs` — Context bridge (Rust <-> Python)
   - `workflow.rs` — Python workflow definition
   - `trigger.rs` — Python trigger support (if exists)
   - `value_objects/` — Value type conversions
2. Add `pyo3` dependency to `crates/cloacina/Cargo.toml`
3. Create `crates/cloacina-build` (port from archive commit `5aaaa21`):
   - Single `configure()` function
   - Emits PyO3 cfg flags and rpath linker args
   - Handles macOS framework builds
4. Add `cloacina-build` as build-dependency in `crates/cloacinactl/Cargo.toml`
5. Create `crates/cloacinactl/build.rs` calling `cloacina_build::configure()`

### Prior Art
- PyO3 in core: archive commit `a412e0c`
- cloacina-build: archive commit `5aaaa21`

### Dependencies
None — this is the foundational task.

### Risk Considerations
- macOS rpath is the main gotcha — `cloacina-build` must handle both framework and non-framework Python installs
- PyO3 version must match across cloacina core and cloacina-build

## Status Updates

### 2026-03-26 — Complete

**PyO3 in core:**
- Ported 7 new files from archive (`context.rs`, `loader.rs`, `namespace.rs`, `task.rs`, `trigger.rs`, `workflow.rs`, `workflow_context.rs`) into `crates/cloacina/src/python/`
- Updated `mod.rs` to expose all concrete PyO3 bindings (PyContext, PyWorkflowBuilder, TaskDecorator, TriggerDecorator, loader, etc.)
- Added `pyo3 = "0.25"` and `pythonize = "0.25"` to `crates/cloacina/Cargo.toml`
- Added `parse_duration_str()` to `packaging/manifest_v2.rs` (needed by trigger module)
- Added `cloacina-build` as build-dependency + `build.rs` for cloacina core (rpath needed for test binary too)

**cloacina-build crate:**
- Created `crates/cloacina-build` — ported from archive commit `5aaaa21`
- Single `configure()` function: emits PyO3 cfg flags + rpath linker args
- Handles macOS framework builds (splits framework path for rpath)
- Uses `pyo3-build-config` with `resolve-config` feature
- Added to workspace members

**cloacinactl integration:**
- Added `cloacina-build` as build-dependency in `crates/cloacinactl/Cargo.toml`
- Created `crates/cloacinactl/build.rs` calling `cloacina_build::configure()`

**Test results:**
- `angreal cloacina all` passes — 585 tests, 0 failures
- Archive python module tests pass (workflow via GIL, cloaca module registration, stdlib shadowing validation)
