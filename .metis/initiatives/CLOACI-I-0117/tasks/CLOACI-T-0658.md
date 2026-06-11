---
id: ui-api-key-management-create-one
level: task
title: "UI API key management — create (one-time plaintext), list, revoke"
short_code: "CLOACI-T-0658"
created_at: 2026-06-11T02:19:00.689036+00:00
updated_at: 2026-06-11T02:19:00.689036+00:00
parent: CLOACI-I-0117
blocked_by: ["CLOACI-T-0651"]
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0117
---

# UI API key management — create (one-time plaintext), list, revoke

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0117]]

## Objective **[REQUIRED]**

Tenant-scoped API-key management (REQ-006, drives UC-4): `/keys` list + create + revoke over `client.createTenantKey()` / `client.listKeys()` / `client.revokeKey()`. The one-time-plaintext flow is the security-sensitive bit.

## Acceptance Criteria **[REQUIRED]**

- [ ] `/keys` — list keys (name, role, created, revoked) with no plaintext ever shown.
- [ ] **Create**: name + role → `createTenantKey()` → the plaintext key is shown **exactly once** in a copy-to-clipboard panel with a clear "you won't see this again" warning; dismissing it removes it from memory/DOM. Never re-displayed, never persisted by the UI.
- [ ] **Revoke**: confirm dialog → `revokeKey()`; list reflects revoked state.
- [ ] Typed error surfacing (REQ-007); cache invalidation on create/revoke.

## Implementation Notes **[CONDITIONAL: Technical Task]**

### Technical Approach
The created key lives only in transient component state for the one-time display — never written to sessionStorage or query cache. Copy-to-clipboard + an explicit acknowledge-and-dismiss step.

### Dependencies
Blocked by CLOACI-T-0651. Independent of the other feature tasks.

### Risk Considerations
Don't let the one-time plaintext leak into TanStack Query cache, logs, or error reports. Note: creating a tenant-scoped key requires an admin-role key (server enforces `is_admin`); surface the 403 cleanly when the user's key can't.

## Status Updates **[REQUIRED]**

*To be added during implementation*
