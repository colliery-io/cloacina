---
id: cloacina-server-oidc
level: initiative
title: "cloacina-server OIDC authentication — bring-your-own-IdP login, server-side refresh, minted scoped keys"
short_code: "CLOACI-I-0118"
created_at: 2026-06-11T01:59:15.243679+00:00
updated_at: 2026-06-11T01:59:15.243679+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: cloacina-server-oidc
---

# cloacina-server OIDC authentication — bring-your-own-IdP login, server-side refresh, minted scoped keys Initiative

## Context **[REQUIRED]**

`cloacina-server` authenticates exclusively with API keys today: `Authorization: Bearer <key>`, validated against the hashed `api_keys` table with an LRU cache (`routes/auth.rs`). There is no user identity, no session, no OIDC/OAuth, no IdP integration anywhere in the server or `cloacina` core.

The tenant-scoped web UI (CLOACI-I-0117) needs human SSO login (Google/GitHub and enterprise IdPs), which the API-key model cannot provide. "OAuth login for the UI" is therefore **primarily a new server auth capability**, not a UI feature — the UI is the consumer. This initiative builds that capability; I-0117 consumes it.

**Framing decisions made in discovery (2026-06-10):**
- **API keys stay and remain primary** for programmatic access — CLI, SDKs, execution agents, automation all authenticate with keys. OIDC is **additive**: a human-login front door that resolves to the *existing* tenant + permission model.
- **Bring-your-own IdP, OIDC relying party.** cloacina-server is a configurable OIDC *relying party* (issuer URL + client credentials + JWKS discovery + claim mapping). Google, Keycloak, Auth0, Okta, Dex, Azure AD are all "just an OIDC issuer the operator configures." cloacina **does not bundle or operate an IdP** (no shipped Keycloak). GitHub (not OIDC) gets a small first-class OAuth2 path.
- **Hybrid credential model** (resolves I-0117's browser-auth need without a full session layer): a successful login resolves identity → tenant → role, then **mints a short-TTL cloacina API key** the browser uses via the existing bearer path; the IdP **refresh token is held server-side (encrypted)** and auto-renews the short-lived key via a refresh endpoint. This keeps **one** bearer auth model platform-wide — zero change to API auth middleware, SDKs, CORS, or CSRF posture — while delivering all-day login with the refresh token never touching the browser.
- **SCIM and bundled Keycloak are explicit non-goals.** SCIM (automated directory provisioning) is enterprise-end-state, deferred until a customer demands it. Operators who want enterprise SSO/SCIM bring their own IdP (Keycloak/Okta) — cloacina just speaks OIDC to it.

## Goals & Non-Goals **[REQUIRED]**

**Goals:**
- cloacina-server becomes a **configurable OIDC relying party**: authorization-code + PKCE browser flow against any OIDC issuer, token validation (signature via JWKS, `iss`/`aud`/`exp`, nonce/PKCE), claim extraction. Verified against at least two issuers (e.g. a dockerized Keycloak/Dex + Google).
- A **first-class GitHub OAuth2 path** (GitHub is OAuth2, not OIDC — access token + user API), modeled behind the same "identity provider" abstraction.
- **Hybrid credential issuance**: login → identity→tenant→role resolution → mint a **short-TTL** key (reusing the `api_keys` table + `generate_api_key`, tagged with OIDC provenance); store the IdP **refresh token server-side, encrypted**; a `/auth/refresh` endpoint silently re-mints before expiry; `/auth/logout` revokes the key and forgets the refresh token.
- A **pluggable identity→tenant/role mapping policy** (allowlist / org-or-domain / first-login-approval — chosen in design) so an operator controls how an OIDC identity becomes a cloacina principal.
- API-key auth **unchanged and primary**; OIDC is purely additive. No change to how the CLI/SDKs/agents authenticate.
- Security-reviewed, audit-logged (reuse the existing audit infra), multi-replica-safe (shared Postgres state), and documented (Diataxis how-to + config reference + an ADR for the hybrid-credential decision).

**Non-Goals:**
- **Bundling or operating an IdP.** No shipped Keycloak. Bring-your-own IdP via OIDC config.
- **SCIM / automated provisioning.** Deferred to a future enterprise initiative; it rides on top of this, not under it.
- **Full backend-for-frontend session cookies.** The hybrid keeps a short-lived browser-held key by design; eliminating the browser-held credential entirely (httpOnly cookie sessions, CSRF, SDK cookie mode, same-origin pull) is a possible *later* upgrade, explicitly out of scope here.
- **Replacing API keys** or redesigning the permission model. OIDC maps onto the existing tenant + role model; it does not introduce a new RBAC system, multi-user accounts, or org hierarchy.
- **The UI login flow itself.** The "Login with X" buttons, redirect handling, browser-side key storage (sessionStorage), and silent-refresh client logic live in **I-0117**. This initiative owns the server + the auth contract the UI calls.

## Requirements **[CONDITIONAL: Requirements-Heavy Initiative]**

### System Requirements

**Functional:**
- REQ-001: Configurable OIDC issuer(s) — issuer URL, client id/secret, scopes, redirect URI — via server config/env. OIDC discovery (`.well-known/openid-configuration`) + JWKS fetch/caching.
- REQ-002: Authorization-code + PKCE browser flow: `/auth/login` (initiate, set state+nonce+PKCE), `/auth/callback` (validate, exchange code), with strict state/nonce/PKCE checks.
- REQ-003: OIDC token validation per spec — JWKS signature, `iss`, `aud`, `exp`/`iat`, nonce. Reject on any failure with a typed error.
- REQ-004: First-class GitHub OAuth2 path behind the same provider abstraction (access-token + `GET /user` identity resolution).
- REQ-005: Identity→tenant→role resolution via a **pluggable mapping policy**; unmapped identities are denied (or land pending, per the chosen policy).
- REQ-006: Mint a **short-TTL** cloacina API key on successful login (reuse `api_keys` + `generate_api_key`), tagged with provenance (`issued_via = oidc:<issuer>:<sub>` or similar) and the resolved tenant + role. Returned to the caller once.
- REQ-007: **Server-side refresh-token custody** — store the IdP refresh token encrypted at rest (reuse the `aes-gcm` dependency already in `cloacina`); never log it; never return it to the browser.
- REQ-008: `/auth/refresh` — using the stored refresh token, re-mint (or extend) the short-TTL key without a full re-login, as long as the IdP session is valid.
- REQ-009: `/auth/logout` — revoke the minted key (existing revoke path) and forget/revoke the stored refresh token.
- REQ-010: Audit-log every login, refresh, and logout (reuse `cloacina::security::audit`).

**Non-Functional:**
- NFR-001: Refresh tokens encrypted at rest, never logged, never sent to the browser. Encryption-key sourcing is a documented config concern (see Open Questions).
- NFR-002: Minted keys are short-TTL and revocable; revocation latency bounded by the existing LRU cache TTL (parity with API-key revocation today).
- NFR-003: Multi-replica safe — all new state (refresh store, any login-flow state) lives in shared Postgres, swept like the WS-ticket / outbox infra; no sticky sessions.
- NFR-004: OIDC validation is spec-correct and defends the standard attacks (replay via nonce, CSRF via state, code interception via PKCE, token substitution via `aud`).
- NFR-005: Zero regression to API-key auth — existing auth tests and the SDK/CLI contract suites stay green; OIDC is a strictly additive code path.

## Use Cases **[CONDITIONAL: User-Facing Initiative]**

### Use Case 1: First login via Google (OIDC)
- **Actor**: a human with a tenant mapping configured
- **Scenario**: clicks "Login with Google" in the UI → redirected to Google → consents → back to `/auth/callback` → server validates the token, maps the identity to a tenant + role, mints a short-TTL key, stores the refresh token server-side, returns the key to the SPA.
- **Expected Outcome**: the user is in the UI with a working scoped key; the refresh token never touched the browser.

### Use Case 2: All-day session via silent refresh
- **Actor**: a logged-in user
- **Scenario**: the short-TTL key nears expiry; the SPA calls `/auth/refresh`; the server uses the stored refresh token to re-mint, returning a fresh key.
- **Expected Outcome**: the user stays logged in across the workday without re-consenting, and no long-lived credential ever sat in the browser.

### Use Case 3: Login via the operator's enterprise IdP (BYO)
- **Actor**: an operator running cloacina with Keycloak/Okta in front
- **Scenario**: configures cloacina with their issuer URL + client credentials + a group→tenant mapping; their users log in through their existing SSO.
- **Expected Outcome**: enterprise SSO works with no cloacina code changes and no bundled IdP — cloacina just spoke OIDC to their issuer.

### Use Case 4: Logout / revocation
- **Actor**: a logged-in user
- **Scenario**: clicks logout → server revokes the minted key + forgets the refresh token.
- **Expected Outcome**: the minted key stops working (within cache-TTL), and no server-side refresh material remains.

## Architecture **[CONDITIONAL: Technically Complex Initiative]**

### Overview
A new auth module in `cloacina-server` implements an OIDC relying party (likely the `openidconnect` crate) plus a GitHub OAuth2 path, both behind an `IdentityProvider` abstraction. The browser flow is authorization-code + PKCE. On success, an identity→tenant→role mapping policy resolves the principal; the server mints a short-TTL key via the existing DAL and stores the IdP refresh token in a new encrypted, Postgres-backed refresh store. A `/auth/refresh` endpoint renews; `/auth/logout` revokes. The existing bearer-key middleware is **untouched** — every authenticated API call still presents a key; OIDC just changes *how a human obtains one*.

### Reuse (don't rebuild)
- `api_keys` table + `generate_api_key` + revoke path (minted keys are ordinary keys with a short TTL + provenance).
- `aes-gcm` (already a `cloacina` dependency) for refresh-token encryption at rest.
- `cloacina::security::audit` for login/refresh/logout audit events.
- The WS-ticket / outbox sweeper pattern for expiring login-flow state and stale refresh records.
- The existing `AuthenticatedKey` + LRU cache — minted keys flow through it unchanged.

### Component sketch
```
SPA (I-0117)
  │  GET /auth/login?provider=google
  ▼
cloacina-server  ── IdentityProvider (oidc | github) ──► IdP (Google / Keycloak / GitHub / ...)
  │  /auth/callback → validate → MappingPolicy(identity) → tenant+role
  │  mint short-TTL key (api_keys)        store refresh token (encrypted, Postgres)
  │  return key ─────────────────────────────────────────────► SPA (sessionStorage, bearer)
  │  /auth/refresh → use stored refresh → re-mint key
  │  /auth/logout  → revoke key + forget refresh
  └─ existing bearer-key middleware unchanged for all /v1/* calls
```

## Detailed Design **[REQUIRED]**

### Decided
- **OIDC relying party, BYO IdP** (config-driven issuer) + first-class GitHub OAuth2.
- **Hybrid credential**: mint short-TTL key + server-side encrypted refresh + `/auth/refresh`.
- **Additive** to API-key auth; one bearer model platform-wide.

### To settle in design (see Open Questions)
- The identity→tenant/role **mapping policy** (allowlist vs org/domain vs first-login-approval), and whether it's one policy or pluggable.
- Refresh-token **storage shape** (extend `api_keys` vs a dedicated `oidc_sessions`/`refresh_tokens` table) and **encryption-key sourcing** (server config secret vs KMS/env).
- Short-TTL **value + refresh cadence**, and interaction with the LRU cache / revocation latency.
- **Multi-issuer** support (one configured issuer vs several simultaneously).
- GitHub: first-class path vs "federate GitHub through your IdP."

## Alternatives Considered **[REQUIRED]**

- **Full BFF session cookies (httpOnly).** Strongest posture (no browser-held credential), but adds a session store, CORS-with-credentials (pulls the UI toward same-origin deploy, undoing I-0117's separate-container decision), CSRF defense, a second server auth path, and a cookie mode in the TS SDK. Rejected for v1 as disproportionate; preserved as a clean later upgrade. The hybrid keeps the refresh token server-side (the main security win) without the rest of the cost.
- **Mint short-lived key with no refresh.** Simplest, but forces re-login every 15–60 min — too much friction for a sit-in-it-all-day control plane. Rejected in favor of server-side refresh.
- **Bundle/require Keycloak.** Most enterprise-ready (federation + SCIM later), but makes cloacina operate a stateful IAM service — overkill for self-hosted/SMB and unnecessary when OIDC-RP lets operators bring Keycloak themselves. Rejected as a default; fully compatible as an operator choice.
- **Direct Google + GitHub only (no generic OIDC).** Simpler, but bakes in per-vendor code and doesn't serve "bring our enterprise SSO." Rejected in favor of generic OIDC-RP + a GitHub special case.
- **Replace API keys with OIDC.** Infeasible — the CLI, SDKs, and agents authenticate with keys. OIDC is additive by necessity.

## Implementation Plan **[REQUIRED]**

Candidate phases (task batches on decomposition):

1. **OIDC relying-party foundation.** Provider config + discovery + JWKS; authorization-code + PKCE; `/auth/login` + `/auth/callback`; spec-correct token validation; resolve a raw identity claim (no minting yet). Exit: complete an OIDC login against a dockerized test IdP and extract a validated identity.
2. **Mapping + key minting.** The identity→tenant/role mapping policy; mint a short-TTL key via the DAL with provenance. Exit: login yields a working scoped key authorized exactly like an equivalent manual key.
3. **Refresh + lifecycle.** Encrypted server-side refresh store; `/auth/refresh`; `/auth/logout` (revoke + forget); TTL + sweeper. Exit: all-day login with silent refresh; logout fully revokes.
4. **GitHub OAuth2 path.** The non-OIDC provider behind the same abstraction. Exit: "Login with GitHub" resolves an identity and mints a key.
5. **Hardening, tests, docs.** Live auth e2e (dockerized Keycloak/Dex in the test compose), security review (`/security-review`), zero-regression check against API-key auth + SDK/CLI suites, Diataxis how-to + config reference, and an ADR for the hybrid-credential decision. Exit: e2e green, security-reviewed, documented; lockstep release.

Sequencing: Phases 1→3 are the spine (login → key → all-day refresh). Phase 4 (GitHub) parallels once the provider abstraction from Phase 1 exists. **I-0117 dependency:** the UI can ship its API-key `/connect` gate immediately, but its OIDC "Login with X" feature depends on this initiative reaching ~Phase 3 (mint + refresh) with a stable `/auth/*` contract.

## Open Questions **[resolve in design]**

- **OQ-1 — Mapping policy**: allowlist (explicit identity→tenant/role) vs org/domain auto-map (GitHub org / Google Workspace domain → tenant) vs first-login-approval (pending until an admin assigns). One fixed policy or pluggable? This is the product crux that sizes the whole thing.
- **OQ-2 — Refresh storage + encryption-key sourcing**: dedicated table vs extend `api_keys`; where the AES key comes from (config secret, env, KMS) and how it rotates.
- **OQ-3 — Short-TTL value + refresh cadence**, and revocation-latency interaction with the LRU cache.
- **OQ-4 — Multi-issuer**: support several configured issuers at once (e.g. Google + the operator's Keycloak), or one at a time?
- **OQ-5 — GitHub scope**: first-class direct path, or "federate GitHub through your IdP" only? (Leaning first-class given GitHub's ubiquity and non-OIDC nature.)
- **OQ-6 — OIDC library**: `openidconnect` crate vs hand-rolled over `oauth2` + JWKS. Spike in Phase 1.
