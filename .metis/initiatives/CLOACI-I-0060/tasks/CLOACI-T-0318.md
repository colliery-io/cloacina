---
id: validation-abi-drift-detection
level: task
title: "Validation — ABI drift detection, wire format mismatch, and end-to-end packaging round-trip"
short_code: "CLOACI-T-0318"
created_at: 2026-03-31T23:46:01.820880+00:00
updated_at: 2026-04-01T04:12:30.319687+00:00
parent: CLOACI-I-0060
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0060
---

# Validation — ABI drift detection, wire format mismatch, and end-to-end packaging round-trip

## Parent Initiative

[[CLOACI-I-0060]]

## Objective

Prove the fidius integration is correct and robust with dedicated validation tests. These tests verify that the safety guarantees fidius provides (ABI drift detection, wire format checking, buffer strategy validation) actually work end-to-end in the cloacina context — not just in fidius's own test suite.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] **ABI drift test**: Build a plugin dylib with interface version 1, then attempt to load it with a host expecting a different interface hash → confirm `LoadError::InterfaceHashMismatch` with clear error message
- [ ] **Wire format mismatch test**: Build a plugin in debug mode (JSON wire), attempt to load from a release-mode host (bincode wire) → confirm `LoadError::WireFormatMismatch`
- [ ] **Non-fidius dylib test**: Attempt to load an arbitrary `.dylib`/`.so` (not a fidius plugin) → confirm `LoadError` with magic bytes check failure, not a SIGSEGV
- [ ] **End-to-end packaging round-trip**: Build a packaged workflow → create `.cloacina` archive → register via `register_workflow_package` → extract metadata via fidius → execute task via fidius → verify context flows through correctly
- [ ] **Multi-plugin dylib test**: If applicable, verify a dylib with multiple `#[plugin_impl]` registrations loads correctly and the correct plugin is selected
- [ ] **Metadata fidelity test**: Load a plugin, call `get_task_metadata()`, verify all fields match what the `#[workflow]` macro embedded (task IDs, dependencies, descriptions, fingerprint)
- [ ] **Task execution fidelity test**: Call `execute_task()` with known context JSON, verify the task runs, modifies context, and returns correct result
- [ ] **Error propagation test**: Task that returns `Err(TaskError)` → fidius converts to `PluginError` → host receives structured error with code, message, details
- [ ] **Full CI green**: All existing tests plus new validation tests pass on both platforms (macOS, Linux) and both backends (postgres, sqlite)

## Test Implementation

### Location
`crates/cloacina/tests/integration/fidius_validation.rs` — new integration test module

### Test structure
```rust
#[tokio::test]
async fn test_abi_hash_mismatch_rejected() { ... }

#[tokio::test]
async fn test_wire_format_mismatch_rejected() { ... }

#[test]
fn test_non_fidius_dylib_rejected_gracefully() { ... }

#[tokio::test]
async fn test_end_to_end_packaging_round_trip() { ... }

#[tokio::test]
async fn test_metadata_fidelity() { ... }

#[tokio::test]
async fn test_task_execution_fidelity() { ... }

#[tokio::test]
async fn test_task_error_propagation() { ... }
```

### ABI drift test approach
Use `fidius_host::load_library()` with validation against a different expected hash. This doesn't require building two interface crate versions — just pass the wrong expected hash to `validate_against_interface()`.

### Non-fidius dylib approach
Create a minimal cdylib (no fidius macros) and try to load it. The magic bytes check (`FIDIUS\0\0`) should fail immediately with a descriptive error.

## Depends on
- T-0315 (host loading via fidius — need working PluginHandle to test against)
- T-0317 (examples migrated — need working dylibs to load)

## Status Updates

*To be added during implementation*
