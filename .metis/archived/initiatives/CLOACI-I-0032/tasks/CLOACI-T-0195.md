---
id: apierror-type-defaultrunner-in
level: task
title: "ApiError type, DefaultRunner in AppState, and route module scaffold"
short_code: "CLOACI-T-0195"
created_at: 2026-03-16T21:09:38.980440+00:00
updated_at: 2026-03-16T21:25:08.951337+00:00
parent: CLOACI-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0032
---

# ApiError type, DefaultRunner in AppState, and route module scaffold

## Parent Initiative

[[CLOACI-I-0032]] — Server Phase 4: Core REST API

## Objective

Establish the foundational error handling, shared application state, and module structure for all Phase 4 REST API endpoints. This task creates the `ApiError` type used by every handler, extends `AppState` with access to `DefaultRunner`, and scaffolds the route modules that subsequent tasks will populate.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `routes/error.rs` exists with an `ApiError` struct containing `code: String` and `message: String`
- [ ] `ApiError` implements `IntoResponse`, mapping library errors to HTTP status codes:
  - `PipelineError::NotFound` -> 404
  - `PipelineError::Configuration` / `ValidationError` -> 400
  - `RegistryError` (conflict) -> 409
  - `PipelineError::Timeout` -> 408
  - `PipelineError::Executor` / other `RegistryError` -> 500
- [ ] JSON error response format matches initiative spec: `{"error": {"code": "...", "message": "..."}}`
- [ ] `AppState` in `routes/health.rs` has `runner: Option<DefaultRunner>` field
- [ ] `serve.rs` passes `runner` into `AppState` when available (None in api-only mode)
- [ ] `routes/workflows.rs` exists as an empty module (scaffolding)
- [ ] `routes/executions.rs` exists as an empty module (scaffolding)
- [ ] `routes/mod.rs` re-exports the new modules (`error`, `workflows`, `executions`)
- [ ] `axum-extra` dependency added to `Cargo.toml` if needed for multipart support
- [ ] Project compiles with no new warnings

## Implementation Notes

### Technical Approach

1. **`routes/error.rs`**: Define `ApiError` with a `status: StatusCode` field (not serialized) plus `code` and `message` fields. Implement `IntoResponse` to produce `(StatusCode, Json<ErrorEnvelope>)` where `ErrorEnvelope` wraps `{"error": {"code", "message"}}`. Provide `From<PipelineError>`, `From<RegistryError>`, and `From<ValidationError>` impls so handlers can use `?` directly.

2. **AppState extension**: Add `pub runner: Option<DefaultRunner>` to the existing `AppState` struct in `routes/health.rs`. This keeps the single shared state struct pattern. A helper method `pub fn require_runner(&self) -> Result<&DefaultRunner, ApiError>` returns 503 SERVICE_UNAVAILABLE when runner is None.

3. **serve.rs wiring**: In the `app()` or server construction function, accept an `Option<DefaultRunner>` parameter and pass it through to `AppState`. When the server is started without a database/runner (api-only mode), this will be `None`.

4. **Module scaffolding**: Create `routes/workflows.rs` and `routes/executions.rs` with minimal content (just `use super::*;` or similar). Update `routes/mod.rs` to declare `pub mod error; pub mod workflows; pub mod executions;`.

### Dependencies

- CLOACI-I-0029 (Foundation) — `AppState`, `serve.rs`, `routes/mod.rs` must already exist
- CLOACI-I-0031 (Auth) — auth middleware and permission guards must be in place
- Library crate types: `PipelineError`, `RegistryError`, `ValidationError`, `DefaultRunner`

### Key Design Decisions

- `ApiError` uses string error codes (e.g., `EXECUTION_NOT_FOUND`, `INVALID_PACKAGE`) rather than numeric codes for better developer experience
- The `require_runner()` helper centralizes the "runner not available" check so every handler doesn't repeat the same pattern
- Error codes are SCREAMING_SNAKE_CASE by convention

## Status Updates

### 2026-03-16 — Completed
- Created routes/error.rs: ApiError with constructors (not_found, bad_request, internal, service_unavailable, conflict, timeout), IntoResponse impl, From<PipelineError> conversion
- Added `runner: Option<DefaultRunner>` to AppState, wired in serve.rs run()
- Created routes/workflows.rs and routes/executions.rs modules
- Updated routes/mod.rs with all new modules
- Added axum `multipart` feature to Cargo.toml
- Updated all 5 test AppState constructions with runner: None
