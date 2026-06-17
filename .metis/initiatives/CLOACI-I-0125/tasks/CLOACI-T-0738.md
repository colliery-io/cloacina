---
id: embedded-cg-runtime-builder-absorb
level: task
title: "Embedded CG runtime builder — absorb the manual main() wiring block"
short_code: "CLOACI-T-0738"
created_at: 2026-06-17T05:33:14.790133+00:00
updated_at: 2026-06-17T11:18:59.835593+00:00
parent: CLOACI-I-0125
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/blocked"


exit_criteria_met: false
initiative_id: CLOACI-I-0125
---

# Embedded CG runtime builder — absorb the manual main() wiring block

## Parent Initiative

[[CLOACI-I-0125]] — acts on theme **T6** of the [[CLOACI-T-0720]] sweep.

## Objective

Give embedded computation-graph authors a builder so they stop copy-pasting the
~60-line `main()` wiring block (four `mpsc::channel`s, an always-`None`-field
`AccumulatorContext`, the `CompiledGraphFn` closure, a "required but unused"
`manual_rx`, a restated `InputStrategy::Latest`, two `tokio::spawn`s). The
production scheduler already proves this is a ~3-line load.

## Type / Priority
- Tech Debt (DX) — additive (new embedded builder; manual wiring still works). P2.

## Background (verified — T-0720)
- The block is verbatim across `examples/tutorials/.../08-accumulators/src/main.rs`
  (incl. the unused `manual_rx`, `:170-171`), tutorial-10, and
  `examples/performance/computation-graph/src/main.rs:274-328`.
- The macro **already emits** the `CompiledGraphFn` closure for inventory
  (`crates/cloacina-computation-graph` codegen `:290-294`).
- The production scheduler does this in ~3 lines via `load_graph(decl)`
  (`crates/cloacina/src/.../scheduler.rs:99-115`).

## Acceptance Criteria
- [ ] An embedded-friendly builder (e.g. `Graph::spawn(&shutdown)` + a generated
      `<mod>_graph_fn()` ctor) replaces the manual channel/spawn/closure block.
- [ ] The embedded CG tutorials/examples are rewritten to use it (the ~60-line
      block disappears); a minimal embedded CG example is the regression guard.
- [ ] No regression in CG runtime behavior (accumulators advance, reactor fires).

## Implementation Notes
Expose the builder from `cloacina-computation-graph` (or `cloacina`), reusing the
already-emitted closure. Mirror what the production scheduler's `load_graph`
already does. Relates to reactor defaults [[CLOACI-T-0740]] (shared `InputStrategy`
/ channel defaults).

## Status Updates
- 2026-06-17: Filed from the T-0720 decomposition. Not started.- 2026-06-17: **BLOCKED — deferred pending fidius wasm traits.** fidius is
  introducing a wasm implementation of traits that may significantly reshape the
  authoring/packaging story (cdylib + FFI + build-shell model). Per the user,
  defer this cluster so we don't build something the wasm direction reworks.
  Unblock = fidius wasm-traits direction settles. See
  [[project_fidius_wasm_authoring_shift]].
