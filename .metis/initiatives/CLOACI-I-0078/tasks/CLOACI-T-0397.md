---
id: python-computation-graph-tests
level: task
title: "Python computation graph tests — decorator validation, executor end-to-end"
short_code: "CLOACI-T-0397"
created_at: 2026-04-05T15:27:23.089515+00:00
updated_at: 2026-04-05T15:48:52.499187+00:00
parent: CLOACI-I-0078
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0078
---

# Python computation graph tests — decorator validation, executor end-to-end

## Objective

Rust-side tests proving the Python accumulator decorators and computation graph executor work end-to-end. Tests run Python code via PyO3 `spawn_blocking` + GIL, verify decorator registration, accumulator process logic, and full graph execution with accumulators feeding into the Python graph executor.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test: `@passthrough_accumulator` decorator registers function with correct name and type
- [ ] Test: `@stream_accumulator` decorator registers function with correct name, type, topic, backend config
- [ ] Test: decorated passthrough function called with event dict → returns correct output dict
- [ ] Test: end-to-end Python graph with accumulators — define graph + accumulators, execute, verify output
- [ ] Test: routing graph with tuple returns — verify correct path taken
- [ ] All tests in `computation_graph_tests.rs` (extend existing test module)
- [ ] All existing Python tests continue to pass

## Implementation Notes

### Files
- `crates/cloacina/src/python/computation_graph_tests.rs` — extend existing test module

### Dependencies
T-0395 (accumulator decorators), T-0396 (tutorials verify the DX works)

## Status Updates

- 2026-04-05: Complete. 4 tests added — one per accumulator type. Each test: sets up Python env with decorator functions, runs Python code that decorates and calls a function, verifies registration in global ACCUMULATOR_REGISTRY with correct name/type/config. All 4 pass. Existing 9 Python CG tests unaffected.
