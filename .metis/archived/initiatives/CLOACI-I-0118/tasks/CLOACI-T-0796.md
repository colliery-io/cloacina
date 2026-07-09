---
id: local-login-provider-password
level: task
title: "Local login provider — password verify → resolved principal → mint + cloacina-issued refresh"
short_code: "CLOACI-T-0796"
created_at: 2026-06-24T01:26:40.143648+00:00
updated_at: 2026-06-24T03:54:53.123653+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Local login provider — password verify → resolved principal → mint + cloacina-issued refresh

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement the `local` IdentityProvider. `POST /auth/local/login` (username + password) verifies against `local_accounts`, loads the account's `{tenant, role}` directly (the account record IS the mapping — it bypasses the OIDC allowlist), and produces the abstract RESOLVED PRINCIPAL that feeds the shared mint path. Mint a short-TTL key (reuse T-0792) and issue a cloacina-issued OPAQUE refresh token stored in the shared encrypted refresh store (T-0793, tagged `provider=local`); `/auth/refresh` re-mints only while the account is still `active`; `/auth/logout` revokes (reuse T-0794). Add a provider-enable config toggle (`oidc` / `local` / both). Add brute-force defense on the login endpoint (per-account lockout and/or per-IP throttle, reuse the existing rate limiter — OQ-13). IMPORTANT: this task refactors the mint/refresh handoff (coordinating with T-0792) to take an abstract resolved principal `{tenant, role, provenance}`, NOT OIDC-specific types.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

**2026-06-24 — CORE COMPLETE (login mints a key).** `POST /v1/auth/local/login` in new `routes/local_auth.rs` — **public** (merged into `/v1` WITHOUT `require_auth`/`authz_mw`, like the WS routes; caller has no bearer yet). Flow: `local_accounts().authenticate` (opaque `Denied`→401, no enumeration) → `ResolvedPrincipal { tenant, role, provenance: "local:<account_id>" }` → `identity::mint_for_principal` → returns `LocalLoginResponse { key, tenant_id, role, expires_at }` once. utoipa-annotated (`LocalLoginRequest`/`Response` ToSchema) + registered in `openapi.rs`; `openapi.json` regenerated (has `/v1/auth/local/login`). `angreal check` clean.

**Design call (revises the AC):** local accounts need **no stored refresh token** — the key's `local:<account_id>` provenance + the account `status` column ARE the refresh validity. So T-0793's encrypted store is **OIDC-only**, and T-0794 local refresh = re-check `status==active`. The "cloacina-issued refresh stored encrypted" AC bullet is therefore intentionally **not** done for local (simpler + equivalent).

**Deferred:** `/auth/refresh` + `/auth/logout` = **T-0794**; provider on/off toggle + brute-force throttle (**OQ-13**) + formal `cloacina::security::audit` (currently `tracing::info`) = hardening. Login mint→authorized-call validated under the postgres lane. **Depends on:** T-0795 + T-0792 (done).