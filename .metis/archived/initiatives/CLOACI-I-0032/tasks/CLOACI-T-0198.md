---
id: post-executions-id-pause-post
level: task
title: "POST /executions/{id}/pause, POST /executions/{id}/resume, DELETE /executions/{id}"
short_code: "CLOACI-T-0198"
created_at: 2026-03-16T21:09:41.344720+00:00
updated_at: 2026-03-16T21:25:23.284274+00:00
parent: CLOACI-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0032
---

# POST /executions/{id}/pause, POST /executions/{id}/resume, DELETE /executions/{id}

## Parent Initiative

[[CLOACI-I-0032]] — Server Phase 4: Core REST API

## Objective

Implement the execution control endpoints: pause, resume, and cancel. These allow clients to manage running pipeline executions via the REST API, completing the execution lifecycle alongside the CRUD endpoints from CLOACI-T-0197.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /executions/{id}/pause` handler in `routes/executions.rs`:
  - Parses path parameter as `Uuid`
  - Accepts optional JSON body `{"reason": "..."}` (reason defaults to empty string if not provided)
  - Calls `runner.pause_execution(uuid, reason)` (via `PipelineExecutor` trait)
  - Returns 200 OK with `{"status": "paused"}`
  - Returns `ApiError` 404 if execution not found
  - Returns `ApiError` 503 if runner is None
  - Requires `can_execute` permission
- [ ] `POST /executions/{id}/resume` handler in `routes/executions.rs`:
  - Parses path parameter as `Uuid`
  - Calls `runner.resume_execution(uuid)` (via `PipelineExecutor` trait)
  - Returns 200 OK with `{"status": "resumed"}`
  - Returns `ApiError` 404 if execution not found
  - Returns `ApiError` 503 if runner is None
  - Requires `can_execute` permission
- [ ] `DELETE /executions/{id}` handler in `routes/executions.rs`:
  - Parses path parameter as `Uuid`
  - Calls `runner.cancel_execution(uuid)` (via `PipelineExecutor` trait)
  - Returns 200 OK with `{"status": "cancelled"}`
  - Returns `ApiError` 404 if execution not found
  - Returns `ApiError` 503 if runner is None
  - Requires `can_execute` permission
- [ ] Optional `PauseRequest` struct defined for the pause reason body
- [ ] All three handlers use `AppState::require_runner()` consistently

## Implementation Notes

### Technical Approach

1. **Pause handler**:
   ```rust
   #[derive(Deserialize, ToSchema, Default)]
   pub struct PauseRequest {
       #[serde(default)]
       pub reason: String,
   }

   async fn pause_execution(
       State(state): State<Arc<AppState>>,
       Path(id): Path<Uuid>,
       body: Option<Json<PauseRequest>>,
   ) -> Result<Json<Value>, ApiError> {
       let runner = state.require_runner()?;
       let reason = body.map(|b| b.0.reason).unwrap_or_default();
       runner.pause_execution(id, &reason).await?;
       Ok(Json(json!({"status": "paused"})))
   }
   ```

2. **Resume handler**: Simpler than pause — no request body needed. Just `Path<Uuid>` extraction and `runner.resume_execution(uuid)`.

3. **Cancel handler**: Uses `DELETE` method. Same pattern as resume but calls `runner.cancel_execution(uuid)`. Returns `{"status": "cancelled"}`.

4. **JSON body on pause is optional**: Use `Option<Json<PauseRequest>>` so clients can POST with an empty body or with a reason. This makes the API ergonomic for simple "just pause it" use cases.

### Dependencies

- CLOACI-T-0195 — `ApiError` type, `AppState.require_runner()`
- CLOACI-T-0197 — Execution must exist (runtime dependency); shares the same `routes/executions.rs` module
- Library: `PipelineExecutor::pause_execution(uuid, reason) -> Result<(), PipelineError>`
- Library: `PipelineExecutor::resume_execution(uuid) -> Result<(), PipelineError>`
- Library: `PipelineExecutor::cancel_execution(uuid) -> Result<(), PipelineError>`

### Key Design Decisions

- Cancel uses `DELETE /executions/{id}` rather than `POST /executions/{id}/cancel` because cancellation is a destructive operation that terminates the execution. This aligns with REST semantics where DELETE removes a resource.
- All three endpoints return 200 (not 204) so the response body confirms the new status. This is helpful for clients that want confirmation without a follow-up GET.
- The pause reason is optional to keep simple use cases simple while allowing structured audit trails when needed.

## Status Updates

*To be added during implementation*
