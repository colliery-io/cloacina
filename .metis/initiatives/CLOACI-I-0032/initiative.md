---
id: server-phase-4-core-rest-api
level: initiative
title: "Server Phase 4: Core REST API"
short_code: "CLOACI-I-0032"
created_at: 2026-03-16T01:32:35.636577+00:00
updated_at: 2026-03-16T22:10:03.021331+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: L
initiative_id: server-phase-4-core-rest-api
---

# Server Phase 4: Core REST API Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0029 (Foundation), CLOACI-I-0031 (Auth — endpoints need auth)
**Blocks**: None

## Context

The core REST API that makes Cloacina remotely usable. Package upload, execution submission, status queries, and execution control. All endpoints authenticated via PAK + ABAC from Phase 3.

The underlying operations already exist in the library (PackageLoader, DefaultRunner, PipelineExecutor). This phase wraps them in HTTP handlers.

## Goals

- Code-first OpenAPI generation via utoipa
- `POST /workflows/packages` — multipart upload → verify signature → store in registry → reconcile
- `GET /workflows` — list registered workflows
- `POST /executions` — trigger workflow run
- `GET /executions/{id}` — pipeline + task status
- `GET /executions` — list executions (filterable)
- `POST /executions/{id}/pause` — pause pipeline
- `POST /executions/{id}/resume` — resume pipeline
- `DELETE /executions/{id}` — cancel execution
- Consistent JSON error format with error codes

## Detailed Design

### Existing Library APIs (audit: 2026-03-16)

Every endpoint wraps an existing library function — no new engine logic needed:

| Endpoint | HTTP | Library Function | Permission |
|---|---|---|---|
| Upload package | `POST /workflows/packages` | `WorkflowRegistry::register_workflow(Vec<u8>)` → `Uuid` | write |
| List workflows | `GET /workflows` | `WorkflowRegistry::list_workflows()` → `Vec<WorkflowMetadata>` | read |
| Trigger run | `POST /executions` | `PipelineExecutor::execute_async(name, context)` → `PipelineExecution` | execute + workflow pattern |
| Get status | `GET /executions/{id}` | `PipelineExecutor::get_execution_result(uuid)` → `PipelineResult` | read |
| List executions | `GET /executions` | `PipelineExecutor::list_executions()` → `Vec<PipelineResult>` | read |
| Pause | `POST /executions/{id}/pause` | `PipelineExecutor::pause_execution(uuid, reason)` | execute |
| Resume | `POST /executions/{id}/resume` | `PipelineExecutor::resume_execution(uuid)` | execute |
| Cancel | `DELETE /executions/{id}` | `PipelineExecutor::cancel_execution(uuid)` | execute |

### AppState Extension

The handlers need access to DefaultRunner (which implements PipelineExecutor). Add to AppState:

```rust
pub struct AppState {
    pub startup_instant: Instant,
    pub mode: String,
    pub auth_state: Option<AuthState>,
    pub runner: Option<DefaultRunner>,  // None in api-only mode without DB
}
```

### Error Response Format

Consistent JSON error format across all endpoints:

```json
{
    "error": {
        "code": "EXECUTION_NOT_FOUND",
        "message": "Pipeline execution not found: 550e8400-e29b-41d4-a716-446655440000"
    }
}
```

Map library errors to HTTP status codes:
- `PipelineError::NotFound` → 404
- `PipelineError::Configuration` → 400
- `PipelineError::Executor` → 500
- `PipelineError::Timeout` → 408
- `RegistryError` → 400/409/500
- `ValidationError` → 400

### Multipart Package Upload

`POST /workflows/packages` uses axum's `Multipart` extractor:
- Field name: `package` or `file`
- Content-Type: `application/octet-stream` or `multipart/form-data`
- Max size: configurable (default 100MB)
- Signature verification happens inside `register_workflow()`

## Implementation Plan

- [ ] Consistent error response type (ApiError struct + IntoResponse impl)
- [ ] Add DefaultRunner to AppState, wire in serve.rs
- [ ] POST /workflows/packages — multipart upload handler
- [ ] GET /workflows — list workflows handler
- [ ] POST /executions — trigger execution handler (async mode)
- [ ] GET /executions/{id} — get execution result handler
- [ ] GET /executions — list executions handler
- [ ] POST /executions/{id}/pause — pause handler
- [ ] POST /executions/{id}/resume — resume handler
- [ ] DELETE /executions/{id} — cancel handler
- [ ] Wire all routes into protected Router with permission guards
- [ ] utoipa annotations on all endpoints + response schemas
- [ ] Integration test: upload package → trigger execution → poll status → complete
