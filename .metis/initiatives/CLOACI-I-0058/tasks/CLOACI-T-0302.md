---
id: workflow-attribute-macro-embedded
level: task
title: "#[workflow] attribute macro — embedded mode with #[ctor] registration"
short_code: "CLOACI-T-0302"
created_at: 2026-03-29T20:39:41.340884+00:00
updated_at: 2026-03-30T02:40:34.930379+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0058
---

# #[workflow] attribute macro — embedded mode with #[ctor] registration

## Parent Initiative

[[CLOACI-I-0058]]

## Objective

Create the new `#[workflow]` attribute macro for embedded (binary-linked) mode. Applied to a module containing `#[task]` functions, it auto-discovers tasks, validates dependencies, and generates `#[ctor]` registration code that registers the workflow and its tasks in the global registries at startup.

This replaces the current `workflow!` declarative macro approach where tasks are loose functions and explicitly listed.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[workflow(name = "...", description = "...", author = "...")]` attribute macro on a `pub mod`
- [ ] Auto-discovers all `#[task]` functions inside the module (no explicit task list needed)
- [ ] Compile-time dependency validation (cycle detection, missing deps) within the module
- [ ] Generates `#[ctor]` auto-registration into `GLOBAL_WORKFLOW_REGISTRY` and `GLOBAL_TASK_REGISTRY`
- [ ] Namespace: `{tenant}::{CARGO_PKG_NAME}::{workflow_name}::{task_id}` — tenant defaults to `"public"`, package name from `CARGO_PKG_NAME` env var
- [ ] `tenant` parameter optional (defaults to `"public"`)
- [ ] Existing `#[task]` macro works unchanged inside the workflow module
- [ ] Code fingerprinting for task versioning (same as current `#[packaged_workflow]`)
- [ ] Macro only generates embedded registration code when `packaged` feature is NOT enabled (conditional via `cfg`)
- [ ] Unit tests for macro expansion, integration test with a simple workflow

## Implementation Notes

### Files to modify
- `crates/cloacina-macros/src/lib.rs` — register new `#[workflow]` proc macro
- `crates/cloacina-macros/src/workflow.rs` — new implementation (or new file, keeping old `workflow!` intact initially)
- `crates/cloacina-workflow/Cargo.toml` — add `packaged` feature flag

### Key design points
- Reuse task scanning logic from `packaged_workflow.rs` — it already discovers `#[task]` attrs in a module
- Generate constructor function + `#[ctor]` registration, similar to current `workflow!` but module-based
- `CARGO_PKG_NAME` available at compile time via `env!("CARGO_PKG_NAME")`

### Depends on
- Nothing — this is the foundation task

## Status Updates

**2026-03-30**: Implementation complete.

### What was done
- Created `crates/cloacina-macros/src/unified_workflow.rs` — new `#[unified_workflow]` attribute macro
- `#[unified_workflow(name = "...", description = "...", author = "...")]` on `pub mod` with `#[task]` functions
- Auto-discovers all `#[task]` functions inside the module (no explicit task list needed)
- Compile-time dependency validation: cycle detection via `detect_package_cycles()`, missing dep detection with Levenshtein suggestions
- Generates `#[ctor]` auto-registration into `GLOBAL_WORKFLOW_REGISTRY` and `GLOBAL_TASK_REGISTRY`
- Namespace: `{tenant}::{CARGO_PKG_NAME}::{workflow_name}::{task_id}` — package name from `env!("CARGO_PKG_NAME")`
- `tenant` optional (defaults to "public")
- Code fingerprinting via `DefaultHasher` on module content
- Trigger rules rewriting for namespaced task references
- Wrapped in `#[cfg(not(feature = "packaged"))]` — ready for T-0303 to add packaged branch
- Added `packaged` feature flag to `cloacina-workflow/Cargo.toml`
- Re-exported as `unified_workflow` from `cloacina-workflow` and `cloacina`
- Integration test: 2-task workflow with dependency, executes on SQLite, verifies context — passes
- All 357+ unit tests pass, no regressions

### Key decisions
- Generated registration code placed at same scope as module (not inside `const _: ()`) to avoid path resolution issues in integration tests
- Reuses task scanning, cycle detection, and Levenshtein suggestion logic from `packaged_workflow.rs`

**2026-03-30 (update)**: Renamed to `#[workflow]` per user feedback — breaking change.
- Old `workflow!` declarative macro renamed to `workflow_legacy!`
- `#[workflow]` is now the attribute macro (new unified API)
- `#[packaged_workflow]` kept as deprecated for now
- Updated all 16 examples + 3 test files + 4 validation examples to use `workflow_legacy!`
- All unit tests (386), integration tests (SQLite), and macro validation tests pass
