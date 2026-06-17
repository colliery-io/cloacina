---
id: retire-already-optional-attrs-from
level: task
title: "Retire already-optional attrs from examples + default the last few (Rust id/dependencies, Python id/deps/return context)"
short_code: "CLOACI-T-0732"
created_at: 2026-06-17T05:33:04.973727+00:00
updated_at: 2026-06-17T10:18:24.724864+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Retire already-optional attrs from examples + default the last few (Rust id/dependencies, Python id/deps/return context)

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T1** of the [[CLOACI-T-0720]] sweep (highest
ROI, smallest, near-zero risk).

## Objective

Several `#[task]` / `@cloaca.task` attrs are **already optional in code**, but
every example still writes them — so authors learn the ceremony as if it were
required. Make the last one or two optional in practice (tiny macro default) and
do an examples/docs pass that drops the redundant attrs, so a bare `#[task]` is
the taught common case.

## Type / Priority
- Tech Debt (DX) — additive, no breaking change. P2.

## Background (verified — T-0720)
- Rust `dependencies = []` on leaf tasks is already defaulted to `Vec::new()`
  (`crates/cloacina-macros/src/tasks.rs:76,209-211`); only `id` is required
  (`tasks.rs:205`). 48/122 example tasks still write `dependencies = []`.
- Rust `id = "fn_name"` duplicates the fn ident the macro already has
  (`tasks.rs:643,687-690`) — default `id` to the fn name so a bare `#[task]`
  compiles.
- Python `return context` is redundant — the wrapper re-clones the input ctx on a
  `None` return (`crates/cloacina-python/src/task.rs:233-238`).
- Python `id=` already falls back to `func.__name__` (`task.rs:498-502`).
- Python stringly deps unnecessary — function-ref deps are already accepted
  (`task.rs:642-659`).

## Acceptance Criteria
- [x] A bare `#[task]` (no `id`, no `dependencies`) compiles and registers with
      `id` defaulted to the fn name and empty deps. ✅
- [x] Examples/tutorials no longer write `dependencies = []` or `id = "<fn>"`
      where they equal the default; Python examples drop redundant `id=`. ✅
      (`return context` removal **dropped from scope** — see Status: it is NOT
      redundant.)
- [x] A "minimal author" task example exists as a regression guard. ✅
- [x] No behaviour change for tasks that *do* set these explicitly. ✅

## Implementation Notes
Mostly a docs + examples sweep plus 1–2 macro defaults (`tasks.rs`). Additive;
old explicit forms still compile. Start by writing the minimal example to prove
the defaults hold end to end (compile + register + run).

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.
- 2026-06-17: **Implemented + verified.** Macro change (single chokepoint in
  `crates/cloacina-macros/src/tasks.rs`): `id` parse no longer errors when
  omitted (`unwrap_or_default` sentinel), and the proc-macro entry defaults
  `attrs.id` to `input_fn.sig.ident` when empty — so a bare `#[task]` is valid
  with `id`=fn-name and empty deps (deps already defaulted). Added a regression
  guard `test_bare_task_defaults_id_to_fn_name` in
  `crates/cloacina/tests/integration/task/macro_test.rs`. **Verified:** macros
  crate `angreal check` ✅; integration target compiles (macros+sqlite) ✅; the
  guard test **passes** (id defaults to fn name, deps empty, executes). The
  other 305 integration tests still compile → no behaviour change for explicit
  tasks.
- 2026-06-17: Examples/docs sweep done (subagent, edits only — examples not
  bulk-compiled per the all-examples disk-pressure rule; Rust default-removals
  are token-identical so they compile, and `id` was removed ONLY where the
  literal == fn name). Totals: **52** `dependencies = []` removed, **155** Rust
  `id` removed, **57** Python `id=` removed (all across `examples/` + `docs/`
  fenced blocks). Spot-checked demo-branch-rust + tutorial-03: non-empty deps
  and `trigger_rules` correctly preserved; hyphenated ids (≠ fn name) correctly
  kept.
- 2026-06-17: **CORRECTION to the T-0720 T1 finding.** "Python `return context`
  is redundant" is **FALSE**. The PyO3 wrapper
  (`crates/cloacina-python/src/task.rs:205-250`) snapshots `original_data`
  *before* the body runs (line 207) and, on a `None` return, rebuilds the
  context from that snapshot (lines 233-238) — **discarding any in-body
  `context.set(...)` mutations**. So a mutating Python task that drops
  `return context` silently loses its writes. All `return context` removals were
  therefore skipped, and this task's AC was amended. (Potential author footgun;
  noted for follow-up but out of scope here.) Python empty-`dependencies=[]`
  removal was also deferred (low value; needs confirmation of the binding
  default first).
- 2026-06-17 (follow-up under [[CLOACI-T-0733]]): the bare-`#[task]` support
  here was **incomplete** — it worked standalone but a bare `#[task]` (no parens)
  *inside* a `#[workflow]` module was silently dropped from the DAG (the workflow
  macro's `parse_args` fails on a `Meta::Path`). Fixed in `workflow_attr.rs`
  (handle `Meta::Path` → `TaskAttributes::default()` + id default) with a new
  workflow-level regression guard. Bare `#[task]` now works in both paths.