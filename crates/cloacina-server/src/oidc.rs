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
}
