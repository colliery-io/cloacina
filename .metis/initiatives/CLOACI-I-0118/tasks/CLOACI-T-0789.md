---
id: oidc-rp-config-discovery-jwks
level: task
title: "OIDC RP config + discovery + JWKS (single issuer)"
short_code: "CLOACI-T-0789"
created_at: 2026-06-24T01:07:32.683057+00:00
updated_at: 2026-06-24T01:07:32.683057+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# OIDC RP config + discovery + JWKS (single issuer)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement a configurable OIDC relying party for a SINGLE issuer — issuer URL, client id/secret, scopes, redirect URI via server config/env — with `.well-known/openid-configuration` discovery and JWKS fetch + caching. Multi-issuer support (OQ-4) is explicitly deferred: exactly one configured issuer for now.

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

*To be added during implementation*
