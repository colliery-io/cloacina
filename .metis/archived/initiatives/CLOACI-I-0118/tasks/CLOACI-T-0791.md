---
id: identity-tenant-role-mapping
level: task
title: "Identity→tenant/role mapping policy (allowlist default, god-owned)"
short_code: "CLOACI-T-0791"
created_at: 2026-06-24T01:08:30.974333+00:00
updated_at: 2026-06-24T04:15:50.697779+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# Identity→tenant/role mapping policy (allowlist default, god-owned)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Resolve a validated OIDC identity to ABAC `Principal` attributes (tenant + role) via a god-owned, config-driven ALLOWLIST policy: explicit `claim(group | email-domain | sub) → {tenant, role}` rules. An unmapped identity is denied. The default policy is allowlist; org/domain auto-map and first-login-approval are deferred variants (note them, do not build). Uses the same attribute vocabulary as the Phase 0 ABAC matcher.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria **[REQUIRED]**

- [ ] A validated identity resolves to a `{tenant, role}` pair or is denied with a typed error when unmapped.
- [ ] Mapping rules are god-configured (single shared IdP assumption, OQ-11 god-owned).
- [ ] Tests cover mapped → correct tenant/role and unmapped → denial.

## Implementation Notes

**Scope:** the mapping policy only (allowlist). Resolves OQ-1 to the allowlist default; OQ-11 god-owned.
**Depends on:** Task 3 (validated identity); soft-depends on Phase 0 matcher (CLOACI-T-0782) for the shared Principal vocabulary — can be developed in parallel.
**References:** CLOACI-I-0118 REQ-005; OQ-1 (defaulted to allowlist), OQ-11 (god-owned).

## Status Updates **[REQUIRED]**

**2026-06-24 — COMPLETE (policy logic; producer pending).** New `crates/cloacina-server/src/oidc.rs`: `IdentityClaims { subject, email, groups }`, `ClaimMatch::{Group, EmailDomain, Subject}`, `MappingRule { claim, tenant, role }`, `MappingPolicy::resolve(claims, issuer) -> Option<ResolvedPrincipal>` — **god-owned config-driven allowlist** (OQ-1 default, OQ-11 god-owned), **first match wins**, unmatched → `None` (denied), provenance `oidc:<issuer>:<sub>`. Produces the same provider-agnostic `ResolvedPrincipal` local login does → feeds `identity::mint_for_principal`. `angreal check` clean; **5/5 unit tests**. **Pending (the producer):** the OIDC RP that extracts these claims from a validated ID token — config/discovery/JWKS (T-0789) + login/callback (T-0790) via `openidconnect` (OQ-6) — plus a Dex sidecar for live verification. **Depends on:** T-0782 (done).