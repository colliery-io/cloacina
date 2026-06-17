---
id: accumulator-passthrough-default
level: task
title: "Accumulator passthrough default + surface the passthrough/stream/polling/batch/state macros in tutorials"
short_code: "CLOACI-T-0739"
created_at: 2026-06-17T05:33:16.300028+00:00
updated_at: 2026-06-17T05:33:16.300028+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


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
- [ ] A blanket default `process` for `Output: DeserializeOwned` exists so the
      common passthrough case needs no hand-written `process`.
- [ ] At least one tutorial uses the accumulator macros instead of a hand-rolled
      impl; a minimal accumulator example is the regression guard.
- [ ] Hand-written `process` impls still override the default (additive).

## Implementation Notes
Trait default in the accumulator crate (`crates/cloacina-macros` exports + the
accumulator trait — cross-check `CLOACI-S-0004` impl, not opened in the sweep).
Largely a default + docs/examples surfacing pass.

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.