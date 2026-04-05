---
id: computation-graph-tutorial-series
level: initiative
title: "Computation Graph Tutorial Series — Market Maker Walkthrough"
short_code: "CLOACI-I-0072"
created_at: 2026-04-04T17:48:56.100724+00:00
updated_at: 2026-04-05T13:54:43.988691+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: L
initiative_id: computation-graph-tutorial-series
---

# Computation Graph Tutorial Series — Market Maker Walkthrough

## Context

Tutorial series that teaches the computation graph system through a market maker scenario. Follows the existing tutorial pattern (`examples/tutorials/01-basic-workflow` through `06-multi-tenancy`) — each tutorial builds on the last, explicitly showing channel plumbing and internal machinery so users understand how the pieces connect.

Uses **embedded mode** — manual wiring in `main()`, no reconciler, no server, no packages. The tutorials teach the primitives: macros, accumulators, reactors, and how they wire together. Plugin-style / server-mode tutorials come later once reconciler routing (T-0380) is wired.

Blocked by: nothing. All required primitives exist from I-0070, I-0074, I-0071.

## Goals & Non-Goals

**Goals:**
- Progressive tutorial series (3-4 tutorials) using a market maker as the narrative
- Tutorial 07: Define a computation graph — `#[computation_graph]` macro, node functions, topology, call the compiled function directly with a hand-built `InputCache`
- Tutorial 08: Add accumulators — `#[passthrough_accumulator]`, create `AccumulatorContext` + `BoundarySender`, spawn `accumulator_runtime`, push events via socket channel, wire boundary output to reactor
- Tutorial 09: Full reactive pipeline — multiple accumulators, reactor with `when_any`, `InputStrategy::Latest`, push events, watch graph fire, inspect terminal outputs
- Tutorial 10: Routing and enum dispatch — `=>` syntax for enum routing, multiple downstream paths, decision engine → signal/no-action branches, terminal nodes on each path
- Each tutorial has its own crate in `examples/tutorials/`
- Each tutorial runnable via `angreal demos tutorial-07` etc.
- Angreal tasks created for each tutorial
- Tutorials compile and run in CI (continue-on-error like existing Python tutorials)
- Documentation pages for each tutorial in the docs site

**Non-Goals:**
- Server mode / WebSocket / reconciler (plugin-path tutorials, later)
- DAL persistence, health states, crash recovery (I-0073 or later)
- State accumulator, batch accumulator, polling accumulator (I-0073)
- Python computation graph tutorials (I-0073)
- Packaging as `.cloacina` (requires T-0380)

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Tutorial 07: `#[computation_graph]` macro with linear chain (3 nodes), call `_compiled()` with hand-built cache, print terminal output
- [ ] Tutorial 08: Passthrough accumulator → boundary → reactor → compiled graph, push events via channel, print output
- [ ] Tutorial 09: 2 accumulators (passthrough + stream via MockBackend) → reactor (when_any) → graph with 3 nodes, push events, verify fire count and output values
- [ ] Tutorial 10: Routing graph with enum dispatch, decision engine → signal handler / audit logger, multiple terminal paths, demonstrate path selection based on input values
- [ ] Each tutorial in `examples/tutorials/07-computation-graph` through `10-routing`
- [ ] Each tutorial has angreal demo task
- [ ] All tutorials compile and run successfully
- [ ] All existing tests continue to pass
- [ ] Docs pages for each tutorial

## Implementation Plan

1. **Tutorial 07** — Minimal computation graph: define 3 nodes, linear topology, call compiled function, print result. Teaches: macro syntax, node functions, topology declaration, `InputCache`, `GraphResult`.
2. **Tutorial 08** — Add accumulator: define passthrough accumulator, create runtime, push events via socket, wire boundary to reactor, watch graph fire. Teaches: `Accumulator` trait, `accumulator_runtime`, `BoundarySender`, `AccumulatorContext`, channel plumbing.
3. **Tutorial 09** — Full pipeline: two accumulators feeding one reactor, `when_any` criteria, `Latest` strategy, multiple pushes, inspect outputs. Teaches: multi-source graphs, reactor lifecycle, dirty flags, cache snapshots.
4. **Tutorial 10** — Routing: `=>` syntax, enum variants, decision engine routing to signal/no-action paths, terminal nodes on each branch. Teaches: enum dispatch, conditional execution paths, `Option<T>` propagation.
5. **Angreal tasks** — `angreal demos tutorial-07` through `tutorial-10`
6. **Docs** — Tutorial pages mirroring the existing tutorial documentation pattern
