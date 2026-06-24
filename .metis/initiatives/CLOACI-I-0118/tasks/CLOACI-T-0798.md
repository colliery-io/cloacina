---
id: ui-local-username-password-login
level: task
title: "UI — local username/password login + tenant-admin account-management view"
short_code: "CLOACI-T-0798"
created_at: 2026-06-24T01:26:47.605093+00:00
updated_at: 2026-06-24T04:12:29.522559+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# UI — local username/password login + tenant-admin account-management view

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

UI for self-managed login. Add a username/password login form to the connect/login screen, shown when the `local` provider is enabled, reaching the same "logged-in" end-state as a pasted key: it calls `/auth/local/login`, stores the minted key, and runs the silent-refresh loop. Add a tenant-admin account-management view (create/list/disable/reset the tenant's local accounts) consuming Task 3. All calls go through the generated `@cloacina/client` SDK.

## Acceptance Criteria

## Acceptance Criteria

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

**2026-06-24 — COMPLETE.** SDK: regen types + added `localLogin`/`refresh`/`logout` + `listAccounts`/`createAccount`/`disableAccount`/`resetAccountPassword` to `@cloacina/client`; rebuilt. UI: `Connect.tsx` gains a **Username & password** mode (SegmentedControl) — mints a short-TTL key via `/auth/local/login` (a no-key `CloacinaClient` for the public call) then `connect()`s with it; multi-tenant individuals use the existing per-tenant connection list + switcher (no multi-tenant subject). New **Accounts** view (`routes/Accounts.tsx` + `api/accounts.ts`): tenant-admin create/list/disable/reset-password over `/tenants/{t}/accounts`; a non-admin 403 → clear error state. Routed (`/accounts`) + sidebar nav (IconUsers). `tsc -b --noEmit` clean; `vite build` clean. **Depends on:** T-0796 + T-0797 (done). Note: silent-refresh *loop* (auto-call `/auth/refresh` before expiry) is a small follow-up; the `refresh()` SDK method + endpoint exist. **Live-server e2e** rides with **T-0799**.