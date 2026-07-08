---
id: signature-module-de-ceremony-bare
level: task
title: "Signature/module de-ceremony — bare Context / Result in task macro + prelude injection"
short_code: "CLOACI-T-0734"
created_at: 2026-06-17T05:33:08.875440+00:00
updated_at: 2026-06-17T10:35:55.584144+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


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

## Acceptance Criteria
- [x] A `#[task]` fn written as `async fn t(context: &mut Context) -> Result<()>`
      compiles, with the macro expanding to the full typed forms. ✅
- [x] The fully-spelled form still compiles (additive). ✅ (11/11 macro tests,
      all existing explicit-signature tasks unchanged.)
- [~] Generated module injects the prelude so common `use` lines are unneeded.
      **Descoped** — auto-injecting `use super::*;` warns (`unused_imports`) on
      every existing module that still writes it. See Status.
- [x] Minimal example demonstrates the bare signature; typed-error contract
      intact (compiled+run guard `test_bare_signature_task_compiles_and_runs`). ✅

## Implementation Notes
Macro-only change in `cloacina-macros` (`tasks.rs`, `workflow_attr.rs`). Pairs
naturally with [[CLOACI-T-0733]] (typed Context accessors) for a clean task body.
Watch error-message quality on the defaulted error path.

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.
- 2026-06-17: **Implemented + verified (with one AC descoped).** Two enablers:
  (1) `Context<T = serde_json::Value>` default type param
  (`crates/cloacina-workflow/src/context.rs`) so a task can write `&mut Context`;
  (2) a return-type **rewrite** in the task macro
  (`normalize_task_return_type`, `crates/cloacina-macros/src/tasks.rs`): a
  single-arg `-> Result<T>` is rewritten to
  `-> ::std::result::Result<T, ::cloacina_workflow::TaskError>` in the re-emitted
  fn, so `-> Result<()>` works while two-arg `Result<T, E>` is emitted verbatim
  (purely additive; typed-error contract preserved). Guard
  `test_bare_signature_task_compiles_and_runs` (`#[task] async fn t(context:
  &mut Context) -> Result<()>`) compiles + runs; 11/11 macro tests pass.
- 2026-06-17: **Prelude `use super::*` injection — tried and reverted (AC
  descoped).** Injecting it into the generated workflow module makes every
  existing module's manual `use super::*;` a redundant glob → `unused_imports`
  warning across the whole codebase + examples. The win (drop one line) isn't
  worth the warning noise unless we also sweep every manual import out. Chose to
  ship the signature de-ceremony (the substantive win) without it. Rewriting via
  a forced 1-arg `Result` alias was also rejected — it would break the many
  modules that still spell the full `Result<(), TaskError>` (2-arg). The macro
  return-type rewrite avoids both problems.
- 2026-06-17: Caveat — verified at crate level (cloacina-workflow + cloacina +
  macros via the sqlite integration build). The `Context` default-param is
  backward-compatible (explicit `Context<T>` unaffected); full-workspace build
  (server/python) left to CI.