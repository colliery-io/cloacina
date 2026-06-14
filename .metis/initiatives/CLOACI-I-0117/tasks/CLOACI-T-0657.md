---
id: ui-workflow-write-ops-package
level: task
title: "UI workflow write ops — package upload (progress), execute-with-context, delete"
short_code: "CLOACI-T-0657"
created_at: 2026-06-11T02:18:59.299878+00:00
updated_at: 2026-06-11T12:14:36.498570+00:00
parent: CLOACI-I-0117
blocked_by: [CLOACI-T-0652]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI workflow write ops — package upload (progress), execute-with-context, delete

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

The workflow write surface (REQ-003 write half) + execute (drives UC-1/UC-3): multipart `.cloacina` package upload with progress, execute-with-context, and delete — all with confirms and typed error surfacing, hung off the workflows views from T-0652.

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [x] `/workflows/upload` — file picker (`.cloacina`) → `useUploadWorkflow` → success links to the new package's detail (`/workflows/:package_id`; server resolves UUID); a rejected package surfaces the server's `code`/`error` inline via `ErrorState`. **Progress caveat:** the SDK upload is fetch-based (no byte-progress) — shown as a busy state, not a percentage; noted below.
- [x] **Execute** (workflow detail): Modal with optional JSON-context textarea (validated client-side), `useExecuteWorkflow`, then navigates to `/executions/:execution_id` — the UC-1 hand-off into T-0656's live stream.
- [x] **Delete** (workflow detail): confirm Modal → `useDeleteWorkflow(name, version)` → navigates back to `/workflows`; idempotent-200 handled (success regardless).
- [x] All three are `useMutation` hooks; errors classified to actionable UI (REQ-007); upload/delete invalidate the workflows-list cache.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
Mutation hooks over the SDK with optimistic-or-invalidate cache handling. The JSON-context editor can be a simple validated textarea (or a light JSON editor component). Execute → redirect to `/executions/:id` is the UC-1 seam into T-0656.

### Dependencies
Blocked by CLOACI-T-0652 (actions live on the workflow detail view). Execute's payoff depends on T-0656 (live stream) for the full "watch it run" loop, but the execute action itself doesn't hard-require it.

### Risk Considerations
Multipart upload from the browser via the SDK — confirm `@cloacina/client`'s `uploadWorkflow` handles `FormData`/progress in-browser (it was built for this, but this is its first real browser exercise; any gap is an upstream SDK fix, not a UI workaround, per the initiative goal).

## Status Updates **[REQUIRED]**

**2026-06-11 — Implemented, typechecks clean.**
- `ui/src/api/workflows.ts`: added `useUploadWorkflow` (mutationFn `client.uploadWorkflow(file)`, invalidates the workflows list), `useExecuteWorkflow` (`client.executeWorkflow(name, …)`), `useDeleteWorkflow` (`client.deleteWorkflow(name, version)`, invalidates list).
- `ui/src/routes/WorkflowUpload.tsx` (new): hidden file input (`accept=".cloacina"`), busy-state upload, success → Alert with link to the new package detail, error → `ErrorState`.
- `ui/src/routes/WorkflowDetail.tsx` (rewritten): Execute Modal (JSON-context textarea, client-side validation, success → `navigate("/executions/:id")`) + Delete confirm Modal (`deleteWorkflow(name, version)` → `navigate("/workflows")`); mutation errors via `classifyError`.
- `ui/src/routes/Workflows.tsx`: Upload button → `/workflows/upload`.
- `ui/src/App.tsx`: wired the `workflows/upload` route.
- **Caveat:** browser upload is fetch-based, so progress is a busy state (no byte-percentage) — true progress would need an XHR path in `@cloacina/client` (upstream, not a UI workaround).