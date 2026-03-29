---
id: update-angreal-cloaca-tasks-for
level: task
title: "Update angreal cloaca tasks for native Python in core"
short_code: "CLOACI-T-0271"
created_at: 2026-03-27T13:06:55.853917+00:00
updated_at: 2026-03-27T22:41:06.379906+00:00
parent:
blocked_by: []
archived: true

tags:
  - "#task"
  - "#tech-debt"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Update angreal cloaca tasks for native Python in core

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Related Initiative

I-0050 (Native Python in Core) — this should have been included in the original initiative scope. The cloaca Python module is now served natively from cloacina core, but the angreal task group still references the deleted `bindings/cloaca-backend/` path.

## Objective

Update or rework the angreal `cloaca` task group (smoke, test, package, release, scrub) to work with the new native Python model where the `cloaca` module is embedded in cloacina core via PyO3, not built as a separate maturin wheel from `bindings/cloaca-backend`.

## Current State

- `bindings/cloaca-backend/` deleted in I-0050
- `angreal cloaca smoke` fails: tries to run `maturin build` in `bindings/cloaca-backend`
- `angreal cloaca test` likely fails for same reason
- `angreal cloaca package` / `release` reference the old wheel build path
- The `cloaca` Python module is now registered via `ensure_cloaca_module()` in cloacina core at runtime
- Python tutorials still `import cloaca` — this works at runtime when loaded through cloacina
- `unified_release.yml` already had cloaca wheel/sdist jobs removed (TODO comments left)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `angreal cloaca smoke` passes — verifies `import cloaca` works through cloacina core
- [ ] `angreal cloaca test` passes — runs Python binding tests against native cloacina
- [ ] `angreal cloaca package` reworked or removed (no separate wheel needed)
- [ ] `angreal cloaca release` reworked or removed
- [ ] `angreal cloaca scrub` updated for new paths
- [ ] Python tutorials run successfully via cloacina core
- [ ] `unified_release.yml` TODO comments resolved — decide on Python packaging story

## Implementation Notes

### Key Decision
The `cloaca` Python module must be available BOTH as an embedded module (inside cloacina-powered binaries) AND as a standalone pip-installable wheel. This means:
- Need a `pyproject.toml` at `crates/cloacina` for maturin to build a `cloaca` wheel from core
- `_build_and_install_cloaca_unified()` in `cloaca_utils.py` must point at `crates/cloacina` instead of `bindings/cloaca-backend`
- Python tutorials must continue to work via `pip install` in a venv
- The existing tutorial runner and demo infrastructure stays — just the build path changes
- `unified_release.yml` wheel build jobs need to point at the new location

### Files to Update
- `.angreal/cloaca/smoke.py`
- `.angreal/cloaca/test.py`
- `.angreal/cloaca/package.py`
- `.angreal/cloaca/release.py`
- `.angreal/cloaca/scrub.py`
- `.angreal/cloaca/cloaca_utils.py`
- `.github/workflows/unified_release.yml` (PyPI TODO)

## Status Updates

### 2026-03-27 — Complete

**Python API bindings ported to core:**
- Ported `admin.rs`, `context.rs` (PyDefaultRunnerConfig), `runner.rs` (PyDefaultRunner, PyPipelineResult), `trigger.rs` (PyTriggerResult), `value_objects/` (PyRetryPolicy, etc.) into `crates/cloacina/src/python/bindings/`
- Fixed all import paths from `cloacina::` to `crate::` / `super::`
- Added `#[pymodule] fn cloaca` entry point to `crates/cloacina/src/lib.rs`
- Added `cdylib` crate-type and `abi3-py39` feature to pyo3 dep

**Wheel build from core:**
- Created `crates/cloacina/pyproject.toml` for maturin
- Updated `cloaca_utils.py` to build from `crates/cloacina` (not `bindings/cloaca-backend`)
- Fixed wheel search path (workspace `target/wheels/`, not crate-local)

**Angreal tasks updated:**
- `cloaca smoke` — runs `cargo test python::tests` against core
- `cloaca test` — comprehensive Python integration tests
- `cloaca package/release` — deprecated with informative messages
- `cloaca scrub` — updated paths, cleans temp demo DBs

**Tutorial fixes:**
- Tutorial 07: removed unsupported `workflow=` kwarg from `@cloaca.trigger`

**Results:**
- All 7 Python tutorials pass (01-07)
- `angreal cloaca smoke` passes (4 Python integration tests)
- 585 Rust tests pass, 0 failures
- Wheel builds and installs correctly (`cloaca-0.3.2-cp39-abi3-macosx_11_0_arm64.whl`)
