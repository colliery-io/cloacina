---
id: post-executions-trigger-and-get
level: task
title: "POST /executions trigger and GET /executions list and GET /executions/{id} status"
short_code: "CLOACI-T-0197"
created_at: 2026-03-16T21:09:40.276327+00:00
updated_at: 2026-03-16T21:25:21.461406+00:00
parent: CLOACI-I-0032
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0032
---

# POST /executions trigger and GET /executions list and GET /executions/{id} status

## Parent Initiative

[[CLOACI-I-0032]] — Server Phase 4: Core REST API

## Objective

Implement the execution lifecycle endpoints: triggering a new workflow execution (`POST /executions`), listing all executions (`GET /executions`), and retrieving a specific execution's status and result (`GET /executions/{id}`). These endpoints form the core execution management API.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `POST /executions` handler in `routes/executions.rs`:
  - Parses JSON request body as `ExecutionRequest { workflow_name: String, context: serde_json::Value }`
  - Calls `runner.execute_async(name, context)` (via `PipelineExecutor` trait)
  - Returns 202 Accepted with JSON body `{"execution_id": "<uuid>"}`
  - Returns `ApiError` 503 if runner is None
  - Returns `ApiError` 400 if request body is invalid or workflow not found
  - Requires `can_execute` permission + workflow pattern check from ABAC
- [ ] `GET /executions` handler in `routes/executions.rs`:
  - Calls `runner.list_executions()` (via `PipelineExecutor` trait)
  - Returns 200 OK with JSON array of execution summaries
  - Returns `ApiError` 503 if runner is None
  - Requires `can_read` permission
- [ ] `GET /executions/{id}` handler in `routes/executions.rs`:
  - Parses path parameter as `Uuid`
  - Calls `runner.get_execution_result(uuid)` (via `PipelineExecutor` trait)
  - Returns 200 OK with `PipelineResult` serialized as JSON
  - Returns `ApiError` 404 if execution not found (`PipelineError::NotFound`)
  - Returns `ApiError` 503 if runner is None
  - Requires `can_read` permission
- [ ] Request/response serde structs defined: `ExecutionRequest`, `ExecutionResponse`, `ExecutionListResponse`
- [ ] All structs derive `Serialize`, `Deserialize`, and `utoipa::ToSchema`

## Implementation Notes

### Technical Approach

1. **Request/Response types** (defined at the top of `routes/executions.rs`):
   ```rust
   #[derive(Deserialize, ToSchema)]
   pub struct ExecutionRequest {
       pub workflow_name: String,
       pub context: serde_json::Value,
   }

   #[derive(Serialize, ToSchema)]
   pub struct ExecutionResponse {
       pub execution_id: Uuid,
   }

   #[derive(Serialize, ToSchema)]
   pub struct ExecutionListResponse {
       pub executions: Vec<PipelineResult>,
   }
   ```

2. **POST /executions handler**: Use `Json<ExecutionRequest>` extractor. The 202 status code signals that the execution has been accepted and is running asynchronously. The workflow pattern check is performed by the ABAC middleware layer (the permission guard checks `can_execute` on the specific workflow pattern).

3. **GET /executions/{id} handler**: Use `Path<Uuid>` extractor for the ID parameter. The `PipelineError::NotFound` variant maps to 404 via the `From<PipelineError>` impl on `ApiError`.

4. **GET /executions handler**: Returns the full list. Pagination/filtering is a future enhancement (noted in initiative but not in Phase 4 scope).

### Dependencies

- CLOACI-T-0195 — `ApiError` type, `AppState.runner`, module scaffolding
- CLOACI-T-0196 — Workflow must be registered before execution (runtime dependency, not code dependency)
- Library: `PipelineExecutor::execute_async(name, context) -> Result<PipelineExecution, PipelineError>`
- Library: `PipelineExecutor::get_execution_result(uuid) -> Result<PipelineResult, PipelineError>`
- Library: `PipelineExecutor::list_executions() -> Result<Vec<PipelineResult>, PipelineError>`

### Key Design Decisions

- POST returns 202 (Accepted) not 201 (Created) because execution is asynchronous — the pipeline is running, not completed
- The `context` field is `serde_json::Value` to allow arbitrary workflow-specific input without schema coupling
- `ExecutionListResponse` wraps the array in an object for forward-compatible pagination (can add `total`, `offset`, `limit` fields later)

## Status Updates

*To be added during implementation*
