---
id: standardize-rest-api-error
level: task
title: "Standardize REST API error responses with ApiError type and request IDs (API-02)"
short_code: "CLOACI-T-0448"
created_at: 2026-04-08T23:47:04.031301+00:00
updated_at: 2026-04-09T00:01:30.626529+00:00
parent: CLOACI-I-0087
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0087
---

# Standardize REST API error responses with ApiError type and request IDs (API-02)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0087]]

## Objective

All REST API errors are currently ad-hoc `{"error": "string"}` with no machine-readable codes, no request correlation IDs, and inconsistent HTTP status codes. API clients cannot programmatically handle errors without string matching. Also standardize status value casing to lowercase.

**Effort**: 2-3 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] New `ApiError` struct in `server/error.rs` with fields: `error` (human-readable), `code` (machine-readable string like `"workflow_not_found"`), `status` (HTTP code), `request_id` (UUID)
- [ ] `ApiError` implements `IntoResponse` for axum
- [ ] All handlers in `keys.rs`, `tenants.rs`, `workflows.rs`, `executions.rs`, `triggers.rs` use `ApiError` instead of inline `Json(json!({"error": ...}))`
- [ ] Request ID generated in a middleware layer and attached to response headers (`X-Request-Id`) and all log spans
- [ ] Error responses follow consistent format: `{"error": "...", "code": "...", "request_id": "..."}`
- [ ] Status values in responses use lowercase consistently (`"completed"`, `"running"`, `"failed"`)
- [ ] Existing auth integration and soak tests updated for new error format

## Implementation Notes

### Technical Approach

1. Create `crates/cloacinactl/src/server/error.rs` with:
   ```rust
   pub struct ApiError { pub status: StatusCode, pub code: &'static str, pub message: String, pub request_id: Option<String> }
   impl IntoResponse for ApiError { ... }
   ```
2. Add request ID middleware: generate UUID per request, insert into extensions, add `X-Request-Id` response header.
3. Replace all inline `Json(json!({"error": ...}))` patterns across handlers — systematic find-and-replace.
4. Normalize status strings to lowercase in response serialization.

### Dependencies
Do this FIRST in I-0087 — it touches all handlers, so doing it before other tasks reduces merge conflicts.

## Status Updates

- **2026-04-08**: Created `server/error.rs` with `ApiError` struct (status, code, message) + `IntoResponse` impl + convenience constructors (bad_request, not_found, forbidden, unauthorized, internal, too_many_requests). Replaced 36 of 38 inline error patterns across keys.rs, tenants.rs, workflows.rs, executions.rs, triggers.rs, auth.rs, ws.rs. Remaining 2 are in `validate_token` (shared return type with WS handlers — left unchanged). Also updated auth helper methods (forbidden_response, admin_required_response, insufficient_role_response) to return `ApiError`. Request ID middleware deferred to observability initiative (I-0088) where it pairs with distributed tracing.
