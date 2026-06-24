---
id: oidc-rp-library-spike-decision-oq
level: task
title: "OIDC RP library spike + decision (OQ-6) — openidconnect crate vs hand-rolled oauth2+JWKS"
short_code: "CLOACI-T-0788"
created_at: 2026-06-24T01:06:59.152198+00:00
updated_at: 2026-06-24T01:06:59.152198+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# OIDC RP library spike + decision (OQ-6) — openidconnect crate vs hand-rolled oauth2+JWKS

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Time-boxed spike to choose the OIDC relying-party implementation for cloacina-server. Build a throwaway authorization-code + PKCE login against a single dockerized test issuer (Dex or Keycloak, added to the test compose) using (a) the `openidconnect` crate and (b) hand-rolled `oauth2` + JWKS, and pick one on ergonomics, spec-correctness, and maintenance cost. The decision + rationale is the deliverable (it feeds the Phase 5 ADR); the prototype code is disposable.

## Acceptance Criteria **[REQUIRED]**

- [ ] A working throwaway OIDC login completes against the dockerized test issuer.
- [ ] A documented decision (`openidconnect` crate vs hand-rolled `oauth2`+JWKS) with rationale.
- [ ] The chosen approach is recorded back in CLOACI-I-0118 (resolves OQ-6).

## Implementation Notes

**Scope:** spike only — throwaway code; the decision is the artifact. Single generic OIDC issuer (no provider/vendor commitment).
**Depends on:** none (can run in parallel with Phase 0).
**References:** CLOACI-I-0118 → Open Questions OQ-6; Implementation Plan Phase 1.

## Status Updates **[REQUIRED]**

*To be added during implementation*
