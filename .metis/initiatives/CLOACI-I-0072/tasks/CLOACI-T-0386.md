---
id: tutorial-07-your-first-computation
level: task
title: "Tutorial 07: Your First Computation Graph — macro, nodes, topology, compiled execution"
short_code: "CLOACI-T-0386"
created_at: 2026-04-05T13:36:41.209943+00:00
updated_at: 2026-04-05T13:58:29.168879+00:00
parent: CLOACI-I-0072
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0072
---

# Tutorial 07: Your First Computation Graph — macro, nodes, topology, compiled execution

## Objective

First computation graph tutorial. Introduces the `#[computation_graph]` macro, node functions, topology declaration, and calling the compiled function directly. No accumulators, no reactor — just the macro and a hand-built `InputCache`. Market maker narrative: a simple pricing pipeline that takes an order book snapshot and computes a signal.

## What the user learns
- `#[computation_graph]` attribute macro syntax
- `react = when_any(...)` declaration
- `graph = { entry(input) -> process, process -> output }` linear topology
- Async node functions with `Option<&T>` inputs and typed outputs
- `InputCache`, `serialize()`, `SourceName`
- Calling `{module}_compiled(&cache)` and inspecting `GraphResult`

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Example crate at `examples/tutorials/computation-graphs/library/07-computation-graph/`
- [ ] Defines 3 node functions: `ingest` (reads order book), `compute_signal` (calculates spread), `format_output` (produces human-readable result)
- [ ] Linear topology: `ingest(orderbook) -> compute_signal -> format_output`
- [ ] `main()` builds an `InputCache` by hand, calls `_compiled(&cache)`, prints the terminal output
- [ ] Compiles and runs with `angreal demos tutorial-07`
- [ ] Docs page at `docs/content/tutorials/computation-graphs/library/07-computation-graph.md`
- [ ] Docs page explains each concept step by step with code snippets

## Implementation Notes

### Files
- `examples/tutorials/computation-graphs/library/07-computation-graph/Cargo.toml`
- `examples/tutorials/computation-graphs/library/07-computation-graph/src/main.rs`
- `docs/content/tutorials/computation-graphs/library/07-computation-graph.md`

### Dependencies
None — first tutorial in the series.

## Status Updates

- 2026-04-05: Example code complete and running. Pricing pipeline: OrderBookSnapshot → SpreadSignal → FormattedOutput. Output: "Mid: 100.53, Spread: 5.0 bps". Required `ctor` dep and `build.rs` with `cloacina_build::configure()` for PyO3 linking. Docs page deferred — will write after all 4 tutorials are code-complete.
