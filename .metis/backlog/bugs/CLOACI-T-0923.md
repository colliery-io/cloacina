---
id: auth-lifecycle-edges-oidc-refresh
level: task
title: "Auth lifecycle edges — OIDC refresh is 501, no login brute-force defense"
short_code: "CLOACI-T-0923"
created_at: 2026-08-05T22:33:46.884336+00:00
updated_at: 2026-08-05T22:33:46.884336+00:00
parent: 
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/backlog"
  - "#bug"


exit_criteria_met: false
initiative_id: NULL
---

# Auth lifecycle edges — OIDC refresh is 501, no login brute-force defense

## Objective

Close the two auth-lifecycle gaps the deep dive found (risk register #24, MEDIUM). One is an unfulfilled use case from I-0118; the other is an explicitly-deferred open question ("required before production") that is still open.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: browser/UI users of OIDC SSO (hard logout every 15 minutes with no refresh) and any internet-exposed deployment using local password accounts (unthrottled credential stuffing).

## Findings

1. OIDC REFRESH RETURNS 501. Logins mint 15-minute API keys; there is no refresh path, so an SSO session dies hard at 15 minutes and the user must re-authenticate through the whole flow. I-0118 UC2 ("stay signed in") is unfulfilled. The refresh route exists but is unimplemented — verify current shape in crates/cloacina-server (routes/oidc_auth.rs / session.rs) before designing.
2. NO BRUTE-FORCE DEFENSE ON `/v1/auth/local/login`. Argon2id hashing makes each attempt expensive server-side, but nothing rate-limits or locks out repeated failures against a known username; I-0118 OQ-13 flagged this as "required before production" and it was never resolved.
3. ACCEPTED, NOT A BUG: cross-replica API-key revocation lag is bounded by the 30s identity LRU cache TTL. The deep dive judged this a reasonable trade; document it rather than change it (a revoked key remaining valid for <=30s on other replicas should be stated in the security docs).

## Proposed shape (adjust on implementation)

- Refresh: mint a new short-lived key against the still-valid session/refresh material, rotating the old one; respect the same god/tenant scoping rules as the original mint (every mint path hard-codes is_admin=false — preserve that). Consider whether the OIDC provider's own refresh token should be stored (encrypted) or whether a server-side session record suffices.
- Throttle: per-(username, source-IP) failure counter with exponential backoff or temporary lockout, persisted (multi-replica correctness — mirror the T-0916 pattern of DB-backed state rather than per-replica maps), with a metric and audit events on lockout.
- Document the 30s revocation window in the security model docs.

## Acceptance Criteria

- [ ] OIDC sessions can refresh without a full re-auth; refresh respects tenant/role scoping and cannot escalate to god
- [ ] Repeated failed local logins are throttled/locked out, enforced consistently across replicas, with audit + metric
- [ ] The 30s cross-replica key-revocation window is documented in the security model
- [ ] Tests: refresh happy path + scope preservation; lockout triggers and expires; throttle state shared across two server handles

## Status Updates

- 2026-08-05: Filed from the architecture deep dive (DEEPDIVE.md consolidated risk register #24; control-plane report S7.3-7.6). Item 3 is documentation-only by deliberate judgment.
