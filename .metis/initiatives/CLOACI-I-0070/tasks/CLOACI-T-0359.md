---
id: proc-macro-crate-setup-and
level: task
title: "Proc macro crate setup and topology parser"
short_code: "CLOACI-T-0359"
created_at: 2026-04-04T19:51:00.527201+00:00
updated_at: 2026-04-04T20:07:19.983123+00:00
parent: CLOACI-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0070
---

# Proc macro crate setup and topology parser

## Objective

Create the proc macro crate and implement the topology parser that reads the `graph = { }` declaration from the `#[computation_graph]` macro attribute. This is the foundation — everything else in I-0070 depends on the parser producing a correct intermediate representation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New crate `cloacina-computation-graph-macros` (or extension of `cloacina-macros`) with proc macro setup
- [ ] `#[computation_graph(...)]` attribute macro compiles on a Rust module (skeleton — no code gen yet)
- [ ] Parser reads `react = when_any(...)` / `react = when_all(...)` — extracts accumulator names and reaction criteria
- [ ] Parser reads linear edges: `node_a(inputs) -> node_b`
- [ ] Parser reads routing edges: `node_a(inputs) => { Variant -> node_b, Variant2 -> node_c }`
- [ ] Parser reads node cache inputs from parenthesized names: `decision_engine(alpha, beta, gamma)`
- [ ] Parser handles fan-in (multiple edges targeting same node) and fan-out (same source, multiple targets)
- [ ] Parse errors produce clear compile-time error messages with span information
- [ ] Unit tests for each syntax pattern

## Implementation Notes

Use `syn` for parsing attribute arguments. The topology syntax is custom — needs a custom parser via `syn::parse::Parse`. Parser output is a `ParsedTopology` struct (not yet the full Graph IR — that's T-0360).

### Dependencies
None — first task in the initiative.

## Status Updates

**2026-04-04**: Completed.
- Added `computation_graph/` module to `cloacina-macros` crate (extended existing crate, not new crate)
- Created `parser.rs`: `ParsedTopology`, `ReactionCriteria`, `ParsedEdge` (Linear/Routing), `RoutingVariant`
- Skeleton `#[computation_graph]` macro registered in lib.rs — parses topology, passes module through unchanged
- Parser handles: react (when_any/when_all), linear edges (->), routing edges (=> {}), cache inputs, fan-in, fan-out
- 13 unit tests passing: all syntax patterns + error cases
