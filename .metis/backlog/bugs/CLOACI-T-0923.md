---
id: auth-lifecycle-edges-oidc-refresh
level: task
title: "Auth lifecycle edges — OIDC refresh is 501, no login brute-force defense"
short_code: "CLOACI-T-0923"
created_at: 2026-08-05T22:33:46.884336+00:00
updated_at: 2026-08-06T02:33:08.492306+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


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

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [x] OIDC sessions refresh without full re-auth; scope read from the persisted key row (never the request) so it cannot widen, and mint_for_principal still takes no is_admin param at all
- [x] Local logins throttled/locked out, DB-persisted so it holds across replicas, with audit + metric
- [x] 30s cross-replica key-revocation window documented in the security model
- [x] Tests: refresh happy path + scope preservation (is_admin=false asserted via whoami), lockout triggers/blocks-correct-password/expires, two-handle shared throttle state

## Status Updates

- 2026-08-05: Filed from the architecture deep dive (DEEPDIVE.md consolidated risk register #24; control-plane report S7.3-7.6). Item 3 is documentation-only by deliberate judgment.
- 2026-08-06: DONE — merged to main in PR #240 (squash). DESIGN DECISION on item 1: server-side session record, NOT custody of the IdP refresh token — that would need offline_access (not universally issued), lean on a token endpoint only RECOMMENDED to return a fresh id_token on the refresh grant, put the IdP on the hot path (outage = platform-wide forced logout), and mean holding a long-lived credential. Instead the callback opens an oidc_sessions row whose expires_at is the session's ABSOLUTE deadline (CLOACINA_OIDC_SESSION_MAX_AGE_S, default 8h); refresh re-mints inside it, rotates the session, revokes the old key. Trade (IdP-side deactivation unnoticed mid-session) is bounded by that deadline and documented. This also revived genuinely dead code — the T-0793 oidc_sessions table/DAL had exactly one caller: logout deleting a row nothing created. THROTTLE KEY: dual-keyed, either lock refuses — username key (5 strikes) catches IP rotation, IP key (50, 10x looser so shared NAT cannot self-lock) catches one host spraying many usernames; a composite (username, ip) key was rejected as strictly worse since it resets per source IP and stops neither attack. Success clears only the username key (clearing IP would let an attacker with one valid account zero their spray counter). Store errors fail OPEN deliberately — a brute-force mitigation must not become an availability kill-switch. BONUS FIX: closed a pre-existing username-enumeration TIMING ORACLE — authenticate() returned immediately for unknown users while a wrong password paid full argon2; it now burns an equivalent decoy verify, and unknown-username failures are throttled identically so the 429 is not an oracle either. NOTE these must ship together: unknown usernames now cost real CPU and the throttle is what bounds it. Migrations 046 (postgres-only; the auth strand is postgres-only) + 047 (both backends, unified throttle DAL). openapi.json + TS client regenerated. RESIDUALS (open): no oidc_sessions sweeper (opportunistic prune, same as ws_tickets); IP-scope defaults want a soak check; nothing warns at boot if a proxy deployment sets neither CLOACINA_TRUST_PROXY_HEADERS nor an IP threshold of 0.
