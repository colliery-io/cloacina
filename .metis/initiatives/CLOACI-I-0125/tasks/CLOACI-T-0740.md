---
id: reactor-declaration-defaults
level: task
title: "Reactor declaration defaults (criteria=all-declared, optional manual_rx, default InputStrategy) + collapse ReactionMode/ReactionCriteria [breaking]"
short_code: "CLOACI-T-0740"
created_at: 2026-06-17T05:33:17.504047+00:00
updated_at: 2026-06-17T11:19:00.264806+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/blocked"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Reactor declaration defaults (criteria=all-declared, optional manual_rx, default InputStrategy) + collapse ReactionMode/ReactionCriteria [breaking]

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T8** of the [[CLOACI-T-0720]] sweep. Sequenced
**last** because it carries the one breaking sub-item.

## Objective

Default the redundant reactor declaration ceremony and (separately) collapse the
two parallel enums that restate each other. The additive defaults can land early;
the enum collapse is breaking and must be sequenced/announced.

## Type / Priority
- Tech Debt (DX) — **mixed**: defaults are additive; the enum collapse is breaking.
  P2.

## Background (verified — T-0720)
- `accumulators=[a,b]` and `criteria=when_any(a,b)` restate each other — the parser
  even validates one is a subset of the other (`reactor_attr` `:215-229`). Make
  `criteria=when_any` (no args) default to all declared accumulators.
- `manual_rx` and `InputStrategy::Latest` are restated — default them on a
  `Reactor` builder (shared with [[CLOACI-T-0738]]).
- Two parallel enums `ReactionMode` (macro) vs `ReactionCriteria` (runtime) are
  bridged by `From` — collapsing them is the **breaking** item.

## Acceptance Criteria
- [ ] `criteria=when_any`/`when_all` with no args defaults to all declared
      accumulators; explicit subsets still work.
- [ ] `manual_rx` and `InputStrategy` are optional with sane defaults on the
      reactor builder.
- [ ] The `ReactionMode`/`ReactionCriteria` collapse is done as a clearly-flagged
      breaking change, sequenced after the additive wins, with a migration note.
- [ ] Minimal reactor example as the regression guard.

## Implementation Notes
`reactor_attr` parser + runtime enum (`crates/cloacina-macros` + `crates/cloacina`).
Split delivery: ship the additive defaults first; do the enum collapse as its own
breaking commit/PR section. Coordinate channel/strategy defaults with the embedded
CG builder [[CLOACI-T-0738]].

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.- 2026-06-17: **BLOCKED — deferred pending fidius wasm traits.** fidius is
  introducing a wasm implementation of traits that may significantly reshape the
  authoring/packaging story (cdylib + FFI + build-shell model). Per the user,
  defer this cluster so we don't build something the wasm direction reworks.
  Unblock = fidius wasm-traits direction settles. See
  [[project_fidius_wasm_authoring_shift]].
