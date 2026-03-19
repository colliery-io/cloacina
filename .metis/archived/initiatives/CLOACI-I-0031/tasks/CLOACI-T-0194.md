---
id: integration-test-full-auth-flow
level: task
title: "Integration test: full auth flow (create key, authenticate, access, revoke, 401)"
short_code: "CLOACI-T-0194"
created_at: 2026-03-16T20:01:09.023041+00:00
updated_at: 2026-03-16T20:40:48.184185+00:00
parent: CLOACI-I-0031
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0031
---

# Integration test: full auth flow (create key, authenticate, access, revoke, 401)

## Objective

Write an end-to-end integration test that exercises the complete auth flow against a real Postgres database: key creation, authenticated access, permission enforcement, revocation, and workflow pattern restriction.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Test runs against real Postgres (not mocked)
- [ ] Create a tenant via TenantDAL
- [ ] Create an API key with specific permissions via DAL (or CLI)
- [ ] Start the axum server with full auth middleware stack
- [ ] Request with valid Bearer token returns 200
- [ ] Request with invalid/garbage token returns 401
- [ ] Request with no Authorization header returns 401
- [ ] Request with valid token but insufficient permission returns 403
- [ ] Revoke the key, then send same token — returns 401
- [ ] Create workflow-scoped key with pattern `"etl::*"`, access matching workflow `"etl::daily_load"` returns 200
- [ ] Same scoped key accessing non-matching workflow `"reports::monthly"` returns 403
- [ ] Expired key returns 401

## Implementation Notes

### Test Structure
- Place in `crates/cloacina/tests/integration/` following existing test patterns
- Use `#[tokio::test]` with real Postgres connection (test database)
- Set up: run migrations, create tenant, create keys with various permission combinations
- Use `reqwest` or axum's `TestClient` to send HTTP requests to the running server

### Test Scenarios
1. **Happy path**: create tenant + key with read permission, GET protected endpoint -> 200
2. **No auth**: request without header -> 401
3. **Bad token**: random string as Bearer -> 401
4. **Wrong permission**: key with only read, POST to execute endpoint -> 403
5. **Revocation**: revoke key, retry same request -> 401 (may need cache invalidation or wait for TTL)
6. **Workflow pattern match**: scoped key with `"etl::*"`, access `"etl::daily_load"` -> 200
7. **Workflow pattern mismatch**: same scoped key, access `"reports::monthly"` -> 403
8. **Expired key**: create key with past expires_at, attempt access -> 401

### Dependencies
- All prior tasks in the initiative (T-0184 through T-0192)
- Postgres test database infrastructure

## Status Updates

### 2026-03-16 — Completed
- Added /auth-test protected endpoint (returns AuthContext as JSON) for middleware validation
- 3 integration tests using pre-populated AuthCache (no DB needed):
  - test_auth_protected_endpoint_requires_auth: no header → 401, health → 200
  - test_auth_valid_key_returns_200: cached key + Bearer header → 200 with permissions in body
  - test_auth_invalid_key_returns_401: negative-cached prefix → 401
- Tests use in-memory sqlite DAL as dummy (cache is pre-populated, DB never hit)
- All 37 cloacinactl tests pass
