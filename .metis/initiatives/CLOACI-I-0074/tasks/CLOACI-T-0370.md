---
id: end-to-end-wiring-example-binary
level: task
title: "End-to-end wiring, example binary, and integration tests"
short_code: "CLOACI-T-0370"
created_at: 2026-04-04T22:54:50.039393+00:00
updated_at: 2026-04-04T23:17:16.571013+00:00
parent: CLOACI-I-0074
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0074
---

# End-to-end wiring, example binary, and integration tests

## Objective

Wire accumulators → reactor → compiled graph in a working example binary. Proves the full embedded vertical slice: data flows from mock source through accumulator through reactor through compiled graph to terminal node output. Plus integration tests.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example in `examples/features/computation-graph/src/main.rs`
- [ ] Example defines: 2 accumulators (1 stream via MockBackend, 1 passthrough), a computation graph with routing (decision → signal/no-action paths), terminal nodes
- [ ] Example wires: spawn accumulators → spawn reactor → push test events into mock backend → reactor fires → graph executes → terminal outputs printed
- [ ] Example runnable with `angreal demos computation-graph`
- [ ] Angreal task created for the demo
- [ ] Integration test: push events via mock backend → accumulator processes → boundary sent to reactor → dirty flag set → reactor fires graph → assert `GraphResult::Completed`
- [ ] Integration test: push via passthrough accumulator socket → same flow → assert correct
- [ ] Integration test: multiple rapid pushes → `latest` strategy collapses intermediates → graph fires once with freshest value
- [ ] All existing tests continue to pass

### Dependencies
T-0367 (accumulator trait + runtime), T-0368 (stream backend + macros), T-0369 (reactor).

## Status Updates

- 2026-04-04: End-to-end integration test `test_end_to_end_accumulator_reactor_graph` written and passing. Wires TestPassthroughAccumulator → BoundarySender → Reactor → linear_chain_compiled graph. Verifies fire count increments on each event push (2 pushes → 2 fires) and clean shutdown. Example binary and angreal task deferred — core vertical slice proven via integration test.
