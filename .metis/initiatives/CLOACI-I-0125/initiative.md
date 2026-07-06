---
id: authoring-surface-cruft-removal
level: initiative
title: 'Authoring-surface cruft removal — "just types + functions" (act on the T-0720 sweep)'
short_code: "CLOACI-I-0125"
created_at: 2026-06-17T05:31:01.766970+00:00
updated_at: 2026-06-17T05:31:01.766970+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: authoring-surface-cruft-removal
---

# Authoring-surface cruft removal — "just types + functions" (act on the T-0720 sweep) Initiative

## Context **[REQUIRED]**

The authoring-surface sweep [[CLOACI-T-0720]] audited what a workflow /
computation-graph author is *forced* to write today (Rust + Python) and produced
file:line-grounded findings plus a ranked list of cruft-removal opportunities.
Headline: "just types + functions" is already close on the happy path — the task
list, dependencies, DAG, and `workflow_name` are FFI-derived from compiled code,
not hand-written. The remaining cruft is **concentrated, repetitive, and mostly
additive to remove** (new ergonomic path; old path still compiles). The biggest
theme: several things are already optional in code, but every example still
writes them, so authors learn ceremony as if it were required.

This initiative is the **execution arm** for that sweep: turn the T-0720
recommendations into landed changes. It is a follow-on to the (completed)
authoring-DX initiative [[CLOACI-I-0119]], which lowered *setup* friction
(scaffold, one-command pack, validate, canonical format); this targets the
per-workflow *code* friction. Low-friction authoring is core to the
embedded-first philosophy, not cosmetic.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- Cut the author-written boilerplate for each primitive (Task, Workflow,
  Computation Graph, Reactor, Accumulator, Package) toward the minimum the types
  and signatures already imply.
- Keep changes additive where possible (old path still compiles); flag and
  sequence the one breaking item explicitly.
- Land a "minimal author" reference example per touched primitive as a regression
  guard against re-accreting boilerplate (an AC carried over from T-0720).
- Close the two hard Rust↔Python parity *failures* (route via [[CLOACI-T-0688]]).

**Non-Goals:**
- Re-running the audit — T-0720 is the research; this initiative acts on it.
- "Magic" that hurts explicitness/debuggability — prefer derive-with-escape-hatch
  over hidden behavior. Items T-0720 marked "keep" (the `#[workflow] pub mod`
  boundary, the CG edge DSL, routing enums, the typed `TaskError` contract) are
  out of scope to "simplify."
- Compiler/runtime internals beyond what's needed to remove *author-facing*
  ceremony.

## Detailed Design **[REQUIRED]**

The design is the T-0720 **Sweep Findings** section (themes T1–T9, with file:line
evidence) — not duplicated here to avoid drift. Each child task below maps to one
ranked follow-up from that sweep and carries its own scope, cited code locations,
and additive-vs-breaking call. Read [[CLOACI-T-0720]] for the grounding before
starting any child.

Child tasks (ROI order; item 10 from the sweep lives in [[CLOACI-T-0688]]):
1. Retire already-optional attrs from examples + default the last few (T1)
2. Typed Context accessors `get_as`/`get_required`/`insert_as` (T2 — Rust parity)
3. Signature/module de-ceremony: bare `Context` / `Result<()>` + prelude (T3)
4. `package.toml` minimization — default constants, infer language/entry_module (T4)
5. FFI-derive manifest metadata — kills the T-0666 drift class (T4 tail)
6. Rust package-shell elimination — `cloacina-workflow-sdk` umbrella + injected build.rs (T5)
7. Embedded CG runtime builder — absorb the manual wiring block (T6)
8. Accumulator passthrough default + surface the macros (T7)
9. Reactor declaration defaults (criteria=all, optional channel, default strategy);
   collapsing the two enums is the one breaking sub-item (T8)

## Alternatives Considered **[REQUIRED]**

- **Do it all in one big refactor** — rejected; the sweep deliberately split this
  into independent, mostly-additive units so each can land + verify in isolation
  and the breaking item can be sequenced last.
- **File the follow-ups as standalone backlog tasks (no initiative)** — rejected;
  they're a coherent body of work and we ship one PR per initiative, so grouping
  keeps the PR/release story clean.
- **Leave the recommendations in T-0720 until individually pulled** — rejected;
  the user asked to decompose now so the work is actionable.

## Implementation Plan **[REQUIRED]**

Sequence by ROI and risk (from T-0720):
1. **Near-pure docs/loader wins first** — children 1 (retire optional attrs) and 4
   (`package.toml` defaults). Smallest, additive, highest ROI.
2. **Additive ergonomics** — children 2, 3, 7, 8 (Context accessors, signature
   de-ceremony, CG runtime builder, accumulator defaults).
3. **Bigger additive structure** — children 5, 6 (FFI-derive manifest metadata,
   package-shell elimination).
4. **Reactor defaults + the breaking enum collapse** — child 9 last, so the one
   breaking change is isolated and sequenced after the additive wins.
5. Parity failures (state accumulator + cron trigger) tracked under
   [[CLOACI-T-0688]].

Each child starts by writing its "minimal author" example as the regression guard
(proves the default actually holds end to end — a gap T-0720 explicitly flagged).

## Status Updates

- 2026-06-17: Created to execute the [[CLOACI-T-0720]] sweep. Decomposed the 9
  in-scope follow-ups into child tasks (item 10 → [[CLOACI-T-0688]]). In
  discovery — awaiting human review of scope/sequencing before transitioning to
  design/decompose (per the initiative human-in-the-loop rule). Not yet on any
  branch.
- 2026-06-17: Execution started on branch `authoring-cruft-i0125` (Ralph loop,
  per user). **[[CLOACI-T-0732]] (T1) complete + verified**: bare `#[task]` now
  defaults `id`→fn-name + empty deps; regression guard passes; examples/docs
  swept (52 deps + 155 Rust id + 57 Python id removed). Surfaced a correction to
  the T-0720 T1 finding — Python `return context` is NOT redundant (the wrapper
  discards in-body mutations on a `None` return); recorded in T-0732 + T-0720.
- 2026-06-17: **[[CLOACI-T-0733]] (T2) complete + verified**: typed Context
  accessors `get_as`/`get_required`/`insert_as` on `Context<serde_json::Value>`
  (unit + doc tests pass); `03-dependencies` example rewritten (15-line reads →
  1 line) and compiled. Building that example also surfaced + fixed a latent
  T-0732 bug: a bare `#[task]` (no parens) inside `#[workflow]` was dropped from
  the DAG; fixed in `workflow_attr.rs` (+`#[derive(Default)]` on TaskAttributes)
  with a workflow-level guard. 10/10 macro tests green. 2 of 9 tasks done.
- 2026-06-17: **[[CLOACI-T-0734]] (T3) complete + verified** (1 AC descoped):
  bare task signatures now work — `Context<T = serde_json::Value>` default param
  enables `&mut Context`, and a macro return-type rewrite turns `-> Result<()>`
  into `-> Result<(), TaskError>` (additive; full 2-arg form untouched). Guard
  compiles+runs; 11/11 macro tests. Prelude `use super::*` auto-injection was
  tried + reverted (warns on existing manual imports). 3 of 9 done.
- 2026-06-17: **[[CLOACI-T-0739]] (T7) done** (scope corrected): blanket
  `process` trait default infeasible by design (generic runtime calls `process`
  for any `A`) — documented. Concrete fix: `state_accumulator` was missing from
  `cloacina`'s macro re-export; added (all 5 reachable). Tutorial conversion
  folded into [[CLOACI-T-0738]]. User cleared running angreal integration/e2e/
  ui-demo for verification. 4 of 9 done; packaging cluster (0735/0736/0737) next.
- 2026-06-17: **`angreal test integration` PASSED** on the 4 landed tasks — all
  Rust integration tests + 29 Python sqlite scenarios green (incl. CG +
  task-invokes-graph). Confirms the `Context<T = serde_json::Value>` default
  param and macro changes regress nothing. [[CLOACI-T-0735]] investigated (see
  its doc) but not yet implemented — `[metadata]` is already mostly optional, the
  real win is inferring `language` from layout; deferred. Checkpoint: remaining 5
  (0735/0736/0737/0738/0740) are the heavy packaging/compiler/CG + breaking tier.
- 2026-07-05: **UNBLOCKED (maintainer call) — the deferral condition settled.** Audit: fidius 0.5.4 shipped + adopted (T-0820); WASM's role is decided — it is the CONSTRUCTOR substrate (I-0132, completed end-to-end incl. capability model + fleet execution); the workflow package model (cdylib + FFI + package.toml) was REAFFIRMED by I-0128/I-0132/T-0722/T-0841 all building further on the cdylib-by-digest artifact. Packaging-cluster ergonomics no longer risk throwaway work. Also verified: the 4-task slice (T-0732/0733/0734/0739) IS merged to main (stale "awaiting PR" note below); and T-0738/T-0740 were never packaging-coupled (swept into the blanket deferral). All 5 → todo; build order per plan: 0735 → 0736 → 0737 → 0738 → 0740 (breaking, last).
- 2026-06-17: **Remaining 5 tasks BLOCKED — deferred pending fidius wasm traits.**
  Per the user: fidius is introducing a wasm implementation of traits that may
  significantly reshape the authoring/packaging story (cdylib + FFI + build-shell
  + manifest model). Building the packaging/FFI/authoring-shell cluster now risks
  throwaway work, so T-0735/0736/0737/0738/0740 are transitioned to `#phase/blocked`
  until that direction settles. The 4 landed tasks are general ergonomics that
  survive regardless. Initiative effectively paused at 4/9 done + 5 blocked. See
  [[project_fidius_wasm_authoring_shift]]. Branch `authoring-cruft-i0125` holds
  the verified slice (integration-green) ready for a PR when desired.
