---
id: cloacina-server-oidc
level: initiative
title: "cloacina-server OIDC authentication — bring-your-own-IdP login, server-side refresh, minted scoped keys"
short_code: "CLOACI-I-0118"
created_at: 2026-06-11T01:59:15.243679+00:00
updated_at: 2026-06-23T01:55:13.060633+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/design"


exit_criteria_met: false
estimated_complexity: L
initiative_id: cloacina-server-oidc
---

# cloacina-server OIDC authentication — bring-your-own-IdP login, server-side refresh, minted scoped keys Initiative

## Context **[REQUIRED]**

`cloacina-server` authenticates exclusively with API keys today: `Authorization: Bearer <key>`, validated against the hashed `api_keys` table with an LRU cache (`routes/auth.rs`). There is no user identity, no session, no OIDC/OAuth, no IdP integration anywhere in the server or `cloacina` core.

The tenant-scoped web UI (CLOACI-I-0117) needs human SSO login (Google/GitHub and enterprise IdPs), which the API-key model cannot provide. "OAuth login for the UI" is therefore **primarily a new server auth capability**, not a UI feature — the UI is the consumer. This initiative builds that capability; I-0117 consumes it.

**Scope expansion (2026-06-22): authorization model folded in.** A UI-auth review surfaced that cloacina's *authentication* design here is sound, but its *authorization* model is too thin for a human-facing control plane. Today authZ is two non-composing axes: a `tenant_id` scope on the key (NULL = global) and a linear `read < write < admin` role, plus a separate `is_admin` god-mode boolean. Concrete gaps: (a) **no real tenant-admin** — managing a tenant's own keys requires cross-tenant god-mode (`routes/keys.rs:234`), because `is_admin` is a global flag with no relational attribute; (b) **no per-resource conditions** (e.g. "execute only workflows tagged X", "an agent fetches artifacts only for its assigned tenant"); (c) the OIDC identity→tenant→role mapping (OQ-1) is itself attribute logic with no home. This initiative now also **replaces the role model with a hand-rolled, in-process attribute-based access control (ABAC) matcher** onto which OIDC maps. Decisions: **ABAC, not RBAC** — decisions are conditions over subject/resource/action attributes; roles survive only as named bundles of attribute rules (a "tenant-admin" role is sugar for the policy `subject.tenant == resource.tenant && subject.role == admin`). **Hand-rolled matcher in Rust, in-process** — explicitly NOT Cedar and NOT OPA/Rego: a policy-engine sidecar is server-only and breaks the embedded-first philosophy, and a per-request Rego round-trip is the wrong direction given known scheduler-throughput pressure (CLOACI-T-0745). **This layer lives ONLY in `cloacina-server`**, attached as an authZ middleware on top of the existing `require_auth` authN middleware — the authZ checks already live there (`AuthenticatedKey::can_*` in `routes/auth.rs`). The embedded `cloacina` runtime never mounts it (no HTTP, no tenants, no keys), so the earlier "no-op when embedded" contract collapses to "the layer is simply not present." The matcher evaluates cheaply behind the existing 30s LRU cache. **Placement — a single fail-closed route-table middleware (decided 2026-06-22).** Because object/sub-tenant isolation is a permanent non-goal (see Non-Goals), every authZ decision is URL-derivable — `tenant_id` from the path, action level from method+route, subject role from the key — so all of it lives in **one authZ middleware** layered after `require_auth`, driven by a **declarative route table** (`(method, route-pattern) -> Access`). An unclassified route is **denied by default** (fail-closed), and the whole authorization policy is readable in one file. We rejected the per-handler-helper and hybrid placements: their only advantage over the route table was reaching a loaded resource row for per-instance conditions, which this product never needs. No middleware DB pre-fetch, no response post-filter — both were considered and rejected (a pre-fetch needs a per-route resolver registry + double-fetches the row, and a post-filter breaks pagination and is filter-on-egress); neither is necessary once object-level authZ is off the table.

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
- **Replace the linear role model with a hand-rolled in-process ABAC matcher.** Authorization decisions become `evaluate(subject, action, resource, env) -> Permit | Deny` over attributes — subject `{tenant_id, role, is_admin, provenance}`, resource `{tenant, kind, name, version, status, tags}`, action `{read|write|admin|execute|...}`. Ship a **real per-tenant-admin** capability (admin within one tenant, powerless outside it) that today's global `is_admin` boolean cannot express. Every existing call-site check (`can_access_tenant` / `can_write` / `can_admin`) routes through the matcher; god-mode and the current roles are re-expressed as named policy bundles so behavior is preserved by construction. The matcher is in-process, cheap (sits behind the 30s LRU cache), and **scoped entirely to `cloacina-server` — mounted as authZ middleware, never compiled into or run by the embedded core**. OIDC claim→principal mapping (OQ-1) is expressed in the same attribute vocabulary.
- Security-reviewed, audit-logged (reuse the existing audit infra), multi-replica-safe (shared Postgres state), and documented (Diataxis how-to + config reference + an ADR for the hybrid-credential decision).

**Non-Goals:**
- **Bundling or operating an IdP.** No shipped Keycloak. Bring-your-own IdP via OIDC config.
- **SCIM / automated provisioning.** Deferred to a future enterprise initiative; it rides on top of this, not under it. **Why deferral is cheap:** SCIM's two real wins are a pre-login roster and *active* deprovisioning (instant push-revoke when an identity leaves the IdP). The short-TTL + server-side-refresh design already bounds deprovisioning latency to minutes — a deactivated identity fails its next `/auth/refresh` and the minted key dies within the TTL — so SCIM degrades from "prerequisite" to "tighten minutes→seconds + pre-login roster." It stays out until a customer needs that tighter guarantee.
- **Full backend-for-frontend session cookies.** The hybrid keeps a short-lived browser-held key by design; eliminating the browser-held credential entirely (httpOnly cookie sessions, CSRF, SDK cookie mode, same-origin pull) is a possible *later* upgrade, explicitly out of scope here.
- **Replacing API keys.** Keys stay primary for CLI/SDK/agents; OIDC and the ABAC matcher both resolve to a bearer key. (Note: redesigning the *permission* model into ABAC is now **in scope** — see the scope-expansion note above. What stays out is a full multi-user-account system and org hierarchy: the ABAC subject is still a key, optionally carrying OIDC provenance as an attribute, not a first-class `User` entity with its own lifecycle.)
- **A general policy engine / external authZ service.** No Cedar, no OPA/Rego, no sidecar. The matcher is a small hand-rolled in-process Rust component; policies are not operator-authored DSL in v1 (a policy-as-config story is a possible later upgrade, explicitly out of scope here).
- **Object/sub-tenant isolation (workflow-level, execution-ownership, per-resource ACLs).** A **permanent non-goal**, not merely deferred. The product's isolation boundary is the **tenant**, and tenants are intentionally cheap to create: to isolate a set of workflows you **spin up a new tenant**, and an individual simply belongs to multiple tenants. Consequence: no authorization decision ever depends on a loaded resource row, so the matcher stays **coarse forever** (tenant + level, all URL-derivable) and there is no per-subject list filtering (anyone who can reach a tenant sees all of that tenant's resources). This is what makes a fail-closed route-table middleware both sufficient and future-proof.
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
- **ABAC authorization via a hand-rolled in-process matcher** (not Cedar, not OPA), **server-only — attached as authZ middleware in `cloacina-server`, never in the embedded core**. Roles become named policy bundles; real per-tenant-admin ships; existing `can_*` checks route through `evaluate(subject, action, resource, env)`. OIDC maps into the same attribute vocabulary.

### Provisioning model (decided 2026-06-22)
The two provisioning flows are **different in kind** — only one is an identity concern:
- **God tier provisions tenants** — a tenant is a Postgres schema + runner, i.e. *infrastructure*, not an identity. This is **never an IdP/SCIM operation** (an IdP has no tenant concept; a SCIM "Group" is a bag of users, not a schema). It stays a god-tier control-plane call (`POST /tenants`) that also drops a bootstrap tenant-admin. Scriptable, but it's cloacina's own API.
- **Tenant admins provision users** — since the subject is a key (no `User` entity), "provisioning a user" is either: (a) **IdP/JIT path (primary)** — the god tier wires up one shared IdP and defines **mapping rules** (`IdP group acme-engineers → tenant acme, role write`); membership is delegated to the IdP; a user in the group logs in and a short-TTL key is minted just-in-time. No per-user provisioning, **no SCIM**. Or (b) **manual path** — mint a tenant-scoped key (`POST /tenants/{t}/keys`, now tenant-admin-gated) for shops with no IdP.

**"God tier handles it all" = single shared IdP + god-owned mappings.** That is the easy, recommended path: the god tier owns the IdP relationship and the group→tenant/role mappings; thereafter user lifecycle collapses into IdP group membership. SCIM is not required for any of this (see Non-Goals for why the short-TTL model already covers most of SCIM's deprovisioning value).

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

### Authorization-engine alternatives (2026-06-22)
- **Cedar (embeddable, Rust-native policy language).** Formally-analyzed, policies-as-data, in-process — the strongest "real ABAC" option that still respects embedded-first. Rejected for v1 because it adds a dependency + a DSL + an entity/schema modeling burden for a policy set that is currently small and closed (a handful of tenant/role/resource rules). Kept as a clean later upgrade if policy complexity or operator-authored policy demand grows — the matcher's `evaluate()` boundary is designed so Cedar could slot in behind it without touching call sites.
- **OPA / Rego (external policy engine).** Industry standard and fully decoupled, but a server-only sidecar that breaks embedded-first outright and adds a per-request network round-trip into the hot path against known scheduler-throughput pressure (CLOACI-T-0745). Rejected.
- **Keep linear RBAC, just add a tenant-admin role.** Cheapest, but the role explosion starts immediately (tenant-admin, then resource-scoped roles, then agent-scoped) and never composes — every new condition is a new role. The real decisions are attribute conditions (`subject.tenant == resource.tenant`), so ABAC is the honest model. Rejected.
- **Chosen: hand-rolled in-process matcher.** Smallest thing that expresses the real attribute conditions, owns its correctness, has zero new runtime dependencies, runs identically embedded and server (no-op when there's no tenant model), and leaves a clean seam to adopt Cedar later if warranted.

## Implementation Plan **[REQUIRED]**

Candidate phases (task batches on decomposition):

0. **ABAC authorization middleware (foundation, precedes mint+mapping).** Implement the coarse in-process matcher `evaluate(principal, access) -> Permit | Deny(reason)` over `principal { tenant: Option<String>, role: Level, platform_admin: bool }` and `access { scope: Platform | Tenant(String), level: Read|Write|Admin }` (default-deny; god-mode short-circuits to Permit). Build the **declarative route table** `(method, route-pattern) -> Access` covering every current route, and mount it as a single authZ middleware after `require_auth`, replacing the scattered `can_access_tenant`/`can_write`/`can_admin` if-blocks. Re-express god-mode + `read/write/admin` as the table's Access values so existing behavior is preserved by construction (characterization tests against today's matrix first). **One intentional behavior change**: move key-management (`GET/POST/DELETE /auth/keys`) out of the unscoped "admin" bucket into `Tenant(own) + Admin`, fixing the current cross-tenant leak where a tenant `role=admin` key can list/revoke *any* tenant's keys (requires `list_keys`/`revoke_key` to gain a tenant filter + ownership check). Add the **per-tenant-admin** capability. Note: data-scoping that *widens* output by god-mode (e.g. `health_graphs`) stays in the handler — the middleware gates access, the handler shapes the payload. **Execution-agent endpoints:** agent *dispatch* is already tenant-isolated and stays as-is — registration pins `agent.tenant_id` from the registering key (`routes/agent.rs:87`) and the fleet executor only selects agents whose tenant matches the task's, failing dispatch otherwise (`fleet_executor.rs:269-282`, fleet REQ-008). The fail-closed route table must still classify every agent route, and three drop `_auth` today and need tightening: `POST /agent/heartbeat` + `POST /agent/result` act on body `agent_id`/`task_execution_id` with no caller-tenant check — add a cheap in-memory guard that the caller key's tenant equals the registered agent's tenant; `GET /agent/artifact|source/{digest}` is content-addressed and unscoped (low-risk — agents only learn their own dispatched digests — but not tenant-enforced); `GET /agents` is god-mode-only and should become `Tenant(own) + Admin` filtered to that tenant's roster. Exit: existing authZ tests green through the middleware; an unclassified route is denied by default; a tenant-admin manages only its own tenant's keys. Independently shippable — improves authZ (and closes the leak) before any OIDC lands.

1. **OIDC relying-party foundation.** Provider config + discovery + JWKS; authorization-code + PKCE; `/auth/login` + `/auth/callback`; spec-correct token validation; resolve a raw identity claim (no minting yet). Exit: complete an OIDC login against a dockerized test IdP and extract a validated identity.
2. **Mapping + key minting.** The identity→tenant/role mapping policy; mint a short-TTL key via the DAL with provenance. Exit: login yields a working scoped key authorized exactly like an equivalent manual key.
3. **Refresh + lifecycle.** Encrypted server-side refresh store; `/auth/refresh`; `/auth/logout` (revoke + forget); TTL + sweeper. Exit: all-day login with silent refresh; logout fully revokes.
4. **GitHub OAuth2 path.** The non-OIDC provider behind the same abstraction. Exit: "Login with GitHub" resolves an identity and mints a key.
5. **Hardening, tests, docs.** Live auth e2e (dockerized Keycloak/Dex in the test compose), security review (`/security-review`), zero-regression check against API-key auth + SDK/CLI suites, Diataxis how-to + config reference, and an ADR for the hybrid-credential decision. Exit: e2e green, security-reviewed, documented; lockstep release.

Sequencing: Phases 1→3 are the spine (login → key → all-day refresh). Phase 4 (GitHub) parallels once the provider abstraction from Phase 1 exists. **I-0117 dependency:** the UI can ship its API-key `/connect` gate immediately, but its OIDC "Login with X" feature depends on this initiative reaching ~Phase 3 (mint + refresh) with a stable `/auth/*` contract.

## Open Questions **[resolve in design]**

- **OQ-1 — Mapping policy** (now an ABAC concern): how IdP claims (`sub`, `email`/domain, `org`, `groups`) resolve to the ABAC subject attributes (`tenant_id`, `role`). Options: allowlist (explicit identity→tenant/role) vs org/domain auto-map vs first-login-approval (pending until an admin assigns). One fixed policy or pluggable? Still the product crux. It now lands in the same attribute vocabulary the matcher uses, so "mapping" and "authorization" share one model. **Multi-tenant individuals**: because the isolation boundary is the tenant and an individual may belong to several tenants, a login resolves to a *set* of `{tenant, role}` memberships. This is handled entirely at mapping + UI: mint **one tenant-scoped key per tenant**, and the UI switches tenant context by swapping which scoped key it presents. The subject stays a single-tenant key — no multi-tenant subject, no change to the matcher — which confirms the "subject is a key, not a user" scope line.
- **OQ-7 — ABAC matcher shape**: the concrete Rust representation of subject/resource/action/env attributes and a policy (enum of typed rules vs a small condition AST vs a list of `Fn(ctx) -> Decision`); where the policy set is defined (compiled-in constants for the closed v1 set vs server config); and `Permit`/`Deny`/default-deny + combining semantics.
- ~~**OQ-8 — Resource attribute sourcing**~~ **RESOLVED (2026-06-22)**: moot — object-level authZ is a permanent non-goal, so no decision needs a loaded resource row. The only resource attributes a decision uses (`tenant`, `kind`, action) are all URL-derivable in the middleware. No pre-fetch, ever.
- ~~**OQ-9 — Embedded degradation**~~ **RESOLVED (2026-06-22)**: moot — the matcher is server-only middleware in `cloacina-server`; the embedded core never mounts it, so there is no in-core no-op contract to design.
- **OQ-10 — Tenant-admin reach**: precisely which actions a per-tenant-admin gains (manage own-tenant keys, members, workflows, pause/resume) vs what remains god-mode only (create/delete tenants). This is the user-visible payoff of the redesign.
- **OQ-11 — Mapping ownership** (provisioning model): who edits the `IdP group → tenant/role` mappings — **god-only** (simplest; god wires every tenant's rules — matches the "god tier handles it all" assumption now pinned in Detailed Design) vs **tenant-admin self-serve** (each tenant-admin manages their own tenant's mappings — more product surface, still no SCIM). Couples to **OQ-4** (single shared IdP vs per-tenant multi-issuer): "god handles it all" implies one shared IdP, the easy case. Default assumption for design: **single shared IdP + god-owned mappings**; revisit only if a tenant-self-serve requirement appears.
- **OQ-2 — Refresh storage + encryption-key sourcing**: dedicated table vs extend `api_keys`; where the AES key comes from (config secret, env, KMS) and how it rotates.
- **OQ-3 — Short-TTL value + refresh cadence**, and revocation-latency interaction with the LRU cache.
- **OQ-4 — Multi-issuer**: support several configured issuers at once (e.g. Google + the operator's Keycloak), or one at a time?
- **OQ-5 — GitHub scope**: first-class direct path, or "federate GitHub through your IdP" only? (Leaning first-class given GitHub's ubiquity and non-OIDC nature.)
- **OQ-6 — OIDC library**: `openidconnect` crate vs hand-rolled over `oauth2` + JWKS. Spike in Phase 1.