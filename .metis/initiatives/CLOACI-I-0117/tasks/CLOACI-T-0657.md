---
id: ui-workflow-write-ops-package
level: task
title: "UI workflow write ops — package upload (progress), execute-with-context, delete"
short_code: "CLOACI-T-0657"
created_at: 2026-06-11T02:18:59.299878+00:00
updated_at: 2026-06-11T02:18:59.299878+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0652"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI workflow write ops — package upload (progress), execute-with-context, delete

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The workflow write surface (REQ-003 write half) + execute (drives UC-1/UC-3): multipart `.cloacina` package upload with progress, execute-with-context, and delete — all with confirms and typed error surfacing, hung off the workflows views from T-0652.

## Acceptance Criteria **[REQUIRED]**

- [ ] `/workflows/upload` — select/drag a `.cloacina` file → `client.uploadWorkflow()` with upload progress → result: success links to the new package's detail; a rejected/garbage package surfaces the server's `code`/`error` inline (not a generic failure).
- [ ] **Execute** (from workflow detail): optional JSON-context editor (validated), `client.executeWorkflow()`, then navigate to the new execution's detail — handing off to the live stream (T-0656) for UC-1.
- [ ] **Delete** (from workflow detail): confirm dialog → `client.deleteWorkflow(name, version)`; reflect the documented idempotent-200 behavior gracefully.
- [ ] All three map `CloacinaApiError` to actionable UI (REQ-007); mutations invalidate the relevant TanStack Query caches so lists/detail refresh.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Mutation hooks over the SDK with optimistic-or-invalidate cache handling. The JSON-context editor can be a simple validated textarea (or a light JSON editor component). Execute → redirect to `/executions/:id` is the UC-1 seam into T-0656.

### Dependencies
Blocked by CLOACI-T-0652 (actions live on the workflow detail view). Execute's payoff depends on T-0656 (live stream) for the full "watch it run" loop, but the execute action itself doesn't hard-require it.

### Risk Considerations
Multipart upload from the browser via the SDK — confirm `@cloacina/client`'s `uploadWorkflow` handles `FormData`/progress in-browser (it was built for this, but this is its first real browser exercise; any gap is an upstream SDK fix, not a UI workaround, per the initiative goal).

## Status Updates **[REQUIRED]**

*To be added during implementation*
