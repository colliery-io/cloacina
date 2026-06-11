---
id: ui-overview-computation-graph
level: task
title: "UI overview + computation-graph health (accumulators, graphs)"
short_code: "CLOACI-T-0655"
created_at: 2026-06-11T02:18:56.442518+00:00
updated_at: 2026-06-11T02:18:56.442518+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI overview + computation-graph health (accumulators, graphs)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The landing dashboard (REQ-002) and the computation-graph health surface: `/` overview with an at-a-glance tenant rollup, plus accumulator/graph health via `client.listAccumulators()` / `client.listGraphs()` / `client.getGraph()`. Resolves initiative OQ-4 (graph health: overview-only vs its own top-level view).

## Acceptance Criteria **[REQUIRED]**

- [ ] `/` overview: recent executions (with status), a status rollup/counts, and a graph-health summary for graphs visible to the key. Each tile deep-links to its full view.
- [ ] Computation-graph health rendered: accumulators (name + status) and graphs (name, health, accumulators, paused). `get_graph` 404 handled.
- [ ] **OQ-4 decision recorded**: graph health stays on the overview, or earns `/graphs` as its own nav item — decide based on the prototype and note the rationale in the status update.
- [ ] Loading/empty/error states; data only via `@cloacina/client`.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Compose the overview from the existing query hooks (recent executions reuse T-0653's hook; build-status badge reuses T-0652's). Graph-health is its own small section/component that can be promoted to a top-level route cheaply if OQ-4 lands that way.

### Dependencies
Blocked by CLOACI-T-0651. Reuses hooks/components from T-0652/T-0653 if those land first (not hard-required).

### Risk Considerations
`health`/`status` are free-form JSON in the API types (the server hasn't structured them yet) — render defensively and don't assume a fixed shape.

## Status Updates **[REQUIRED]**

*To be added during implementation*
