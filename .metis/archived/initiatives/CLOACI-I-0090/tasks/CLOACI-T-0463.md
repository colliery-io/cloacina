---
id: extract-python-bindings-into
level: task
title: "Extract Python bindings into separate cloacina-python crate (EVO-01, EVO-05)"
short_code: "CLOACI-T-0463"
created_at: 2026-04-09T15:47:38.663292+00:00
updated_at: 2026-04-09T15:47:38.663292+00:00
parent: CLOACI-I-0090
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0090
---

# Extract Python bindings into separate cloacina-python crate (EVO-01, EVO-05)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0090]]

## Objective

Extract `src/python/` from the `cloacina` core crate into a new `crates/cloacina-python/` crate. The core crate becomes `crate-type = ["lib"]` only, eliminating the PyO3 dependency for pure-Rust consumers. The Python wheel (maturin) targets the new crate.

**Effort**: 1-2 weeks

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `crates/cloacina-python/` crate with `crate-type = ["cdylib"]`
- [ ] All files from `crates/cloacina/src/python/` moved to the new crate
- [ ] `crates/cloacina/Cargo.toml` no longer has `cdylib` in crate-type
- [ ] `crates/cloacina/Cargo.toml` no longer depends on `pyo3` (except maybe as optional dev-dep)
- [ ] `cloacina-python` depends on `cloacina` (lib) + `pyo3`
- [ ] `extension-module` feature flag moves to `cloacina-python`
- [ ] Maturin `pyproject.toml` / `Cargo.toml` targets `cloacina-python` for wheel builds
- [ ] `#[pymodule] fn cloaca()` moves to the new crate's `lib.rs`
- [ ] `angreal cloaca test` still passes
- [ ] `angreal cloaca package` builds a working wheel
- [ ] All Python tutorials and examples work unchanged
- [ ] Rust-only consumers can `cargo add cloacina` without pulling in PyO3

## Implementation Notes

### Technical Approach

1. Create `crates/cloacina-python/Cargo.toml` with `crate-type = ["cdylib"]`, depends on `cloacina` (lib) + `pyo3`
2. `git mv crates/cloacina/src/python/ crates/cloacina-python/src/`
3. Move `#[pymodule] fn cloaca()` from `crates/cloacina/src/lib.rs` to `crates/cloacina-python/src/lib.rs`
4. Remove `cdylib` from `crates/cloacina/Cargo.toml` crate-type
5. Remove `pyo3` and `pythonize` deps from core (keep as optional if needed for serialization)
6. Update `pyproject.toml` to point maturin at the new crate
7. Update CI (wheel builds, Python tests) to target the new crate
8. Run full Python test suite to verify nothing broke

### Dependencies
Can run in parallel with T-0461/T-0462 (rename phases). No dependency on the rename.

### Risk Considerations
- PyO3 module registration macro interacts with crate-type. Must verify the `#[pymodule]` works from a separate crate that depends on the core library.
- Maturin build config may need `manifest-path` adjustment.
- Python imports (`import cloaca`) must still work — the wheel name stays the same.

## Status Updates

*To be added during implementation*
