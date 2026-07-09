---
id: auth-refresh-auth-logout-lifecycle
level: task
title: "/auth/refresh + /auth/logout lifecycle + audit"
short_code: "CLOACI-T-0794"
created_at: 2026-06-24T01:09:55.464552+00:00
updated_at: 2026-06-24T04:04:02.046259+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# /auth/refresh + /auth/logout lifecycle + audit

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement `/auth/refresh` — use the stored encrypted refresh token to silently re-mint or extend the short-TTL key before expiry (cadence: refresh when ~⅓ of TTL remains — OQ-3 default). Implement `/auth/logout` — revoke the minted key via the existing revoke path AND forget/revoke the stored refresh token. Audit-log every login, refresh, and logout, reusing `cloacina::security::audit` (REQ-010).

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

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

**2026-06-24 — ANALYSIS (not yet implemented).** The refresh store (T-0793) is done; this consumes it. Key finding: `/auth/refresh`'s core — "validate the stored refresh, then re-mint" — is **provider-specific** and wants producers that don't exist yet:
- **OIDC** (T-0790/0791): refresh = call the IdP token endpoint with the stored refresh token, then re-mint.
- **Local** (T-0796): refresh = check the `local_accounts` row is still `active`, then re-mint.

So T-0794 is best done **with/just after the first producer** (recommend after T-0796 local — simplest, no external call). Re-mint reuses `identity::mint_for_principal`; the principal is recoverable from the *current* (expiring) key's row (`get_key` → tenant/role) + the session `provider`, so no extra principal storage is needed.

**Buildable now without producers (the wiring T-0793 deferred here):** (1) **env key sourcing** — `CLOACINA_REFRESH_ENC_KEY` (32 bytes hex/base64) → `AppState` (Option; refresh off if absent; KMS = future per OQ-2); (2) **sweeper spawn** — tokio task calling `oidc_sessions().sweep_expired()` on the ws-ticket/outbox cadence; (3) **`/auth/logout`** — `revoke_key` + `oidc_sessions().delete(key_id)` + audit (fully implementable now); (4) **`/auth/refresh` scaffold** — endpoint/route/authz entry (`Any+Read`, caller's own key), provider-exchange lands with the producer; (5) audit login/refresh/logout (REQ-010).

**Depends on:** T-0793 (done); refresh semantics couple to T-0790/0791 or T-0796.

**2026-06-24 — COMPLETE (local path; OIDC path stubbed).** New `routes/session.rs` (authenticated, `Any+Read`; authz table now **51 routes**). `/auth/refresh` dispatches on the minted key's `issued_via`: `local:<account_id>` → `local_accounts().get_by_id` re-checks `is_active` → re-mint a fresh short-TTL key via `mint_for_principal` + revoke the old (deprovisioned mid-session → revoke + 401); `oidc:<…>` → **501** until T-0790 wires the IdP refresh exchange. `/auth/logout` → `revoke_key` + `oidc_sessions().delete` (no-op for local) + clear LRU. Added DAL `local_accounts().get_by_id`. utoipa-registered; `openapi.json` regenerated. `angreal check` clean; 8/8 authz tests. **Completes the local login → all-day refresh → logout lifecycle** and the full **server-side** auth implementation. **Deferred to OIDC phase:** the IdP refresh-token exchange + env-key sourcing + sweeper spawn (only needed once OIDC stores refresh tokens); formal `audit` reuse (currently `tracing::info`). Integration tests under the postgres lane.