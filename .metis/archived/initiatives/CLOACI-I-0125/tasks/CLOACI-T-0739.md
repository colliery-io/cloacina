---
id: accumulator-passthrough-default
level: task
title: "Accumulator passthrough default + surface the passthrough/stream/polling/batch/state macros in tutorials"
short_code: "CLOACI-T-0739"
created_at: 2026-06-17T05:33:16.300028+00:00
updated_at: 2026-06-17T11:03:56.126368+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Accumulator passthrough default + surface the passthrough/stream/polling/batch/state macros in tutorials

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T7** of the [[CLOACI-T-0720]] sweep.

## Objective

Stop authors hand-writing the boilerplate `Accumulator` impl whose `process` just
deserializes the event, and surface the existing-but-unused accumulator macros in
tutorials so they're the taught path.

## Type / Priority
- Tech Debt (DX) — additive (blanket default + docs). P2.

## Background (verified — T-0720)
- Every example hand-writes `#[async_trait] impl Accumulator { type Output; fn
  process(&mut self, Vec<u8>) -> Option<…> { deserialize(&event).ok() } }`.
- The macros `#[passthrough_accumulator]` / `#[stream_accumulator]` /
  `#[polling_accumulator]` / `#[batch_accumulator]` / `#[state_accumulator]` are
  fully implemented + exported (`crates/cloacina-macros/src/lib.rs:170-240`) with
  **zero** non-test author uses.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria
- [~] A blanket default `process` for `Output: DeserializeOwned` exists.
      **Infeasible by design — rejected with rationale.** A `where Self::Output:
      DeserializeOwned` default cannot be called from the generic runtime
      `accumulator_runtime<A: Accumulator>` (accumulator.rs:486), which invokes
      `process` for any `A` whose `Output` is only `Serialize`-bound; tightening
      that bound breaks the format-agnostic contract. Documented in the trait.
- [x] The accumulator macros are surfaced. ✅ Found `state_accumulator` was
      **missing** from `cloacina`'s macro re-export (only the other four were
      exported) — added it, so all five (`passthrough`/`stream`/`polling`/
      `batch`/`state`) are now reachable via `cloacina::`.
- [~] Tutorial conversion + minimal-example guard — **folded into [[CLOACI-T-0738]]**,
      which rewrites the `08-accumulators` runtime wiring anyway; converting the
      hand-rolled `impl Accumulator` to `#[passthrough_accumulator]` there avoids
      doing the same example twice.
- [x] Hand-written `process` impls unaffected (the macro is the no-boilerplate
      path; the trait method stays required). ✅

## Implementation Notes
Trait default in the accumulator crate (`crates/cloacina-macros` exports + the
accumulator trait — cross-check `CLOACI-S-0004` impl, not opened in the sweep).
Largely a default + docs/examples surfacing pass.

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.
- 2026-06-17: **Done (scope corrected during impl).** The headline AC (blanket
  trait `process` default) is **infeasible by design** — proved by compiler
  error E0277: the generic `accumulator_runtime<A: Accumulator>` calls `process`
  for any `A`, so a `where Self::Output: DeserializeOwned` default isn't callable
  there, and tightening the runtime bound would break the format-agnostic
  contract. Recorded the rationale in the trait doc
  (`crates/cloacina/src/computation_graph/accumulator.rs`). The
  `#[passthrough_accumulator]` macro is the real no-boilerplate path and already
  generates `process`. Concrete fix delivered: `state_accumulator` was **missing**
  from `cloacina`'s macro re-export (`crates/cloacina/src/lib.rs:597-600`) — the
  other four were exported — so `#[state_accumulator]` was unreachable via the
  `cloacina` facade. Added it; cloacina compiles. The tutorial conversion
  (`08-accumulators` hand-rolled impl → macro) is folded into [[CLOACI-T-0738]]
  (which rewrites that example's runtime block anyway), to avoid touching the
  same example twice.