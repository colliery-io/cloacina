---
id: graph-ir-topological-sort-and
level: task
title: "Graph IR, topological sort, and cycle detection"
short_code: "CLOACI-T-0360"
created_at: 2026-04-04T19:51:01.088128+00:00
updated_at: 2026-04-04T20:09:27.246513+00:00
parent: CLOACI-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0070
---

# Graph IR, topological sort, and cycle detection

## Objective

Transform the `ParsedTopology` from T-0359 into a Graph IR (internal representation) suitable for code generation. Implement topological sort via Kahn's algorithm and cycle detection.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Graph IR struct: `GraphNode` (name, cache inputs, return type info, edges out), `GraphEdge` (linear or routing with variant names), terminal node flag
- [ ] Build Graph IR from `ParsedTopology` — resolve node references, connect edges
- [ ] Topological sort via Kahn's algorithm — produces execution order
- [ ] Cycle detection — cycles produce a compile-time error with the cycle path
- [ ] Terminal node identification — nodes with no outgoing edges
- [ ] Fan-in resolution — nodes with multiple incoming edges get parameters from all upstream
- [ ] Unit tests: linear chain ordering, diamond graph (fan-out + fan-in), cycle detection error, terminal node identification

## Implementation Notes

The Graph IR is the bridge between parsing and code generation. It should be independent of `syn` types — pure data structures that the code generator can consume.

### Dependencies
T-0359 (topology parser — produces `ParsedTopology`)

## Status Updates

**2026-04-04**: Completed.
- Created `graph_ir.rs`: `GraphIR`, `GraphNode`, `GraphEdge` (Linear/Routing), `IncomingEdge`, `GraphRoutingVariant`
- `GraphIR::from_parsed()` builds IR from ParsedTopology: resolves references, builds adjacency lists, marks terminals
- Topological sort via Kahn's algorithm with deterministic ordering (sorted initial queue + sorted candidates)
- Cycle detection: remaining nodes after sort = cycle, error message lists involved nodes
- Terminal node identification (no outgoing edges), entry node identification (no incoming edges)
- Fan-in tracked via `edges_in` with variant info, fan-out via multiple outgoing edges
- Helper methods: `terminal_nodes()`, `entry_nodes()`, `get_node()`, `incoming_sources()`
- 9 unit tests passing: linear chain, routing, diamond, cycle detection, terminals, entries, cache inputs, incoming variants, mixed routing+linear
