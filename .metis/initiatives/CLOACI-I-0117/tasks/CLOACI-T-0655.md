---
id: ui-overview-computation-graph
level: task
title: "UI overview + computation-graph health (accumulators, graphs)"
short_code: "CLOACI-T-0655"
created_at: 2026-06-11T02:18:56.442518+00:00
updated_at: 2026-06-11T11:04:02.031570+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0651]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI overview + computation-graph health (accumulators, graphs)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The landing dashboard (REQ-002) and the computation-graph health surface: `/` overview with an at-a-glance tenant rollup, plus accumulator/graph health via `client.listAccumulators()` / `client.listGraphs()` / `client.getGraph()`. Resolves initiative OQ-4 (graph health: overview-only vs its own top-level view).

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `/` overview: recent executions table + a status rollup (counts by status) + a graph-health summary tile; each deep-links to its full view (`/executions`, `/graphs`, `/workflows`).
- [x] Computation-graph health rendered at `/graphs`: graphs (name, health, accumulator count, paused) + accumulators (name, status), via `listGraphs`/`listAccumulators`; per-graph `/graphs/:name` via `getGraph` with its **404 handled** (typed not-found state).
- [x] **OQ-4 → own top-level view.** Graph health gets a `/graphs` nav item + per-graph detail; the overview shows only a compact summary tile linking to it. Rationale: the overview is a dashboard (summary), graph detail (accumulator lists, per-graph health) deserves its own page rather than crowding the landing.
- [x] Loading/empty/error states throughout; free-form `health`/`status` (`unknown`) rendered defensively by `GraphHealth` (state-badge when `{state}` present, else pretty JSON, never crashes); data only via `@cloacina/client`.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Compose the overview from the existing query hooks (recent executions reuse T-0653's hook; build-status badge reuses T-0652's). Graph-health is its own small section/component that can be promoted to a top-level route cheaply if OQ-4 lands that way.

### Dependencies
Blocked by CLOACI-T-0651. Reuses hooks/components from T-0652/T-0653 if those land first (not hard-required).

### Risk Considerations
`health`/`status` are free-form JSON in the API types (the server hasn't structured them yet) — render defensively and don't assume a fixed shape.

## Status Updates **[REQUIRED]**

**2026-06-11** — Implemented on `i0117-web-ui`:
- `api/health.ts` (`useAccumulators`/`useGraphs`/`useGraph`), `components/GraphHealth.tsx` (defensive renderer for the `unknown` health/status JSON — state-badge or pretty JSON), real `routes/Overview.tsx` (recent executions + status rollup + graph summary tile, all deep-linked), `routes/Graphs.tsx` (graphs + accumulators tables), `routes/GraphDetail.tsx` (per-graph via `getGraph`, 404 handled). Added "Graphs" nav item to `Shell`; wired `/graphs` + `/graphs/:name` in `App.tsx`.
- **OQ-4 resolved → graph health is its own top-level `/graphs` view** (+ per-graph detail), overview shows a summary tile. Recorded in the initiative's open-questions intent; rationale in the acceptance notes.
- **Verified:** `npm run typecheck` clean (one unused-import nit fixed). Vite hot-reloaded into the running stack.
- Read surface (overview + workflows + executions + triggers + graphs) is now complete; T-0656 adds the live tail to the executions detail.