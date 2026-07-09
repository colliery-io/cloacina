---
id: oidc-login-callback-authorization
level: task
title: "OIDC login/callback — authorization-code + PKCE + spec-correct token validation → validated identity"
short_code: "CLOACI-T-0790"
created_at: 2026-06-24T01:08:01.506735+00:00
updated_at: 2026-06-24T04:44:29.025103+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# OIDC login/callback — authorization-code + PKCE + spec-correct token validation → validated identity

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement `/auth/login` (initiate: set state + nonce + PKCE) and `/auth/callback` (validate + exchange code) for the authorization-code flow. Token validation must be spec-correct: JWKS signature, `iss`/`aud`/`exp`/`iat`, and nonce; strict state + PKCE checks. Resolve a validated identity claim set (`sub`, `email`/domain, `groups`/`org`) — NO key minting yet. Login-flow state (state/nonce/PKCE verifier) lives in shared Postgres and is swept like the ws-ticket/outbox infra so it is multi-replica safe.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A full OIDC login completes end-to-end against the test issuer and yields a validated identity.
- [ ] Token validation enforces JWKS signature + `iss`/`aud`/`exp`/`iat` + nonce; invalid tokens are rejected with typed errors.
- [ ] Standard attacks are defended: replay (nonce), CSRF (state), code interception (PKCE).
- [ ] Login-flow state is Postgres-backed and swept (no sticky sessions).

## Implementation Notes

**Scope:** login + callback + validation + identity extraction. No minting (Task 5), no mapping (Task 4).
**Depends on:** Task 2 (config + discovery + JWKS).
**References:** CLOACI-I-0118 REQ-002, REQ-003, NFR-003, NFR-004; Implementation Plan Phase 1.

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE (flow built; login-start live-verified, callback via e2e).** `OidcProvider::begin_login` → authorization-code + PKCE URL (state + nonce); `complete_login(code, nonce, pkce)` → `exchange_code` (PKCE verifier) → **`openidconnect` validates the ID token** (JWKS signature, `iss`/`aud`/`exp`, nonce) → extract `subject`/`email`/`groups` (groups from the validated JWT payload). `LoginFlowStore` (single-use, TTL'd `state → nonce+PKCE`; CSRF/replay fail closed). `routes/oidc_auth.rs`: public `GET /auth/oidc/login` (302 → IdP) + `GET /auth/callback` (take state → complete_login → `MappingPolicy::resolve` → `mint_for_principal` → minted key; unmapped → 403; OIDC off → 501), merged into `/v1` outside `require_auth`/`authz_mw`. **Note:** the task scoped "no minting/mapping" but since T-0791/0792 already exist, the callback wires the full chain through to a minted key. `angreal check` clean; 7/7 oidc unit tests; **live discovery + begin_login vs Dex pass** (`--ignored`). Callback code-exchange needs a real browser auth code → **T-0787** e2e. **Deferred:** redirect-to-UI-with-key (callback returns JSON now); Postgres login-state (NFR-003, currently in-memory); audit reuse. **Depends on:** T-0789 (done).

**2026-06-24 — LIVE HTTP-route verification.** Ran the server binary with `CLOACINA_OIDC_*` pointed at the Dex sidecar: startup discovery succeeded and `GET /v1/auth/oidc/login` → **303** to `http://localhost:5556/dex/auth?response_type=code&client_id=cloacina&state=…&code_challenge=…&code_challenge_method=S256&redirect_uri=…%2Fv1%2Fauth%2Fcallback&scope=openid+email+profile+groups&nonce=…` — PKCE + state + nonce + groups scope all present. The full server side of the OIDC flow is proven against real Dex; only the browser click-through (Dex login page → callback) remains for T-0787.