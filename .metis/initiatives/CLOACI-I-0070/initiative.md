---
id: computation-graph-vertical-slice
level: initiative
title: "Computation Graph Vertical Slice — Embedded Mode"
short_code: "CLOACI-I-0070"
created_at: 2026-04-04T17:48:53.852921+00:00
updated_at: 2026-04-04T20:39:46.785749+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: computation-graph-vertical-slice
---

# Computation Graph Vertical Slice — Embedded Mode Initiative

## Context

First implementation initiative for CLOACI-I-0069 (Continuous Scheduling for Reactive Strategy Workloads). Implements the `#[computation_graph]` macro and proves that the compile-time graph resolution model works: topology parsing, topological sort, enum routing via match arms, compile-time validation, and code generation.

No accumulators, no reactor, no WebSocket. Just the macro crate and direct execution of the compiled function with test data. This is the riskiest piece — if the macro works, everything else is plumbing around it.

Spec: CLOACI-S-0006 (Computation Graph Macro).

## Goals & Non-Goals

**Goals:**
- Implement the `#[computation_graph]` attribute proc macro on modules
- Parse `graph = { }` topology declaration syntax (linear `->`, routing `=> { Variant -> downstream }`, fan-in, fan-out)
- Topological sort of the graph declaration → execution order
- Generate a single compiled async function with nested match arms for enum routing
- Compile-time validation: completeness (no orphans/dangling), enum variant coverage, type safety across edges
- Implement `#[node(blocking)]` detection and `spawn_blocking` wrapping
- Terminal node detection — identify leaf nodes, collect outputs into `GraphResult`
- `Option<T>` on intermediate nodes → branch-level conditional propagation
- Define `GraphResult` type (Completed / Error)
- Unit tests: topology parsing, validation error cases, generated code correctness
- Test examples: linear chain, enum routing, fan-in, fan-out, conditional propagation, blocking nodes

**Non-Goals:**
- Accumulators (I-0074)
- Reactor (I-0074)
- WebSocket / API server (I-0071)
- DAL persistence (I-0072)
- Packaging (I-0072+)
- Python graph bindings are a fast-follow on this initiative — once the Rust macro works, the Python decorator maps a class + dict topology to the same compiled function. Can be done immediately after or in parallel with I-0074.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `#[computation_graph]` attribute macro compiles on a Rust module
- [ ] `graph = { }` topology declaration parsed: linear edges (`->`), routing edges (`=> { Variant -> node }`)
- [ ] Node inputs from parenthesized names: `decision_engine(alpha, beta)` parsed as cache inputs
- [ ] Topological sort produces correct execution order
- [ ] Generated async function has nested match arms matching the topology
- [ ] Completeness validation: orphan function in module → compile error
- [ ] Completeness validation: dangling reference in graph → compile error
- [ ] Enum variant coverage: unhandled variant → compile error
- [ ] Type safety: mismatched types across edges → compile error
- [ ] `#[node(blocking)]` generates `spawn_blocking` wrapper in compiled function
- [ ] `Option<T>` return on intermediate node → branch short-circuits, other branches unaffected
- [ ] Fan-in: multiple edges to same node, function receives all upstream values
- [ ] Fan-out: one node feeds multiple downstream, all receive the output
- [ ] Terminal nodes identified, outputs collected into `GraphResult::Completed`
- [ ] Test: call compiled function directly with mock `InputCache`, verify correct routing and terminal outputs
- [ ] All existing tests continue to pass

## Implementation Plan

1. **Proc macro crate** — new `cloacina-computation-graph-macros` crate (or extend `cloacina-macros`)
2. **Topology parser** — parse `graph = { }` syntax from macro attribute args
3. **Graph IR** — internal representation: nodes, edges (linear/routing), inputs, terminal detection
4. **Topological sort** — Kahn's algorithm on the graph IR, cycle detection
5. **Validation** — completeness, enum variant coverage, type checking (may need to defer some checks to generated code rather than macro-time)
6. **Code generator** — emit the compiled async function with nested match arms
7. **`#[node(blocking)]` handling** — detect attribute on functions, generate `spawn_blocking` wrapper
8. **GraphResult type** — define in the runtime crate
9. **Tests** — one test per topology pattern (linear, routing, fan-in, fan-out, conditional, blocking, error cases)
