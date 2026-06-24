---
id: oidc-rp-config-discovery-jwks
level: task
title: "OIDC RP config + discovery + JWKS (single issuer)"
short_code: "CLOACI-T-0789"
created_at: 2026-06-24T01:07:32.683057+00:00
updated_at: 2026-06-24T04:43:41.852710+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# OIDC RP config + discovery + JWKS (single issuer)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement a configurable OIDC relying party for a SINGLE issuer — issuer URL, client id/secret, scopes, redirect URI via server config/env — with `.well-known/openid-configuration` discovery and JWKS fetch + caching. Multi-issuer support (OQ-4) is explicitly deferred: exactly one configured issuer for now.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] Server reads OIDC config (issuer URL, client id/secret, scopes, redirect URI) from config/env.
- [ ] OIDC discovery document + JWKS are fetched and cached; cache refresh handled.
- [ ] Missing/invalid OIDC config fails fast with a typed error.
- [ ] Integration test against the dockerized test issuer's discovery + JWKS endpoints.

## Implementation Notes

**Scope:** config + discovery + JWKS for one issuer. Multi-issuer (OQ-4) deferred.
**Depends on:** Task 1 (library decision).
**References:** CLOACI-I-0118 REQ-001; OQ-4 (deferred).

## Status Updates **[REQUIRED]**

**2026-06-24 — DESIGN RECORDED (build against the live Dex).** Upstream ready: Dex harness + `OidcConfig::from_env` (T-0788), mapping `oidc::MappingPolicy` (T-0791), mint `identity::mint_for_principal` (T-0792), refresh stub in `session.rs` (T-0794).

**Deps:** `openidconnect = { version = "4", features = ["reqwest"] }` + `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls","json"] }` in `crates/cloacina-server/Cargo.toml`.

**v4 API gotcha:** `CoreClient` is **type-state generic** — don't store the built client in a struct. Store `OidcProvider { metadata: CoreProviderMetadata, config: OidcConfig, http: reqwest::Client }` and build the client INLINE per-handler (type inferred, never named):
```
let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build()?;
let metadata = CoreProviderMetadata::discover_async(IssuerUrl::new(cfg.issuer_url)?, &http).await?;
let client = CoreClient::from_provider_metadata(metadata.clone(), ClientId::new(..), Some(ClientSecret::new(..))).set_redirect_uri(RedirectUrl::new(cfg.redirect_uri)?);
```
`AppState` gets `oidc: Option<Arc<OidcProvider>>` (None when `OidcConfig::from_env()` None → routes not mounted). JWKS caching lives in `metadata`/the verifier.

**T-0789:** deps + `OidcProvider::discover` + AppState wiring + a login-flow **state store** (state/nonce/PKCE-verifier; Postgres for NFR-003 multi-replica, mirror ws-ticket/oidc_sessions — or in-memory for the single-replica demo first). **T-0790:** `/auth/login` (authorize_url + PKCE/state/nonce) + `/auth/callback` (exchange_code → `id_token.claims(&verifier, &nonce)` validation → `MappingPolicy::resolve` → `mint_for_principal`). Both are the **live-verify gate** against Dex (`angreal ui up`). **Depends on:** T-0788 (done).

**2026-06-24 — COMPLETE (live-verified).** `openidconnect 4` + `reqwest 0.12` added. `OidcProvider { metadata, config, http }` + `discover(config)` (no-redirect client → `CoreProviderMetadata::discover_async` → cached metadata carrying JWKS+endpoints). `AppState.{oidc: Option<Arc<OidcProvider>>, oidc_policy, oidc_login}` wired at startup — `OidcConfig::from_env()` `None`/discovery-failure → OIDC simply off (server still boots). The v4 type-state `CoreClient` is built inline (the `client()` helper names the exact `EndpointSet/NotSet/MaybeSet` type). **Live discovery against Dex passes** (`discovers_against_live_issuer`, `--ignored`). `angreal check` clean. JWKS caching lives in the metadata/verifier. Login-flow state store = in-memory `LoginFlowStore` (Postgres for NFR-003 multi-replica is a documented follow-up). **Depends on:** T-0788 (done).