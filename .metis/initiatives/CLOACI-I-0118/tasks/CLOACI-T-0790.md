---
id: oidc-login-callback-authorization
level: task
title: "OIDC login/callback — authorization-code + PKCE + spec-correct token validation → validated identity"
short_code: "CLOACI-T-0790"
created_at: 2026-06-24T01:08:01.506735+00:00
updated_at: 2026-06-24T01:08:01.506735+00:00
parent: CLOACI-I-0118
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0118
---

# OIDC login/callback — authorization-code + PKCE + spec-correct token validation → validated identity

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0118]]

## Objective **[REQUIRED]**

Implement `/auth/login` (initiate: set state + nonce + PKCE) and `/auth/callback` (validate + exchange code) for the authorization-code flow. Token validation must be spec-correct: JWKS signature, `iss`/`aud`/`exp`/`iat`, and nonce; strict state + PKCE checks. Resolve a validated identity claim set (`sub`, `email`/domain, `groups`/`org`) — NO key minting yet. Login-flow state (state/nonce/PKCE verifier) lives in shared Postgres and is swept like the ws-ticket/outbox infra so it is multi-replica safe.

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

*To be added during implementation*
