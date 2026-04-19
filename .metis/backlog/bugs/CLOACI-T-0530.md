---
id: investigate-flake-computation
level: task
title: "Investigate flake: computation_graph::resilience_tests::test_supervisor_individual_accumulator_restart"
short_code: "CLOACI-T-0530"
created_at: 2026-04-19T02:47:48.132108+00:00
updated_at: 2026-04-19T02:47:48.132108+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Investigate flake: `test_supervisor_individual_accumulator_restart`

## Objective

`angreal cloacina integration` intermittently fails in
`computation_graph::resilience_tests::test_supervisor_individual_accumulator_restart`
at `crates/cloacina/tests/integration/computation_graph.rs:1822:29`.
Observed during I-0097 e2e development:

- Full run summary: `294 passed; 1 failed; 6 ignored; 12 filtered out` —
  that 1 is this test.
- No correlation with the changes in the surrounding work (registry /
  reconciler / compiler refactor) — test touches the reactor's
  per-accumulator supervisor, not the package pipeline.

## Impact

- **Users affected**: contributors running full integration locally.
  Retry usually hides it; CI can flake on an unrelated PR.
- **Reproduction**: run `angreal cloacina integration`. First hit is
  intermittent; no known steady-state repro.

## Acceptance criteria

- [ ] Root cause identified and documented (race, timing assumption,
  DB state leak, pool exhaustion — unknown today).
- [ ] Test either passes deterministically or is marked `#[ignore]`
  with a linked follow-up issue if the underlying work is scoped out.
- [ ] Full `angreal cloacina integration` is green 5 runs in a row
  locally + 3 CI jobs in a row.

## Implementation notes

Starting places to poke at:

- `tests/integration/computation_graph.rs:1822` — the assertion site.
  Note what state is being asserted and what's supposed to have
  recovered by then.
- Accumulator supervisor logic in
  `crates/cloacina/src/computation_graph/accumulator/` — specifically
  the per-accumulator restart path.
- Health-channel + shutdown-signal interactions — prior incidents in
  this area have been races around the reactor's health warming-to-
  live transition.

No dependencies on other tasks. Standalone cleanup — pairs well with
CLOACI-T-0528 (reactor naming drift audit) since both touch the same
subsystem but can be done independently.

## Status Updates

*To be added during investigation*
