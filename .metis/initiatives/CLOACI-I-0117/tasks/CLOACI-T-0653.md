---
id: ui-executions-views-list-status
level: task
title: "UI executions views — list (status/workflow filters, pagination) + detail with event log"
short_code: "CLOACI-T-0653"
created_at: 2026-06-11T02:18:54.351019+00:00
updated_at: 2026-06-11T02:18:54.351019+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI executions views — list (status/workflow filters, pagination) + detail with event log

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The executions surface (REQ-004 non-live half): `/executions` list with filters + pagination, and `/executions/:id` detail rendering the full event log from the REST events endpoint. This is the host view that T-0656 will add live-streaming to — build it streaming-ready.

## Acceptance Criteria **[REQUIRED]**

- [ ] `/executions` — list over `client.listExecutions()` with `status` + `workflow` filters and pagination (the SDK's `iterateExecutions` or explicit limit/offset); each row shows id, workflow, status, started/completed; status visually distinct (esp. Failed).
- [ ] Filters are URL-reflected (e.g. `/executions?status=Failed`) so they're linkable/back-button-safe — supports the debug flow (UC-2).
- [ ] `/executions/:id` — detail via `client.getExecution()` + `client.getExecutionEvents()`: status header + ordered event log (event type, data, timestamp, sequence).
- [ ] Invalid id → typed 400 state; unknown id → 404 (per the server contract: detail 404s, events endpoint returns an empty envelope — handle both).
- [ ] Event-log component is structured so T-0656 can append live events to the same list without a rewrite.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`useExecutions(query)` / `useExecution(id)` / `useExecutionEvents(id)`. Render the event log from a normalized event array so the live hook (T-0656) can push onto the same array with dedup-by-sequence. Keep the "is terminal?" derivation here (drives whether T-0656 even opens a stream).

### Dependencies
Blocked by CLOACI-T-0651. Sequences before CLOACI-T-0656 (live-follow extends this detail view).

### Risk Considerations
The history-vs-live merge (initiative OQ-6) is decided in T-0656, but the event-log data model chosen *here* constrains it — design the event array + ordering key (sequence_num) with that merge in mind.

## Status Updates **[REQUIRED]**

*To be added during implementation*
