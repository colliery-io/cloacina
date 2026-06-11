---
id: ui-triggers-views-list-paged
level: task
title: "UI triggers views — list (paged) + detail with recent executions"
short_code: "CLOACI-T-0654"
created_at: 2026-06-11T02:18:55.136747+00:00
updated_at: 2026-06-11T10:54:44.490676+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0651]
archived: false

tags:
  - "#task"
  - "#phase/active"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI triggers views — list (paged) + detail with recent executions

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Read-only triggers/schedules surface (REQ-005): `/triggers` list (cron + trigger schedules) and `/triggers/:name` detail with recent executions, over `client.listTriggers()` / `client.getTrigger()`.

## Acceptance Criteria **[REQUIRED]**

- [x] `/triggers` — server-paginated list (offset pager, URL-reflected): workflow, type badge (cron/trigger), schedule (cron expr / trigger name), enabled badge, next/last run.
- [x] `/triggers/:name` — detail: schedule fields + recent-executions table. **Deep-link NOT wired** — `TriggerExecution` exposes only a schedule-execution id, not the workflow-execution id `/executions/:id` needs (gap noted below); rows are informational.
- [x] Bad pagination → server's `invalid_pagination` via `ErrorState`/`classifyError`; unknown trigger → typed 404 state.
- [x] Loading/empty/error states throughout; data only via `@cloacina/client` (`useTriggers`/`useTrigger`).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Reuses the list/detail pattern from T-0652/0653. Recent-execution rows deep-link into `/executions/:id`. Read-only — schedule mutation is not in v1 scope.

### Dependencies
Blocked by CLOACI-T-0651. Benefits from T-0653 existing (deep-links to execution detail) but doesn't hard-require it.

### Risk Considerations
Low. Mostly a straightforward read view.

## Status Updates **[REQUIRED]**

**2026-06-11** — Implemented on `i0117-web-ui`:
- `api/triggers.ts` (`useTriggers`/`useTrigger`), `routes/Triggers.tsx` (table + offset pager, type/enabled badges, cron-vs-trigger schedule column), `routes/TriggerDetail.tsx` (schedule card + recent-executions table). Wired into `App.tsx`. Reuses the list/detail pattern from T-0652/0653.
- **Gap found:** the recent-executions deep-link in the acceptance criteria isn't possible — `TriggerExecution` carries only a *schedule-execution* id (`id`, `scheduled_time`, `started_at`, `completed_at`), not the workflow-execution id `/executions/:id` resolves. Rendered informationally instead. Wiring the link would need the server's trigger-detail to include `workflow_execution_id` — a server/SDK enhancement to file if the link is wanted (not blocking).
- **Verified:** `npm run typecheck` clean (exit 0); Vite hot-reloaded into the running stack. Live data verified later via T-0660/T-0661.