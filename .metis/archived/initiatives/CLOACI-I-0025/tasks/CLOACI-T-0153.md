---
id: add-cycle-detection-in-graph
level: task
title: "Add cycle detection in graph assembly"
short_code: "CLOACI-T-0153"
created_at: 2026-03-15T18:24:20.095777+00:00
updated_at: 2026-03-15T19:15:13.275117+00:00
parent: CLOACI-I-0025
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0025
---

# Add cycle detection in graph assembly

**Priority: P0 — CRITICAL**
**Parent**: [[CLOACI-I-0025]]

## Objective

Add topological sort / cycle detection to `assemble_graph()` in `graph.rs`. Currently the graph assembly validates that sources exist but does NOT check for cycles. A cyclic dependency (task A → source S1 → task B → source S2 → task A) will cause infinite loops or deadlocks at runtime with no error at assembly time.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `assemble_graph()` performs topological sort on the task→source→task dependency edges
- [ ] Cyclic graphs return `GraphError::CycleDetected { path: Vec<String> }` with the cycle path for debugging
- [ ] Valid DAGs (including multi-step reactive chains like raw → task A → derived → task B) still assemble correctly
- [ ] LedgerTrigger feedback loops are treated as valid (they are event-driven, not data-flow cycles)
- [ ] Unit test: simple cycle (A→B→A) detected with correct error
- [ ] Unit test: diamond dependency (A→B, A→C, B→D, C→D) is valid
- [ ] Unit test: multi-step reactive chain is valid

## Implementation Notes

- Use Kahn's algorithm (BFS-based topological sort) — simple, O(V+E), reports remaining nodes on cycle
- The graph already has `tasks` and `edges` — build an adjacency list from `edge.source` → task that produces it → `edge.task`
- Need to distinguish data-flow edges (which must be acyclic) from LedgerTrigger edges (which are event-driven and allowed to form "logical cycles")
- Consider: should detector_workflow names be validated for uniqueness here too? Currently last-one-wins silently overwrites in `detector_to_source` HashMap

## Status Updates

### 2026-03-15 — Completed
- Added `CycleDetected { path: Vec<String> }` and `DuplicateDetectorWorkflow` error variants to `GraphAssemblyError`
- Implemented Kahn's algorithm (BFS topological sort) in `assemble_graph()` to detect data-flow cycles
- Cycle detection builds dependency graph from `source.detector_workflow` → consuming task relationships: if a source's detector is itself a task in the graph, that creates a producer→consumer dependency edge
- LedgerTrigger feedback loops are NOT modeled as data-flow edges (they're event-driven, not data dependencies)
- Decided NOT to enforce detector_workflow uniqueness — multiple sources can share a detector (one producer, multiple derived outputs)
- Added 4 tests: simple cycle detected, diamond dependency valid, linear chain valid, shared detector allowed
- All 412 unit tests pass
