---
id: workflow-attribute-macro-embedded
level: task
title: "#[workflow] attribute macro — embedded mode with #[ctor] registration"
short_code: "CLOACI-T-0302"
created_at: 2026-03-29T20:39:41.340884+00:00
updated_at: 2026-03-29T20:39:41.340884+00:00
parent: CLOACI-I-0058
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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

*To be added during implementation*
