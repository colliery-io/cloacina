---
id: local-login-provider-password
level: task
title: "Local login provider — password verify → resolved principal → mint + cloacina-issued refresh"
short_code: "CLOACI-T-0796"
created_at: 2026-06-24T01:26:40.143648+00:00
updated_at: 2026-06-24T01:26:40.143648+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Local login provider — password verify → resolved principal → mint + cloacina-issued refresh

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement the `local` IdentityProvider. `POST /auth/local/login` (username + password) verifies against `local_accounts`, loads the account's `{tenant, role}` directly (the account record IS the mapping — it bypasses the OIDC allowlist), and produces the abstract RESOLVED PRINCIPAL that feeds the shared mint path. Mint a short-TTL key (reuse T-0792) and issue a cloacina-issued OPAQUE refresh token stored in the shared encrypted refresh store (T-0793, tagged `provider=local`); `/auth/refresh` re-mints only while the account is still `active`; `/auth/logout` revokes (reuse T-0794). Add a provider-enable config toggle (`oidc` / `local` / both). Add brute-force defense on the login endpoint (per-account lockout and/or per-IP throttle, reuse the existing rate limiter — OQ-13). IMPORTANT: this task refactors the mint/refresh handoff (coordinating with T-0792) to take an abstract resolved principal `{tenant, role, provenance}`, NOT OIDC-specific types.

## Acceptance Criteria **[REQUIRED]**

- [ ] `POST /auth/local/login` with valid creds mints a working scoped key (authorized identically to any key); invalid creds → typed 401; disabled account → denied.
- [ ] Cloacina-issued refresh stored encrypted with `provider=local`; `/auth/refresh` re-mints while active; a disabled/deleted account fails refresh.
- [ ] `/auth/logout` revokes key + refresh.
- [ ] The provider toggle gates whether local login is enabled.
- [ ] Brute-force throttle/lockout on the login endpoint.
- [ ] Login / refresh / logout are audit-logged.
- [ ] Integration tests: login→authorized call; refresh-while-active; disabled-account-blocked.

## Implementation Notes

**Scope:** the local login provider + the provider-agnostic mint handoff + config toggle + brute-force defense. Reuses T-0792/T-0793/T-0794.
**Depends on:** Task 1 / CLOACI-T-0795 (credential store); T-0792 (minting), T-0793 (refresh store), T-0794 (refresh/logout).
**References:** CLOACI-I-0118 → "Local accounts strand"; OQ-13, OQ-14; local login is credential-over-HTTP and REQUIRES TLS (document).

## Status Updates **[REQUIRED]**

*To be added during implementation*
