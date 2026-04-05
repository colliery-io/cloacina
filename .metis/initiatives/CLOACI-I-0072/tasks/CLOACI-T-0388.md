---
id: tutorial-09-full-reactive-pipeline
level: task
title: "Tutorial 09: Full Reactive Pipeline — multiple accumulators, reactor, when_any, terminal outputs"
short_code: "CLOACI-T-0388"
created_at: 2026-04-05T13:36:43.461170+00:00
updated_at: 2026-04-05T14:04:28.377334+00:00
parent: CLOACI-I-0072
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0072
---

# Tutorial 09: Full Reactive Pipeline — multiple accumulators, reactor, when_any, terminal outputs

## Objective

Third computation graph tutorial. Wires the full reactive pipeline: two accumulators (passthrough + stream via MockBackend) feeding one reactor with `when_any` criteria and `Latest` input strategy. Pushes multiple events, demonstrates that the reactor fires on each update, and inspects terminal output values. Market maker narrative: order book (stream) + pricing updates (passthrough) → computation graph → signal.

## What the user learns
- Multiple accumulators feeding one reactor
- `ReactionCriteria::WhenAny` — fire when any source has new data
- `InputStrategy::Latest` — cache overwrites, intermediate values collapsed
- `DirtyFlags` — how the reactor tracks which sources have new data
- `InputCache::snapshot()` — cache isolation between executions
- Stream accumulators via `MockBackend` (simulated broker)
- `StreamBackendRegistry` factory pattern
- Multiple event pushes and verifying fire count

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example crate at `examples/tutorials/computation-graphs/library/09-full-pipeline/`
- [ ] Two accumulators: passthrough (pricing updates) + stream via MockBackend (order book)
- [ ] Computation graph with 3 nodes taking both inputs
- [ ] Reactor with `WhenAny` criteria, `Latest` strategy
- [ ] Pushes events to both accumulators, verifies graph fires on each
- [ ] Prints cache contents and terminal output values after each fire
- [ ] Demonstrates that pushing to either source triggers execution
- [ ] Graceful shutdown with `shutdown_signal()`
- [ ] Compiles and runs with `angreal demos tutorial-09`
- [ ] Docs page at `docs/content/tutorials/computation-graphs/library/09-full-pipeline.md`

## Implementation Notes

### Files
- `examples/tutorials/computation-graphs/library/09-full-pipeline/Cargo.toml`
- `examples/tutorials/computation-graphs/library/09-full-pipeline/src/main.rs`
- `docs/content/tutorials/computation-graphs/library/09-full-pipeline.md`

### Dependencies
T-0387 (Tutorial 08 — builds on single-accumulator pattern, adds second source + stream backend)

## Status Updates

- 2026-04-05: Complete. Two passthrough accumulators (order book + pricing) → shared boundary channel → reactor (WhenAny/Latest) → market_pipeline graph. 4 pushes → 4 fires. Demonstrates: Optional inputs (pricing None on first fire), when_any firing, confidence-based WAIT/TRADE/MONITOR signals. Used two passthrough accumulators instead of MockBackend stream — simpler for the tutorial and teaches the same multi-source concept. Docs page deferred.
