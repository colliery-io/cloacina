---
id: ui-workflows-views-list-paged
level: task
title: "UI workflows views — list (paged) + detail (build status, tasks, version)"
short_code: "CLOACI-T-0652"
created_at: 2026-06-11T02:18:52.768018+00:00
updated_at: 2026-06-11T02:18:52.768018+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI workflows views — list (paged) + detail (build status, tasks, version)

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Read-only workflows surface (REQ-003 read half): `/workflows` list and `/workflows/:name` detail, over `client.listWorkflows()` / `client.getWorkflow()`. Establishes the list+detail pattern the other read tasks follow.

## Acceptance Criteria **[REQUIRED]**

- [ ] `/workflows` — paged list (package name, version, created); loading/empty/error states; row → detail.
- [ ] `/workflows/:name` — detail: `build_status` (pending/building/failed/success, visually distinct), `build_error` when present, tasks, version, created_at.
- [ ] 404 (unknown workflow) renders the not-found state, not a crash (REQ-007).
- [ ] Tenant-scoped via the auth context's default tenant; data flows only through `@cloacina/client`.
- [ ] Detail view leaves obvious anchor points for the write actions added in T-0657 (Execute / Delete buttons can be stubbed/hidden until then).

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
`useWorkflows()` / `useWorkflow(name)` query hooks over the SDK. Build-status as a typed badge component (reused later in overview). Detail is the natural host for T-0657's write actions and T-0656 isn't involved here.

### Dependencies
Blocked by CLOACI-T-0651 (skeleton). Sequences before CLOACI-T-0657 (write ops hang off this detail view).

### Risk Considerations
`build_status` values must match the server's strings exactly — drive the badge off the generated type, not hand-typed literals.

## Status Updates **[REQUIRED]**

*To be added during implementation*
