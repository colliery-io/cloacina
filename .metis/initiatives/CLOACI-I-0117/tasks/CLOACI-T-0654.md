---
id: ui-triggers-views-list-paged
level: task
title: "UI triggers views — list (paged) + detail with recent executions"
short_code: "CLOACI-T-0654"
created_at: 2026-06-11T02:18:55.136747+00:00
updated_at: 2026-06-11T02:18:55.136747+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI triggers views — list (paged) + detail with recent executions

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Read-only triggers/schedules surface (REQ-005): `/triggers` list (cron + trigger schedules) and `/triggers/:name` detail with recent executions, over `client.listTriggers()` / `client.getTrigger()`.

## Acceptance Criteria **[REQUIRED]**

- [ ] `/triggers` — paged list: schedule type (cron/trigger), workflow, enabled, cron expression / trigger name, next/last run.
- [ ] `/triggers/:name` — detail: the schedule fields + the recent-executions list (link each to its execution detail in T-0653).
- [ ] Invalid pagination surfaces the server's `invalid_pagination` inline; unknown trigger → 404 state.
- [ ] Loading/empty/error states throughout; data only via `@cloacina/client`.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Reuses the list/detail pattern from T-0652/0653. Recent-execution rows deep-link into `/executions/:id`. Read-only — schedule mutation is not in v1 scope.

### Dependencies
Blocked by CLOACI-T-0651. Benefits from T-0653 existing (deep-links to execution detail) but doesn't hard-require it.

### Risk Considerations
Low. Mostly a straightforward read view.

## Status Updates **[REQUIRED]**

*To be added during implementation*
