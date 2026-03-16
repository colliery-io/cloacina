---
id: wire-all-api-routes-into-protected
level: task
title: "Wire all API routes into protected Router with permission guards and utoipa annotations"
short_code: "CLOACI-T-0199"
created_at: 2026-03-16T21:09:42.231294+00:00
updated_at: 2026-03-16T21:25:25.052295+00:00
parent: CLOACI-I-0032
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0032
---

# Wire all API routes into protected Router with permission guards and utoipa annotations

## Parent Initiative

[[CLOACI-I-0032]] — Server Phase 4: Core REST API

## Objective

Wire all Phase 4 API handlers into the Axum router with appropriate permission guards, and add utoipa OpenAPI annotations to all handlers and response types. This is the integration task that makes all endpoints from CLOACI-T-0196, T-0197, and T-0198 accessible and documented.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `serve.rs` `app()` function registers all routes under `protected_routes`:
  - `POST /workflows/packages` with `require_write` guard
  - `GET /workflows` with `require_read` guard
  - `POST /executions` with `require_execute` guard
  - `GET /executions` with `require_read` guard
  - `GET /executions/:id` with `require_read` guard
  - `POST /executions/:id/pause` with `require_execute` guard
  - `POST /executions/:id/resume` with `require_execute` guard
  - `DELETE /executions/:id` with `require_execute` guard
- [ ] All handlers have `#[utoipa::path(...)]` annotations with:
  - Correct HTTP method and path
  - Request body schema (where applicable)
  - Response status codes and body schemas (200/201/202, 400, 401, 403, 404, 503)
  - Security requirement (`bearer_auth`)
  - Tag grouping (`workflows` or `executions`)
- [ ] All request/response structs added to OpenAPI schemas via `#[derive(ToSchema)]`
- [ ] `ApiDoc` struct updated with all new paths in the `#[openapi(paths(...))]` attribute
- [ ] `ApiDoc` struct updated with all new schemas in the `#[openapi(schemas(...))]` attribute
- [ ] `ApiError` / `ErrorEnvelope` added to OpenAPI schemas
- [ ] `/swagger-ui` and `/api-docs/openapi.json` reflect all new endpoints
- [ ] All routes are protected (no unauthenticated access to API endpoints)

## Implementation Notes

### Technical Approach

1. **Route registration in `serve.rs`**: Add routes to the existing `protected_routes` Router that already has auth middleware applied:
   ```rust
   let protected_routes = Router::new()
       // ... existing routes ...
       .route("/workflows/packages", post(workflows::upload_package))
       .route("/workflows", get(workflows::list_workflows))
       .route("/executions", post(executions::create_execution).get(executions::list_executions))
       .route("/executions/:id", get(executions::get_execution).delete(executions::cancel_execution))
       .route("/executions/:id/pause", post(executions::pause_execution))
       .route("/executions/:id/resume", post(executions::resume_execution))
       .layer(auth_layer);
   ```

2. **Permission guards**: The existing ABAC middleware from Phase 3 (CLOACI-I-0031) provides `require_read`, `require_write`, and `require_execute` layer/middleware functions. Apply the appropriate guard per route. For `POST /executions`, the execute guard also checks the workflow pattern from the PAK's allowed patterns.

3. **utoipa annotations**: Add to each handler function, e.g.:
   ```rust
   #[utoipa::path(
       post,
       path = "/workflows/packages",
       request_body(content_type = "multipart/form-data"),
       responses(
           (status = 201, description = "Package registered", body = Value),
           (status = 400, description = "Invalid package", body = ErrorEnvelope),
           (status = 503, description = "Runner not available", body = ErrorEnvelope),
       ),
       security(("bearer_auth" = [])),
       tag = "workflows"
   )]
   ```

4. **ApiDoc update**: Add all handler paths and schema types to the derive macro on `ApiDoc`.

### Dependencies

- CLOACI-T-0195 — `ApiError`, `ErrorEnvelope`, module scaffolding
- CLOACI-T-0196 — Workflow handlers (must be implemented)
- CLOACI-T-0197 — Execution CRUD handlers (must be implemented)
- CLOACI-T-0198 — Execution control handlers (must be implemented)
- CLOACI-I-0031 (Auth) — Permission guard middleware (`require_read`, `require_write`, `require_execute`)

### Risk Considerations

- Route ordering matters in Axum: more specific routes (e.g., `/executions/:id/pause`) must not conflict with catch-all patterns. Axum's router handles this correctly when using `.route()` with distinct paths.
- The `:id` path parameter must be consistent across all execution endpoints (use `Uuid` type for automatic parsing and 400 on invalid UUIDs).

## Status Updates

*To be added during implementation*
