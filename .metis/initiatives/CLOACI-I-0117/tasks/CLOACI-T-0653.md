---
id: ui-executions-views-list-status
level: task
title: "UI executions views — list (status/workflow filters, pagination) + detail with event log"
short_code: "CLOACI-T-0653"
created_at: 2026-06-11T02:18:54.351019+00:00
updated_at: 2026-06-11T10:50:46.870903+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0651]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI executions views — list (status/workflow filters, pagination) + detail with event log

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The executions surface (REQ-004 non-live half): `/executions` list with filters + pagination, and `/executions/:id` detail rendering the full event log from the REST events endpoint. This is the host view that T-0656 will add live-streaming to — build it streaming-ready.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `/executions` — list over `client.listExecutions()` with `status`/`workflow` filters + explicit limit/offset pagination (PAGE_SIZE=50, prev/next); rows show workflow, id, status (`StatusBadge`, Failed visually distinct), started/completed.
- [x] Filters + paging URL-reflected via `useSearchParams` (`?status=Failed&workflow=…&offset=…`), back-button-safe; changing a filter resets paging — the UC-2 debug entry point.
- [x] `/executions/:id` — detail via `useExecution` (status header) + `useExecutionEvents` (the `EventLog`); event type, pretty-printed `event_data`, timestamp, sequence.
- [x] Errors typed via `ErrorState`/`classifyError` (invalid id → 400, unknown id → 404); the events endpoint's empty-envelope case renders "No events yet."
- [x] `EventLog` is pure presentation over a `sequence_num`-sorted array — **the data model T-0656 appends live events into**; `isTerminalStatus` (in `util/status.ts`) computed here drives whether T-0656 opens a stream.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`useExecutions(query)` / `useExecution(id)` / `useExecutionEvents(id)`. Render the event log from a normalized event array so the live hook (T-0656) can push onto the same array with dedup-by-sequence. Keep the "is terminal?" derivation here (drives whether T-0656 even opens a stream).

### Dependencies
Blocked by CLOACI-T-0651. Sequences before CLOACI-T-0656 (live-follow extends this detail view).

### Risk Considerations
The history-vs-live merge (initiative OQ-6) is decided in T-0656, but the event-log data model chosen *here* constrains it — design the event array + ordering key (sequence_num) with that merge in mind.

## Status Updates **[REQUIRED]**

**2026-06-11** — Implemented on `i0117-web-ui`:
- `util/status.ts` — `executionStatusColor` (case-insensitive map, gray fallback) + `isTerminalStatus` (the T-0656 stream-open gate). `components/StatusBadge.tsx` over it (reused by overview T-0655).
- `components/EventLog.tsx` — **the streaming-ready data model**: pure presentation over an `ExecutionEvent[]` sorted by `sequence_num`, JSON-pretty-printing `event_data`. T-0656 appends WS events into the same array (dedup by sequence) and re-renders this unchanged.
- `api/executions.ts` — `useExecutions(query)` (with `placeholderData` to keep the page during filter/paging), `useExecution(id)`, `useExecutionEvents(id)`.
- `routes/Executions.tsx` — table + URL-reflected `status`/`workflow` filters + offset pager. `routes/ExecutionDetail.tsx` — status header (+ "in progress" hint when non-terminal) + EventLog. Wired into `App.tsx`.
- **Spec fidelity fix:** `ExecutionDetail` schema has no `workflow_name` (only execution_id/status/tenant_id) — dropped that field from the header (caught by typecheck). Also: `ListExecutionsQuery` is a top-level SDK export, not under `schemas` — passed the query object directly (structurally assignable).
- **Verified:** `npm run typecheck` clean (exit 0), `npm test` 2/2 pass; Vite hot-reloaded into the running stack. Live interaction verified later by T-0661 UAT (the seeder T-0660 provides the executions to list/stream).