---
id: auth-refresh-auth-logout-lifecycle
level: task
title: "/auth/refresh + /auth/logout lifecycle + audit"
short_code: "CLOACI-T-0794"
created_at: 2026-06-24T01:09:55.464552+00:00
updated_at: 2026-06-24T01:09:55.464552+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# /auth/refresh + /auth/logout lifecycle + audit

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement `/auth/refresh` — use the stored encrypted refresh token to silently re-mint or extend the short-TTL key before expiry (cadence: refresh when ~⅓ of TTL remains — OQ-3 default). Implement `/auth/logout` — revoke the minted key via the existing revoke path AND forget/revoke the stored refresh token. Audit-log every login, refresh, and logout, reusing `cloacina::security::audit` (REQ-010).

## Acceptance Criteria **[REQUIRED]**

- [ ] All-day session works via silent refresh against the test issuer (no re-consent).
- [ ] Logout revokes the minted key (within the LRU TTL) and erases the stored refresh material.
- [ ] Login, refresh, and logout are all audit-logged.
- [ ] Tests cover refresh-extends-session and logout-revokes.

## Implementation Notes

**Scope:** the refresh + logout endpoints + audit wiring.
**Depends on:** Task 6 (refresh store).
**References:** CLOACI-I-0118 REQ-008, REQ-009, REQ-010, NFR-002; OQ-3 (refresh cadence).

## Status Updates **[REQUIRED]**

*To be added during implementation*
