---
id: typed-context-accessors-get-as-get
level: task
title: "Typed Context accessors (get_as / get_required / insert_as) — Rust parity with Python get/set"
short_code: "CLOACI-T-0733"
created_at: 2026-06-17T05:33:06.805738+00:00
updated_at: 2026-06-17T05:33:06.805738+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
- [ ] `Context::get_as::<T: DeserializeOwned>(key) -> Result<Option<T>, TaskError>`,
      `get_required::<T>(key) -> Result<T, TaskError>` (errors on miss/mismatch),
      and `insert_as<T: Serialize>(key, T)` exist and are documented.
- [ ] At least one example/tutorial is rewritten to use them, showing the
      line-count drop.
- [ ] Error messages on miss/type-mismatch are actionable (name the key + type).
- [ ] Existing `get`/`insert` untouched.

## Implementation Notes
Add to `crates/cloacina-workflow/src/context.rs`. Fold the boilerplate, emit a
`TaskError` (keep the typed-error contract). Consider whether the `#[task]` macro
can lean on these later (coordinate with [[CLOACI-T-0734]]).

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.