---
id: ui-local-username-password-login
level: task
title: "UI — local username/password login + tenant-admin account-management view"
short_code: "CLOACI-T-0798"
created_at: 2026-06-24T01:26:47.605093+00:00
updated_at: 2026-06-24T01:26:47.605093+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI — local username/password login + tenant-admin account-management view

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

UI for self-managed login. Add a username/password login form to the connect/login screen, shown when the `local` provider is enabled, reaching the same "logged-in" end-state as a pasted key: it calls `/auth/local/login`, stores the minted key, and runs the silent-refresh loop. Add a tenant-admin account-management view (create/list/disable/reset the tenant's local accounts) consuming Task 3. All calls go through the generated `@cloacina/client` SDK.

## Acceptance Criteria **[REQUIRED]**

- [ ] With `local` enabled, the UI offers a username/password login that logs the user in (minted key stored, silent refresh running) with NO IdP.
- [ ] A tenant-admin sees an account-management view to create/list/disable/reset its tenant's local accounts; a non-admin does not.
- [ ] All calls go through the generated `@cloacina/client` SDK (no hand-rolled fetch).
- [ ] UI build + typecheck clean; verified against a LIVE server.

## Implementation Notes

**Scope:** local login form + silent-refresh wiring + account-management UI.
**Depends on:** Task 2 / CLOACI-T-0796 (login/refresh) + Task 3 / CLOACI-T-0797 (account endpoints) + `@cloacina/client`.
**References:** `ui/src/auth/AuthContext.tsx`, `ui/src/routes/Connect.tsx`; cross-link I-0117.

## Status Updates **[REQUIRED]**

*To be added during implementation*
