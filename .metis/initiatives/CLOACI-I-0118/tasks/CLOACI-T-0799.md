---
id: ui-demo-self-managed-no-idp-local
level: task
title: "UI demo — self-managed (no-IdP) local login flow in the compose stack"
short_code: "CLOACI-T-0799"
created_at: 2026-06-24T01:26:51.876096+00:00
updated_at: 2026-06-24T01:26:51.876096+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI demo — self-managed (no-IdP) local login flow in the compose stack

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Extend the docker-compose demo stack to show the self-managed (no-IdP) flow end-to-end: god creates a tenant + tenant-admin, the tenant-admin creates a local account, that user logs in via username/password in the UI, operates within the tenant, and the session survives a silent refresh — with NO IdP container required.

## Acceptance Criteria **[REQUIRED]**

- [ ] The compose demo stack brings up server + UI with the `local` provider enabled and no IdP container.
- [ ] A created local account logs into the UI via username/password and operates within its tenant.
- [ ] Silent refresh keeps the session alive; logout ends it.
- [ ] Runs against a FRESH DB; no hand-run host server/compiler/UI processes.

## Implementation Notes

**Scope:** demo-stack wiring + a short walkthrough; exercises Tasks 2/3/4 together.
**Depends on:** Task 4 / CLOACI-T-0798 (UI).
**References:** docker-compose demo stack (bring the stack up via the demo stack, never hand-run processes); related demo work in CLOACI-I-0131.

## Status Updates **[REQUIRED]**

*To be added during implementation*
