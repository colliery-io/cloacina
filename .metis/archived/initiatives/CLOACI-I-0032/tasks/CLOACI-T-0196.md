---
id: post-workflows-packages-multipart
level: task
title: "POST /workflows/packages multipart upload and GET /workflows list"
short_code: "CLOACI-T-0196"
created_at: 2026-03-16T21:09:39.332762+00:00
updated_at: 2026-03-16T21:25:20.251911+00:00
parent: CLOACI-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0032
---

# POST /workflows/packages multipart upload and GET /workflows list

## Parent Initiative

[[CLOACI-I-0032]] — Server Phase 4: Core REST API

## Objective

Implement the workflow management endpoints: multipart package upload (`POST /workflows/packages`) and workflow listing (`GET /workflows`). These are the first "real" API handlers in the server, enabling clients to register workflow packages and discover available workflows.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /workflows/packages` handler in `routes/workflows.rs`:
  - Extracts multipart form data using `axum::extract::Multipart`
  - Reads the `package` (or `file`) field bytes from the multipart body
  - Calls `runner.workflow_registry().register_workflow(bytes)` to register the package
  - Returns 201 Created with JSON body `{"id": "<uuid>"}`
  - Returns `ApiError` 503 if runner is None (server in api-only mode)
  - Returns `ApiError` 400 if multipart field is missing or unreadable
  - Returns `ApiError` 409 if package already registered (from `RegistryError`)
- [ ] `GET /workflows` handler in `routes/workflows.rs`:
  - Calls `runner.workflow_registry().list_workflows()`
  - Returns 200 OK with JSON array of workflow metadata objects
  - Returns `ApiError` 503 if runner is None
- [ ] POST handler requires `can_write` permission (enforced by route guard)
- [ ] GET handler requires `can_read` permission (enforced by route guard)
- [ ] Both handlers use `AppState::require_runner()` from CLOACI-T-0195

## Implementation Notes

### Technical Approach

1. **POST /workflows/packages handler**:
   ```rust
   async fn upload_package(
       State(state): State<Arc<AppState>>,
       mut multipart: Multipart,
   ) -> Result<(StatusCode, Json<Value>), ApiError> {
       let runner = state.require_runner()?;
       // iterate multipart fields, find "package" or "file"
       // read field bytes
       let id = runner.workflow_registry().register_workflow(bytes).await?;
       Ok((StatusCode::CREATED, Json(json!({"id": id}))))
   }
   ```

2. **GET /workflows handler**:
   ```rust
   async fn list_workflows(
       State(state): State<Arc<AppState>>,
   ) -> Result<Json<Vec<WorkflowMetadata>>, ApiError> {
       let runner = state.require_runner()?;
       let workflows = runner.workflow_registry().list_workflows().await?;
       Ok(Json(workflows))
   }
   ```

3. **Multipart handling**: Use `axum::extract::Multipart` (built into axum) or `axum_extra::extract::Multipart` depending on version. Iterate fields with `while let Some(field) = multipart.next_field().await?`, match on field name, and collect bytes with `field.bytes().await?`.

### Dependencies

- CLOACI-T-0195 — `ApiError` type, `AppState.runner`, module scaffolding
- Library: `WorkflowRegistry::register_workflow(Vec<u8>) -> Result<Uuid, RegistryError>`
- Library: `WorkflowRegistry::list_workflows() -> Result<Vec<WorkflowMetadata>, RegistryError>`

### Key Design Decisions

- The upload endpoint accepts `multipart/form-data` rather than raw binary to allow future extension (e.g., metadata fields alongside the package)
- Field name is flexible: accepts both `package` and `file` for convenience
- The 201 response includes only the UUID; clients can call `GET /workflows` to retrieve full metadata

## Status Updates

*To be added during implementation*
