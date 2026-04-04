---
id: python-computation-graph-bindings
level: initiative
title: "Python Computation Graph Bindings"
short_code: "CLOACI-I-0075"
created_at: 2026-04-04T17:57:34.795756+00:00
updated_at: 2026-04-04T22:51:22.865639+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: python-computation-graph-bindings
---

# Python Computation Graph Bindings Initiative

## Context

Fast-follow on CLOACI-I-0070 (Computation Graph Macro). Once the Rust `#[computation_graph]` macro works, the Python binding maps a class + dict topology to the same compiled function path via the existing PyO3 executor.

Can run in parallel with I-0074 (Accumulator & Reactor) since it only depends on the macro, not the accumulator/reactor plumbing.

Blocked by: CLOACI-I-0070 (Rust computation graph macro).

Spec: CLOACI-S-0007 Section 3 (Python Bindings).

## Goals & Non-Goals

**Goals:**
- Implement `@computation_graph` Python decorator — dict-based topology declaration, class-based graph
- Python nodes as async methods on the class, returning `(variant_name, value)` tuples for routing
- All Python nodes wrapped in `spawn_blocking` (GIL constraint) — transparent to the author
- PyO3 executor wraps the Python class into a callable async function identical to Rust-compiled graphs
- The reactor can call a Python graph the same way it calls a Rust graph — language invisible
- Python accumulator decorators: `@stream_accumulator`, `@passthrough_accumulator`
- Python accumulators wrapped in `spawn_blocking`
- A working Python computation graph example (same structure as the Rust example from I-0070)
- Unit tests for Python graph execution, routing, conditional propagation

**Non-Goals:**
- Mixed Rust/Python in a single package (one language per package)
- Python-specific topology syntax beyond dict-based declaration
- Python-authored custom `StreamBackend` implementations
- Performance parity with Rust graphs (Python is for DX/prototyping, Rust is for production latency)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `@computation_graph` decorator parses dict topology: inputs, routes (variant → downstream node)
- [ ] Python nodes are async methods on a class decorated with `@computation_graph`
- [ ] Routing via `return ("VariantName", value)` tuples — decorator maps to downstream
- [ ] `None` return on intermediate nodes → branch short-circuits
- [ ] All Python node calls wrapped in `spawn_blocking` automatically
- [ ] PyO3 executor produces a callable async function from the decorated class
- [ ] Reactor can execute a Python graph via the same `graph.execute(&cache)` interface as Rust
- [ ] `@stream_accumulator` decorator generates Python accumulator with `spawn_blocking` wrapping
- [ ] `@passthrough_accumulator` decorator generates socket-only Python accumulator
- [ ] Working example: Python computation graph with routing, callable directly with test data
- [ ] Tests: Python graph routing, conditional propagation, blocking wrapping, error handling
- [ ] All existing tests continue to pass

## Implementation Plan

1. **`@computation_graph` decorator** — parse dict topology, validate node coverage, wire routing
2. **Python node execution** — wrap async methods in `spawn_blocking`, handle `(variant, value)` tuple returns
3. **PyO3 bridge** — expose Python graph as a callable async function matching Rust's `CompiledGraph` interface
4. **Python accumulator decorators** — `@stream_accumulator`, `@passthrough_accumulator` with `spawn_blocking`
5. **Example** — Python version of the computation graph example
6. **Tests** — routing correctness, conditional propagation, spawn_blocking wrapping, error cases
