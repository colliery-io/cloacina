---
id: ui-workflows-views-list-paged
level: task
title: "UI workflows views — list (paged) + detail (build status, tasks, version)"
short_code: "CLOACI-T-0652"
created_at: 2026-06-11T02:18:52.768018+00:00
updated_at: 2026-06-11T10:41:22.359124+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0651]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI workflows views — list (paged) + detail (build status, tasks, version)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Read-only workflows surface (REQ-003 read half): `/workflows` list and `/workflows/:name` detail, over `client.listWorkflows()` / `client.getWorkflow()`. Establishes the list+detail pattern the other read tasks follow.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `/workflows` — list (package name, description, version, task count, created); loading/empty/error states; row → detail. (Note: the workflows endpoint is **not** server-paginated — unlike executions/triggers it returns the full list — so there's no pager here.)
- [x] `/workflows/:name` — detail: `build_status` via `BuildStatusBadge` (colored by status, gray fallback for unknown), `build_error` in an alert when present, tasks list, version, created_at.
- [x] 404 (unknown workflow) renders the typed not-found state via `ErrorState`/`classifyError` (REQ-007), not a crash.
- [x] Tenant-scoped through `useTenant()`; all data via `@cloacina/client` (`useWorkflows`/`useWorkflow` hooks).
- [x] Detail view has the Execute / Delete actions stubbed (disabled, tooltipped "Coming in T-0657") — wired anchor points for T-0657.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`useWorkflows()` / `useWorkflow(name)` query hooks over the SDK. Build-status as a typed badge component (reused later in overview). Detail is the natural host for T-0657's write actions and T-0656 isn't involved here.

### Dependencies
Blocked by CLOACI-T-0651 (skeleton). Sequences before CLOACI-T-0657 (write ops hang off this detail view).

### Risk Considerations
`build_status` values must match the server's strings exactly — drive the badge off the generated type, not hand-typed literals.

## Status Updates **[REQUIRED]**

**2026-06-11** — Implemented on `i0117-web-ui`:
- `api/workflows.ts` — `useWorkflows()` / `useWorkflow(name)` TanStack Query hooks over the SDK, tenant-scoped via new `useTenant()` (added to AuthContext).
- `components/BuildStatusBadge.tsx` — status→color map with a gray fallback for unknown values (defensive per REQ-007; reused by overview T-0655). `util/format.ts` — locale timestamp formatter.
- `routes/Workflows.tsx` (Mantine Table, row→detail, loading/empty/error states) + `routes/WorkflowDetail.tsx` (build badge, build_error alert, tasks list, metadata; Execute/Delete stubbed disabled for T-0657). Wired into `App.tsx`, replacing the placeholders.
- Establishes the **list/detail pattern** T-0653/0654 follow: hook → states → table/card; errors flow through `classifyError`.
- **Verified:** `npm run typecheck` clean (exit 0); Vite hot-reloaded the routes into the running `angreal ui up` stack. Full visual/interaction verification is deferred to the T-0661 Playwright UAT (the initiative's automated-acceptance gate); the dev stack currently has an empty tenant so the list shows the empty state.