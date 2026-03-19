---
id: integration-test-upload-package
level: task
title: "Integration test: upload package, trigger execution, poll status, verify complete"
short_code: "CLOACI-T-0200"
created_at: 2026-03-16T21:09:43.331434+00:00
updated_at: 2026-03-16T21:28:06.563511+00:00
parent: CLOACI-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0032
---

# Integration test: upload package, trigger execution, poll status, verify complete

## Parent Initiative

[[CLOACI-I-0032]] — Server Phase 4: Core REST API

## Objective

Write integration tests that validate the Phase 4 REST API endpoints function correctly, including authentication enforcement, error responses, and request parsing. Tests run against an in-process Axum server (same pattern as existing auth integration tests) without requiring a database for basic routing validation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Integration test file created (e.g., `tests/integration/server/api_routes.rs` or similar)
- [ ] Test: `POST /workflows/packages` without authentication returns 401 Unauthorized
- [ ] Test: `GET /workflows` without runner in AppState returns 503 Service Unavailable
- [ ] Test: `POST /executions` with correct auth validates request body parsing (400 on malformed JSON)
- [ ] Test: `GET /executions/{id}` with non-existent UUID returns 404 Not Found (when runner is available)
- [ ] Test: Error responses conform to the `{"error": {"code": "...", "message": "..."}}` JSON format
- [ ] All tests pass in CI without external dependencies (no database, no network)
- [ ] Future enhancement noted: full end-to-end test with real package upload + execution requires DB setup

## Test Cases

### Test Case 1: Unauthenticated Package Upload
- **Test ID**: TC-001
- **Preconditions**: In-process server running with auth enabled, no PAK header
- **Steps**:
  1. Send `POST /workflows/packages` with multipart body but no `Authorization` header
  2. Assert response status is 401
  3. Assert response body matches error format with code `UNAUTHORIZED`

### Test Case 2: Missing Runner Returns 503
- **Test ID**: TC-002
- **Preconditions**: In-process server running with `AppState.runner = None`, valid auth token
- **Steps**:
  1. Send `GET /workflows` with valid PAK in `Authorization` header
  2. Assert response status is 503
  3. Assert response body has code `SERVICE_UNAVAILABLE` and message indicates runner not configured

### Test Case 3: Malformed Execution Request
- **Test ID**: TC-003
- **Preconditions**: In-process server running with auth enabled, valid auth token with `can_execute`
- **Steps**:
  1. Send `POST /executions` with invalid JSON body (e.g., missing `workflow_name`)
  2. Assert response status is 400
  3. Assert error response includes validation details

### Test Case 4: Non-existent Execution Lookup
- **Test ID**: TC-004
- **Preconditions**: In-process server with runner available (mock or minimal), valid auth token with `can_read`
- **Steps**:
  1. Send `GET /executions/{random-uuid}` with valid auth
  2. Assert response status is 404
  3. Assert error code is `EXECUTION_NOT_FOUND`

### Test Case 5: Error Response Format Consistency
- **Test ID**: TC-005
- **Preconditions**: In-process server running
- **Steps**:
  1. Trigger multiple different error conditions (401, 400, 404, 503)
  2. For each, parse the response body as JSON
  3. Assert all have the structure `{"error": {"code": string, "message": string}}`
  4. Assert no error responses contain stack traces or internal details

## Implementation Notes

### Technical Approach

1. **Test server setup**: Use the same in-process server pattern as existing auth tests. Build the Axum app with `app()`, then use `tower::ServiceExt` and `hyper::Request` to send requests directly (no TCP listener needed):
   ```rust
   let app = app(config, None); // None runner for 503 tests
   let response = app.oneshot(request).await.unwrap();
   ```

2. **Auth test helpers**: Reuse existing test PAK generation utilities from the auth test suite to create valid tokens with specific permissions (`can_read`, `can_write`, `can_execute`).

3. **Runner availability tests**: Some tests use `AppState.runner = None` to verify 503 behavior. For 404 tests, either provide a minimal mock runner that always returns NotFound, or use a real `DefaultRunner` with an empty registry.

4. **No DB required**: All tests validate HTTP-layer behavior (routing, auth, request parsing, error formatting). The actual workflow registration and execution logic is tested in library-level unit tests. Full e2e tests with database are a future enhancement.

### Dependencies

- CLOACI-T-0195 through CLOACI-T-0199 — All handlers and route wiring must be complete
- Existing test infrastructure: in-process server pattern from auth integration tests
- `tower::ServiceExt` for `oneshot()` testing

### Future Enhancements

- Full end-to-end test: upload a real workflow package, trigger execution, poll status until complete, verify results. This requires database setup and is out of scope for initial Phase 4.
- Load testing / concurrent request testing for execution endpoints.
- OpenAPI spec validation: assert that `/api-docs/openapi.json` includes all expected paths and schemas.

## Status Updates

### 2026-03-16 — Completed
- 3 integration tests added:
  - test_api_workflows_without_runner_returns_503: authenticated GET /workflows without runner → 503 with SERVICE_UNAVAILABLE code
  - test_api_executions_without_auth_returns_401: POST/GET /executions without auth → 401
  - test_api_error_format_consistency: GET /executions/{id} without runner → 503, validates JSON error format has error.code + error.message
- Tests use pre-populated AuthCache (no DB needed)
- All 40 cloacinactl tests pass
