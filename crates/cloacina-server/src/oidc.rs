/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

// CLOACI-T-0791: the mapping policy has no live producer until the OIDC RP
// (T-0789/0790) extracts claims. Allow the interim dead_code.
#![allow(dead_code)]

//! OIDC identity → cloacina principal mapping (CLOACI-T-0791, resolves OQ-1).
//!
//! A validated OIDC identity (subject + email + groups) is resolved to an
//! abstract [`ResolvedPrincipal`](crate::identity::ResolvedPrincipal) — the
//! same provider-agnostic handoff local login produces — by a **god-owned,
//! config-driven allowlist** (OQ-1 default, OQ-11 god-owned). An identity that
//! matches no rule is **denied** (returns `None`); there is no implicit access.
//!
//! The allowlist is the simplest policy that serves "the god tier wires up one
//! shared IdP and maps its groups/domains to tenants." Org/domain auto-map and
//! first-login-approval are deferred variants (OQ-1).

use crate::identity::ResolvedPrincipal;

/// Relying-party configuration for a single OIDC issuer (OQ-4: one issuer for
/// now). Read from the environment; absent → OIDC login is simply not mounted.
/// The discovery + JWKS + login/callback that consume this land in T-0789/0790
/// (built on the `openidconnect` crate, OQ-6) against the Dex sidecar.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl OidcConfig {
    /// Build from env (`CLOACINA_OIDC_*`); `None` when not configured.
    pub fn from_env() -> Option<OidcConfig> {
        let issuer_url = std::env::var("CLOACINA_OIDC_ISSUER").ok()?;
        let client_id = std::env::var("CLOACINA_OIDC_CLIENT_ID").ok()?;
        let redirect_uri = std::env::var("CLOACINA_OIDC_REDIRECT_URI").ok()?;
        Some(OidcConfig {
            issuer_url,
            client_id,
            client_secret: std::env::var("CLOACINA_OIDC_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri,
            scopes: parse_scopes(std::env::var("CLOACINA_OIDC_SCOPES").ok().as_deref()),
        })
    }
}

/// Parse a comma-separated scope list, defaulting to the standard OIDC set
/// (+ `groups` for the mapping policy) when unset/empty.
fn parse_scopes(raw: Option<&str>) -> Vec<String> {
    let parsed: Vec<String> = raw
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parsed.is_empty() {
        vec![
            "openid".into(),
            "email".into(),
            "profile".into(),
            "groups".into(),
        ]
    } else {
        parsed
    }
}

/// A validated set of identity claims extracted from an OIDC ID token by the
/// relying party (T-0790). Provider-neutral within OIDC.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityClaims {
    /// The stable subject identifier (`sub`).
    pub subject: String,
    /// The verified email, if the IdP released one.
    pub email: Option<String>,
    /// Group/role memberships from the IdP (e.g. Keycloak `groups`, GitHub orgs).
    pub groups: Vec<String>,
}

/// What an allowlist rule matches an identity on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimMatch {
    /// The identity is a member of this group.
    Group(String),
    /// The identity's email is in this domain (e.g. `acme.com`).
    EmailDomain(String),
    /// An exact subject match (a specific person).
    Subject(String),
}

/// One allowlist rule: a claim match grants a `{tenant, role}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingRule {
    pub claim: ClaimMatch,
    /// Tenant the principal is scoped to (`None` = global/public).
    pub tenant: Option<String>,
    /// Role within that tenant (`read` | `write` | `admin`).
    pub role: String,
}

impl MappingRule {
    fn matches(&self, claims: &IdentityClaims) -> bool {
        match &self.claim {
            ClaimMatch::Subject(s) => &claims.subject == s,
            ClaimMatch::Group(g) => claims.groups.iter().any(|cg| cg == g),
            ClaimMatch::EmailDomain(d) => claims
                .email
                .as_deref()
                .map(|e| email_in_domain(e, d))
                .unwrap_or(false),
        }
    }
}

/// True when `email`'s domain part equals `domain` (case-insensitive).
fn email_in_domain(email: &str, domain: &str) -> bool {
    email
        .rsplit_once('@')
        .map(|(_, d)| d.eq_ignore_ascii_case(domain))
        .unwrap_or(false)
}

/// The ordered allowlist. **First matching rule wins.** An unmatched identity
/// resolves to `None` (denied).
#[derive(Debug, Clone, Default)]
pub struct MappingPolicy {
    rules: Vec<MappingRule>,
}

impl MappingPolicy {
    pub fn new(rules: Vec<MappingRule>) -> Self {
        Self { rules }
    }

    /// Resolve an identity to a principal, or `None` if no rule matches.
    /// `issuer` is recorded in the provenance (`oidc:<issuer>:<sub>`).
    pub fn resolve(&self, claims: &IdentityClaims, issuer: &str) -> Option<ResolvedPrincipal> {
        self.rules.iter().find(|r| r.matches(claims)).map(|r| ResolvedPrincipal {
            tenant: r.tenant.clone(),
            role: r.role.clone(),
            provenance: format!("oidc:{issuer}:{}", claims.subject),
        })
    }

    /// Resolve an identity to **all** the tenant memberships it matches — one
    /// principal per tenant (first rule per tenant wins; `_`/global counts as a
    /// tenant). Empty = denied. This is what an OIDC login uses: a single
    /// sign-in can grant access to several tenants, each minted its own scoped
    /// key, and the UI lets the user pick which to enter.
    pub fn resolve_all(&self, claims: &IdentityClaims, issuer: &str) -> Vec<ResolvedPrincipal> {
        let mut seen: std::collections::HashSet<Option<String>> = std::collections::HashSet::new();
        let mut out = Vec::new();
        for r in self.rules.iter().filter(|r| r.matches(claims)) {
            if seen.insert(r.tenant.clone()) {
                out.push(ResolvedPrincipal {
                    tenant: r.tenant.clone(),
                    role: r.role.clone(),
                    provenance: format!("oidc:{issuer}:{}", claims.subject),
                });
            }
        }
        out
    }

    /// Build the allowlist from `CLOACINA_OIDC_MAP` (god-owned config, OQ-11).
    pub fn from_env() -> MappingPolicy {
        MappingPolicy::parse(&std::env::var("CLOACINA_OIDC_MAP").unwrap_or_default())
    }

    /// Parse a `;`-separated allowlist: each rule `<match>=<tenant>:<role>`,
    /// where `<match>` is `group:NAME` / `domain:NAME` / `sub:NAME` and
    /// `<tenant>` may be `_` for a global principal. Malformed clauses are
    /// skipped. Example: `group:acme-admins=acme:admin;domain:acme.com=acme:write`.
    pub fn parse(raw: &str) -> MappingPolicy {
        let mut rules = Vec::new();
        for clause in raw.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            let Some((matcher, grant)) = clause.split_once('=') else {
                continue;
            };
            let claim = match matcher.trim().split_once(':') {
                Some(("group", v)) => ClaimMatch::Group(v.trim().to_string()),
                Some(("domain", v)) => ClaimMatch::EmailDomain(v.trim().to_string()),
                Some(("sub", v)) => ClaimMatch::Subject(v.trim().to_string()),
                _ => continue,
            };
            let Some((tenant, role)) = grant.trim().split_once(':') else {
                continue;
            };
            let tenant = match tenant.trim() {
                "_" | "" => None,
                t => Some(t.to_string()),
            };
            rules.push(MappingRule {
                claim,
                tenant,
                role: role.trim().to_string(),
            });
        }
        MappingPolicy::new(rules)
    }
}

// ---------------------------------------------------------------------------
// Relying party — discovery + the authorization-code/PKCE flow (CLOACI-T-0789/
// 0790), built on the `openidconnect` crate against the configured issuer.
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};

/// A discovered OIDC relying party. Holds the cached provider metadata (which
/// carries the JWKS + endpoints) + config + an HTTP client. The `openidconnect`
/// `CoreClient` is type-state generic, so it is built **inline** per request
/// from `metadata` rather than stored here.
pub struct OidcProvider {
    pub metadata: CoreProviderMetadata,
    pub config: OidcConfig,
    pub http: reqwest::Client,
}

impl OidcProvider {
    /// Discover the issuer's metadata + JWKS and build a relying party. Called
    /// once at startup when `OidcConfig::from_env()` is `Some`.
    pub async fn discover(config: OidcConfig) -> Result<Arc<OidcProvider>, String> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| format!("oidc http client: {e}"))?;
        let issuer =
            IssuerUrl::new(config.issuer_url.clone()).map_err(|e| format!("oidc issuer url: {e}"))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, &http)
            .await
            .map_err(|e| format!("oidc discovery failed: {e}"))?;
        Ok(Arc::new(OidcProvider {
            metadata,
            config,
            http,
        }))
    }

    /// Begin a login: build the authorization-code + PKCE URL with fresh state
    /// + nonce. The caller stashes `(state, nonce, pkce_verifier)` in the
    /// [`LoginFlowStore`] and redirects the browser to `auth_url`.
    pub fn begin_login(&self) -> Result<LoginStart, String> {
        let client = self.client()?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut req = client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        );
        for s in &self.config.scopes {
            if s != "openid" {
                req = req.add_scope(Scope::new(s.clone()));
            }
        }
        let (auth_url, csrf, nonce) = req.set_pkce_challenge(pkce_challenge).url();
        Ok(LoginStart {
            auth_url: auth_url.to_string(),
            state: csrf.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: pkce_verifier.into_secret(),
        })
    }

    /// Complete a login: exchange the code (with the stashed PKCE verifier),
    /// validate the ID token (signature via JWKS, `iss`/`aud`/`exp`, nonce),
    /// and extract the identity claims. The caller maps those to a principal.
    pub async fn complete_login(
        &self,
        code: String,
        nonce: String,
        pkce_verifier: String,
    ) -> Result<IdentityClaims, String> {
        let client = self.client()?;
        let token_response = client
            .exchange_code(AuthorizationCode::new(code))
            .map_err(|e| format!("exchange_code: {e}"))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier))
            .request_async(&self.http)
            .await
            .map_err(|e| format!("token exchange failed: {e}"))?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| "no id_token in token response".to_string())?;
        let verifier = client.id_token_verifier();
        let nonce = Nonce::new(nonce);
        let claims = id_token
            .claims(&verifier, &nonce)
            .map_err(|e| format!("id_token validation failed: {e}"))?;

        let subject = claims.subject().as_str().to_string();
        let email = claims.email().map(|e| e.as_str().to_string());
        // `groups` is a non-standard claim Dex/Keycloak include; read it from
        // the *already-validated* JWT payload (signature/iss/aud/exp/nonce all
        // checked above) rather than fighting the typed additional-claims path.
        let groups = extract_groups(&id_token.to_string());

        Ok(IdentityClaims {
            subject,
            email,
            groups,
        })
    }

    /// Build the `openidconnect` client from cached metadata. Inlined here
    /// because the v4 `CoreClient` is type-state generic and can't be named in
    /// a stored field or returned without leaking the full type — both callers
    /// use it locally, so the type stays inferred.
    fn client(
        &self,
    ) -> Result<
        openidconnect::core::CoreClient<
            openidconnect::EndpointSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointNotSet,
            openidconnect::EndpointMaybeSet,
            openidconnect::EndpointMaybeSet,
        >,
        String,
    > {
        Ok(CoreClient::from_provider_metadata(
            self.metadata.clone(),
            ClientId::new(self.config.client_id.clone()),
            Some(ClientSecret::new(self.config.client_secret.clone())),
        )
        .set_redirect_uri(
            RedirectUrl::new(self.config.redirect_uri.clone()).map_err(|e| e.to_string())?,
        ))
    }
}

/// The output of [`OidcProvider::begin_login`].
pub struct LoginStart {
    pub auth_url: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

struct LoginFlowEntry {
    nonce: String,
    pkce_verifier: String,
    expires_at: Instant,
}

/// Short-lived store for in-flight login state (`state -> nonce + PKCE
/// verifier`), single-use. In-memory for the single-replica demo; NFR-003
/// (multi-replica) wants this Postgres-backed like the ws-ticket/oidc_sessions
/// infra — a documented follow-up.
pub struct LoginFlowStore {
    inner: tokio::sync::Mutex<HashMap<String, LoginFlowEntry>>,
    ttl: Duration,
}

impl LoginFlowStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: tokio::sync::Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Stash a flow keyed by `state`, evicting anything expired.
    pub async fn put(&self, state: String, nonce: String, pkce_verifier: String) {
        let mut g = self.inner.lock().await;
        let now = Instant::now();
        g.retain(|_, e| e.expires_at > now);
        g.insert(
            state,
            LoginFlowEntry {
                nonce,
                pkce_verifier,
                expires_at: now + self.ttl,
            },
        );
    }

    /// Consume the flow for `state` (single-use). `None` if unknown/expired.
    pub async fn take(&self, state: &str) -> Option<(String, String)> {
        let mut g = self.inner.lock().await;
        g.remove(state)
            .filter(|e| e.expires_at > Instant::now())
            .map(|e| (e.nonce, e.pkce_verifier))
    }
}

/// Extract the `groups` claim from a (already-validated) JWT's payload.
fn extract_groups(jwt: &str) -> Vec<String> {
    use base64::Engine;
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() < 2 {
        return Vec::new();
    }
    let Ok(bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1]) else {
        return Vec::new();
    };
    #[derive(serde::Deserialize)]
    struct Payload {
        #[serde(default)]
        groups: Vec<String>,
    }
    serde_json::from_slice::<Payload>(&bytes)
        .map(|p| p.groups)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(sub: &str, email: Option<&str>, groups: &[&str]) -> IdentityClaims {
        IdentityClaims {
            subject: sub.to_string(),
            email: email.map(str::to_string),
            groups: groups.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn policy() -> MappingPolicy {
        MappingPolicy::new(vec![
            MappingRule {
                claim: ClaimMatch::Group("acme-admins".into()),
                tenant: Some("acme".into()),
                role: "admin".into(),
            },
            MappingRule {
                claim: ClaimMatch::EmailDomain("acme.com".into()),
                tenant: Some("acme".into()),
                role: "write".into(),
            },
            MappingRule {
                claim: ClaimMatch::Subject("sub-root".into()),
                tenant: None,
                role: "admin".into(),
            },
        ])
    }

    #[test]
    fn group_match_resolves_tenant_and_role() {
        let p = policy()
            .resolve(&claims("u1", Some("u1@x.com"), &["acme-admins"]), "https://idp")
            .expect("mapped");
        assert_eq!(p.tenant.as_deref(), Some("acme"));
        assert_eq!(p.role, "admin");
        assert_eq!(p.provenance, "oidc:https://idp:u1");
    }

    #[test]
    fn email_domain_match() {
        let p = policy()
            .resolve(&claims("u2", Some("bob@ACME.com"), &[]), "iss")
            .expect("mapped");
        assert_eq!(p.tenant.as_deref(), Some("acme"));
        assert_eq!(p.role, "write");
    }

    #[test]
    fn subject_match_global() {
        let p = policy()
            .resolve(&claims("sub-root", None, &[]), "iss")
            .expect("mapped");
        assert!(p.tenant.is_none());
        assert_eq!(p.role, "admin");
    }

    #[test]
    fn first_rule_wins() {
        // Both group (admin) and domain (write) match; the group rule is first.
        let p = policy()
            .resolve(&claims("u3", Some("c@acme.com"), &["acme-admins"]), "iss")
            .unwrap();
        assert_eq!(p.role, "admin");
    }

    #[test]
    fn unmapped_identity_denied() {
        assert!(policy()
            .resolve(&claims("nobody", Some("x@other.org"), &["randos"]), "iss")
            .is_none());
    }

    #[test]
    fn scopes_default_and_parse() {
        assert!(parse_scopes(None).contains(&"openid".to_string()));
        assert!(parse_scopes(Some("  ")).contains(&"groups".to_string()));
        assert_eq!(parse_scopes(Some("openid, email")), vec!["openid", "email"]);
    }

    #[test]
    fn parse_allowlist_from_string() {
        let p = MappingPolicy::parse(
            "group:acme-admins=acme:admin; domain:acme.com=acme:write; sub:root=_:admin; garbage",
        );
        let admin = p
            .resolve(&claims("x", None, &["acme-admins"]), "i")
            .unwrap();
        assert_eq!(admin.tenant.as_deref(), Some("acme"));
        assert_eq!(admin.role, "admin");
        assert_eq!(
            p.resolve(&claims("y", Some("y@acme.com"), &[]), "i").unwrap().role,
            "write"
        );
        assert!(p.resolve(&claims("root", None, &[]), "i").unwrap().tenant.is_none());
        assert!(p.resolve(&claims("z", None, &[]), "i").is_none());
    }

    #[test]
    fn resolve_all_one_membership_per_tenant() {
        let p = MappingPolicy::parse(
            "domain:acme.com=acme:admin; domain:acme.com=public:read; group:x=acme:read",
        );
        let ms = p.resolve_all(&claims("u", Some("u@acme.com"), &["x"]), "i");
        // acme (first rule wins → admin; the later acme rule is deduped) + public.
        assert_eq!(ms.len(), 2);
        let acme = ms.iter().find(|m| m.tenant.as_deref() == Some("acme")).unwrap();
        assert_eq!(acme.role, "admin");
        assert!(ms
            .iter()
            .any(|m| m.tenant.as_deref() == Some("public") && m.role == "read"));
        assert!(p
            .resolve_all(&claims("z", Some("z@nope.org"), &[]), "i")
            .is_empty());
    }

    /// Live discovery against the Dex sidecar. Ignored by default — run with
    /// `docker compose -f docker/docker-compose.demo.yml up -d dex` then
    /// `cargo test -p cloacina-server --lib oidc -- --ignored`.
    #[tokio::test]
    #[ignore = "requires a live issuer (Dex)"]
    async fn discovers_against_live_issuer() {
        let issuer = std::env::var("CLOACINA_OIDC_ISSUER")
            .unwrap_or_else(|_| "http://localhost:5556/dex".to_string());
        let cfg = OidcConfig {
            issuer_url: issuer,
            client_id: "cloacina".to_string(),
            client_secret: "cloacina-dex-secret".to_string(),
            redirect_uri: "http://localhost:8080/v1/auth/callback".to_string(),
            scopes: parse_scopes(None),
        };
        let provider = OidcProvider::discover(cfg)
            .await
            .expect("discovery should succeed against Dex");
        assert!(
            provider.metadata.token_endpoint().is_some(),
            "discovered metadata must carry a token endpoint"
        );
    }

    /// Live: discover against Dex then build the real authorization URL, and
    /// assert it carries PKCE + state + nonce + the requested scopes. Ignored
    /// by default (needs Dex). Run: `... oidc -- --ignored`.
    #[tokio::test]
    #[ignore = "requires a live issuer (Dex)"]
    async fn begins_login_against_live_issuer() {
        let cfg = OidcConfig {
            issuer_url: "http://localhost:5556/dex".to_string(),
            client_id: "cloacina".to_string(),
            client_secret: "cloacina-dex-secret".to_string(),
            redirect_uri: "http://localhost:8080/v1/auth/callback".to_string(),
            scopes: parse_scopes(None),
        };
        let provider = OidcProvider::discover(cfg).await.expect("discover");
        let start = provider.begin_login().expect("begin_login");
        assert!(
            start.auth_url.starts_with("http://localhost:5556/dex/auth"),
            "auth_url should point at Dex's authorize endpoint: {}",
            start.auth_url
        );
        assert!(start.auth_url.contains("code_challenge="));
        assert!(start.auth_url.contains("code_challenge_method=S256"));
        assert!(start.auth_url.contains("state="));
        assert!(start.auth_url.contains("nonce="));
        assert!(start.auth_url.contains("groups"));
        assert!(!start.state.is_empty() && !start.pkce_verifier.is_empty());
    }
}
