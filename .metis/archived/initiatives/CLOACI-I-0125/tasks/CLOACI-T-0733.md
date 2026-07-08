---
id: typed-context-accessors-get-as-get
level: task
title: "Typed Context accessors (get_as / get_required / insert_as) — Rust parity with Python get/set"
short_code: "CLOACI-T-0733"
created_at: 2026-06-17T05:33:06.805738+00:00
updated_at: 2026-06-17T10:30:18.711628+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Typed Context accessors (get_as / get_required / insert_as) — Rust parity with Python get/set

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T2** of the [[CLOACI-T-0720]] sweep. Biggest
line-count sink in real Rust task bodies.

## Objective

Add typed Context accessors so Rust task bodies stop hand-writing the
`get().and_then(|v| v.as_array()).ok_or_else(…)?.clone()` + `from_value` dance on
every input read and `json!(...)` on every write. This is a **Rust-side parity
catch-up** — Python's `context.get(k, default)` / `set(k, v)` is already clean.

## Type / Priority
- Tech Debt (DX) — additive (new helpers; existing `get`/`insert` stay). P2.

## Background (verified — T-0720)
- No typed accessor exists: `Context::get` returns `Option<&serde_json::Value>`
  (`crates/cloacina-workflow/src/context.rs:193`), so each input read is a
  multi-line `get → as_array → ok_or_else → from_value` block — repeated ~8× in
  one example (`examples/tutorials/workflows/library/03-dependencies/src/main.rs:118-174`).
- Every write wraps the value in `json!(...)`.

## Acceptance Criteria

## Acceptance Criteria
- [x] `Context::get_as::<T: DeserializeOwned>(key) -> Result<Option<T>, TaskError>`,
      `get_required::<T>(key) -> Result<T, TaskError>` (errors on miss/mismatch),
      and `insert_as<T: Serialize>(key, T)` exist and are documented. ✅
- [x] At least one example/tutorial is rewritten to use them, showing the
      line-count drop (`03-dependencies`: two 15-line blocks → one line each). ✅
- [x] Error messages on miss/type-mismatch are actionable (name the key + type). ✅
- [x] Existing `get`/`insert` untouched. ✅

## Implementation Notes
Add to `crates/cloacina-workflow/src/context.rs`. Fold the boilerplate, emit a
`TaskError` (keep the typed-error contract). Consider whether the `#[task]` macro
can lean on these later (coordinate with [[CLOACI-T-0734]]).

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.
- 2026-06-17: **Implemented + verified.** Added `impl Context<serde_json::Value>`
  in `crates/cloacina-workflow/src/context.rs` with `get_as<V>`,
  `get_required<V>` (both `-> Result<_, TaskError>`, error names key + type via
  `TaskError::ValidationFailed`), and `insert_as<V>` (folds `to_value`).
  `insert_as` **upserts** (mirrors Python's `context.set`), unlike the lower-level
  `insert` which errors on an existing key — documented. Existing `get`/`insert`
  untouched. Unit tests `test_typed_accessors_roundtrip` +
  `test_typed_accessor_errors_are_actionable` pass; 3 doctests pass (the 7
  cron_evaluator doctest failures are pre-existing, unrelated). Rewrote the cited
  `examples/tutorials/.../03-dependencies` blocks (two 15-line get→as_array→
  from_value dances → one `get_required` line each; inserts → `insert_as`) and
  **compiled that example successfully**.
- 2026-06-17: **Caught + fixed a latent T-0732 bug while building the example.**
  A bare `#[task]` (no parens) is a `syn::Meta::Path`, so the `#[workflow]`
  macro's `attr.parse_args::<TaskAttributes>()` failed and **silently dropped the
  task from the compile-time DAG** → a dependent task saw a "missing" dependency
  at runtime. T-0732's standalone guard missed this (it tested a bare task
  *outside* a workflow). Fix in `crates/cloacina-macros/src/workflow_attr.rs`:
  handle `Meta::Path` as `TaskAttributes::default()` (added `#[derive(Default)]`
  to `TaskAttributes`) and resolve the id→fn-name default there too. Added
  regression guard `test_workflow_with_bare_tasks_registers` (bare `#[task]`
  inside `#[workflow]` with a dependent task) — passes; all 10 macro_test cases
  green. (This completes T-0732's bare-task support across both macro paths.)