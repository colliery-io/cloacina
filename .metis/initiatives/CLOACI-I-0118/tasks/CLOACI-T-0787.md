---
id: ui-demo-tenant-admin-key-scoped
level: task
title: "UI demo — tenant-admin key + scoped agent flow in the compose demo stack"
short_code: "CLOACI-T-0787"
created_at: 2026-06-24T00:41:42.766861+00:00
updated_at: 2026-06-24T00:41:42.766861+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI demo — tenant-admin key + scoped agent flow in the compose demo stack

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Extend the docker-compose demo stack so the tenant-admin flow is demonstrable end-to-end: seed a god key, create a demo tenant, mint a tenant-admin key for it (or seed via `CLOACINA_DEMO_TENANT_KEYS`), register a tenant-scoped execution agent, and walk the UI through tenant-admin key management + the scoped roster — including the negative path where the tenant-admin is blocked from the global `/auth/keys` surface.

## Acceptance Criteria **[REQUIRED]**

- [ ] The demo stack brings up server + UI + a tenant-scoped agent via the compose demo stack (no hand-run host server/compiler/UI processes).
- [ ] A seeded tenant-admin key logs into the UI, manages its tenant's keys, and sees only its tenant's agent in the roster.
- [ ] The demo shows the cross-tenant denial: tenant-admin → 403 on `/auth/keys`.
- [ ] The flow runs against a **fresh** DB.

## Implementation Notes

**Scope:** demo-stack wiring + a short walkthrough/readme; exercises T-0784/T-0785/T-0786 together.
**Depends on:** T-0786 (UI surface).
**References:** docker-compose demo stack (bring the stack up via the demo stack, never hand-run processes); `CLOACINA_DEMO_TENANT_KEYS` seeding in `crates/cloacina-server/src/lib.rs`; related demo work in CLOACI-I-0131.

## Status Updates **[REQUIRED]**

*To be added during implementation*
