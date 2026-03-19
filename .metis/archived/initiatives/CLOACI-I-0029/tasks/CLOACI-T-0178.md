---
id: add-utoipa-openapi-generation-and
level: task
title: "Add utoipa OpenAPI generation and Swagger UI endpoint"
short_code: "CLOACI-T-0178"
created_at: 2026-03-16T01:35:10.808703+00:00
updated_at: 2026-03-16T12:53:36.765974+00:00
parent: CLOACI-I-0029
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0029
---

# Add utoipa OpenAPI generation and Swagger UI endpoint

## Objective

Add auto-generated OpenAPI documentation and a Swagger UI endpoint to the Cloacina server. This gives developers an interactive API explorer and a machine-readable OpenAPI spec that can be used for client code generation, testing tools, and documentation.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `utoipa` and `utoipa-swagger-ui` (with axum feature) added as dependencies in `crates/cloacinactl/Cargo.toml`
- [ ] OpenAPI doc struct created with title "Cloacina API", version from `env!("CARGO_PKG_VERSION")`, and a brief description
- [ ] `GET /api-docs/` serves the Swagger UI interactive explorer
- [ ] `GET /api-docs/openapi.json` serves the raw OpenAPI 3.x JSON spec
- [ ] `/health` endpoint from CLOACI-T-0177 annotated with `#[utoipa::path(...)]` and its response type with `utoipa::ToSchema`, so the health endpoint appears in the spec
- [ ] No authentication required on `/api-docs` or `/api-docs/openapi.json`
- [ ] Swagger UI loads correctly in a browser and shows the /health endpoint with request/response schema

## Implementation Notes

Use `utoipa::OpenApi` derive macro to define the API doc struct, listing `/health` in the `paths` attribute and `HealthResponse` in the `schemas` attribute. Mount the Swagger UI using `utoipa_swagger_ui::SwaggerUi::new("/api-docs/{_:.*}").url("/api-docs/openapi.json", ApiDoc::openapi())` nested into the axum Router. The `HealthResponse` struct from CLOACI-T-0177 needs `#[derive(utoipa::ToSchema)]` added. The health handler needs `#[utoipa::path(get, path = "/health", responses((status = 200, body = HealthResponse), (status = 503, body = HealthResponse)))]`. Depends on CLOACI-T-0175 (axum server) and CLOACI-T-0177 (health endpoint).

## Status Updates

### 2026-03-16 — Completed
- Added `utoipa` 5 + `utoipa-swagger-ui` 9 (with axum feature) dependencies
- `ApiDoc` struct with `#[derive(OpenApi)]` — title "Cloacina API", paths=[/health], schemas=[HealthResponse]
- `HealthResponse` annotated with `#[derive(utoipa::ToSchema)]` with doc comments
- `health()` handler annotated with `#[utoipa::path(...)]` with response types
- `GET /api-docs/` serves Swagger UI interactive explorer
- `GET /api-docs/openapi.json` serves OpenAPI 3.1.0 JSON spec
- Note: utoipa-swagger-ui v9 uses plain path `/api-docs/` (not the old `{_:.*}` wildcard pattern)
- Verified: spec generated correctly with /health endpoint, Swagger UI accessible
