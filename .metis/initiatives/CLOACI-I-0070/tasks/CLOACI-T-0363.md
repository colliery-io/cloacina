---
id: tests-topology-patterns-validation
level: task
title: "Tests — topology patterns, validation errors, end-to-end execution"
short_code: "CLOACI-T-0363"
created_at: 2026-04-04T19:51:04.088092+00:00
updated_at: 2026-04-04T19:51:04.088092+00:00
parent: CLOACI-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0070
---

# Tests — topology patterns, validation errors, end-to-end execution

## Objective

Comprehensive test suite for the `#[computation_graph]` macro. One test per topology pattern, validation error case, and an end-to-end test that calls the compiled function directly with a mock `InputCache` and verifies correct routing and terminal outputs.

## Acceptance Criteria

- [ ] Test: linear chain (A -> B -> C) — correct execution order, values passed through
- [ ] Test: enum routing (A => { X -> B, Y -> C }) — correct branch taken based on return value
- [ ] Test: multi-level routing (A routes to B, B routes to C/D) — nested match arms work
- [ ] Test: fan-out (A -> B, A -> C) — both B and C receive A's output
- [ ] Test: fan-in (A -> C, B -> C) — C receives both A and B outputs as parameters
- [ ] Test: diamond (A -> B, A -> C, B -> D, C -> D) — D receives both B and C
- [ ] Test: `Option<T>` on intermediate node — `None` short-circuits branch, other branches unaffected
- [ ] Test: `#[node(blocking)]` — function runs in `spawn_blocking`, result still correct
- [ ] Test: validation error — orphan function in module produces compile error
- [ ] Test: validation error — dangling node reference produces compile error
- [ ] Test: validation error — unhandled enum variant produces compile error
- [ ] Test: validation error — cycle in graph produces compile error
- [ ] Test: end-to-end — construct mock `InputCache`, call compiled function, verify `GraphResult::Completed` with correct terminal outputs
- [ ] All existing tests continue to pass

## Implementation Notes

Use `trybuild` for compile-error tests (validation errors that should produce compile failures with specific error messages). Use standard `#[tokio::test]` for runtime tests.

The end-to-end test is the most important — it proves the macro produces a callable, correct function. Define a small computation graph in the test module, compile it, call it with test data, assert on outputs.

### Dependencies
T-0359 (parser), T-0360 (Graph IR), T-0361 (code generator), T-0362 (runtime types) — all must be complete before this task can fully execute. Some tests (parser unit tests, IR unit tests) can be written alongside those tasks.

## Status Updates

*To be added during implementation*
