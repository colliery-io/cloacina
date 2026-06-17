---
id: signature-module-de-ceremony-bare
level: task
title: "Signature/module de-ceremony — bare Context / Result in task macro + prelude injection"
short_code: "CLOACI-T-0734"
created_at: 2026-06-17T05:33:08.875440+00:00
updated_at: 2026-06-17T05:33:08.875440+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Signature/module de-ceremony — bare Context / Result in task macro + prelude injection

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T3** of the [[CLOACI-T-0720]] sweep.

## Objective

Let the `#[task]` macro accept a bare `context: &mut Context` and `-> Result<()>`
signature (defaulting the spellings) and inject a prelude into the generated
module, so authors stop restating invariant type ceremony on every task. Keep the
typed-error contract; only default the *spelling*.

## Type / Priority
- Tech Debt (DX) — additive (the macro accepts both forms). P2.

## Background (verified — T-0720)
- The macro hardcodes `Context<serde_json::Value>` (`crates/cloacina-macros/src/tasks.rs:919-920`)
  and rebuilds the error as `TaskError::ExecutionFailed` regardless
  (`tasks.rs:923-941`) — so `context: &mut Context<serde_json::Value>) ->
  Result<(), TaskError>` is 100% restated on every task.
- The generated module could inject `use super::*;` so authors stop re-importing
  (`crates/cloacina-macros/src/workflow_attr.rs:314-318`).

## Acceptance Criteria
- [ ] A `#[task]` fn written as `async fn t(context: &mut Context) -> Result<()>`
      compiles, with the macro expanding to the full typed forms.
- [ ] The fully-spelled form still compiles (additive).
- [ ] Generated module injects the prelude so common `use` lines are unneeded.
- [ ] Minimal example demonstrates the bare signature; typed-error contract intact.

## Implementation Notes
Macro-only change in `cloacina-macros` (`tasks.rs`, `workflow_attr.rs`). Pairs
naturally with [[CLOACI-T-0733]] (typed Context accessors) for a clean task body.
Watch error-message quality on the defaulted error path.

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.