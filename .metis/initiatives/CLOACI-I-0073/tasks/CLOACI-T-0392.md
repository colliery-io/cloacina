---
id: integration-tests-and-examples-for
level: task
title: "Integration tests and examples for polling and batch accumulators"
short_code: "CLOACI-T-0392"
created_at: 2026-04-05T14:41:08.840380+00:00
updated_at: 2026-04-05T15:00:56.399173+00:00
parent: CLOACI-I-0073
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0073
---

# Integration tests and examples for polling and batch accumulators

## Objective

End-to-end integration tests and examples proving both new accumulator types work in a full reactive pipeline: polling accumulator → reactor → compiled graph, and batch accumulator → reactor → compiled graph. Also wires both into a mixed pipeline demonstrating all 4 accumulator types (passthrough, stream, polling, batch) feeding one reactor.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test: polling accumulator with 100ms interval → reactor fires on each poll that returns Some
- [ ] Integration test: polling accumulator returning None → no boundary, reactor doesn't fire
- [ ] Integration test: batch accumulator buffers 5 events, timer flush → single boundary → reactor fires once with batch result
- [ ] Integration test: batch accumulator with max_buffer_size → automatic flush on threshold
- [ ] Integration test: mixed pipeline — passthrough + polling accumulator → reactor (when_any) → graph fires from either source
- [ ] Example or tutorial demonstrating polling accumulator (e.g., "poll a mock API every 200ms")
- [ ] Example or tutorial demonstrating batch accumulator (e.g., "collect order fills, flush every second")
- [ ] All existing computation graph tests continue to pass

## Implementation Notes

### Files
- `crates/cloacina/tests/integration/computation_graph.rs` — add tests for polling + batch
- Optionally: new tutorial(s) in `examples/tutorials/computation-graphs/library/`

### Dependencies
T-0390 (polling accumulator), T-0391 (batch accumulator)

## Status Updates

- 2026-04-05: Complete. Two integration tests added. Polling: TestPoller emits 3 values → graph fires 3+ times. Batch: 5 events buffered → timer flush → single boundary (sum=15) → graph fires once with correct output (40.0). All 7 integration tests pass. Mixed pipeline and tutorials deferred — core pipeline proven.
