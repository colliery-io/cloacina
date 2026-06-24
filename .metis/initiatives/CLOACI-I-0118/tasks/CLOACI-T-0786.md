---
id: ui-tenant-admin-surface-own-tenant
level: task
title: "UI tenant-admin surface — own-tenant key management + agent roster"
short_code: "CLOACI-T-0786"
created_at: 2026-06-24T00:41:41.420212+00:00
updated_at: 2026-06-24T00:41:41.420212+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI tenant-admin surface — own-tenant key management + agent roster

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Surface the new tenant-admin capabilities in the web UI (consumes the I-0117 client): a tenant-scoped **Keys** view to list/create/revoke the connected tenant's own keys (against `GET/POST/DELETE /tenants/{t}/keys`), and a tenant-scoped **agent roster** view (`GET /agents`). Handle 403s gracefully — a read/write (non-admin) key simply doesn't see the admin surface. Still bearer-key connect; **no OIDC** (that is later phases).

## Acceptance Criteria **[REQUIRED]**

- [ ] A tenant-admin key sees a Keys view that lists own-tenant keys, mints a new key (plaintext shown exactly once), and revokes a key.
- [ ] A non-admin key does not see the Keys/admin surface (or shows a clear "insufficient permissions" empty state on 403).
- [ ] An agent-roster view shows only the connected tenant's agents.
- [ ] All calls go through the generated `@cloacina/client` SDK — no hand-rolled fetch.
- [ ] UI build + typecheck clean; verified against a **live** server (not spec-vs-spec).

## Implementation Notes

**Scope:** UI consumption of the T-0784/T-0785 endpoints. Conceptually UI (I-0117) but tracked under I-0118 to land the auth slice end-to-end — cross-link [[CLOACI-I-0118]] ↔ I-0117.
**Depends on:** T-0784 + T-0785 (endpoints) and the `@cloacina/client` SDK.
**References:** `ui/src/auth/AuthContext.tsx`, `ui/src/routes/Connect.tsx`; SDK-must-be-tested-against-a-live-server.

## Status Updates **[REQUIRED]**

*To be added during implementation*
