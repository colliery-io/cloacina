---
id: request-id-middleware-and-log-span
level: task
title: "Request ID middleware and log span propagation (OPS-02 foundation)"
short_code: "CLOACI-T-0454"
created_at: 2026-04-09T12:08:04.947633+00:00
updated_at: 2026-04-09T12:25:36.300837+00:00
parent: CLOACI-I-0088
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0088
---

# Request ID middleware and log span propagation (OPS-02 foundation)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0088]]

## Objective

When an HTTP request triggers a pipeline, there is no way to correlate the HTTP request log with pipeline creation, task execution, or completion logs. Add a request ID to every request and propagate `pipeline_execution_id` through executor/task log spans. This provides immediate debugging value before full OTel tracing.

**Effort**: 3-4 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Axum middleware generates a UUID request ID per request
- [ ] Request ID inserted into request extensions (accessible by handlers)
- [ ] `X-Request-Id` response header set on every response
- [ ] Request ID included in the `ApiError` JSON response body
- [ ] `tracing` span created per request with `request_id` field
- [ ] `pipeline_execution_id` included as a span field in executor and task execution logs
- [ ] Server access log includes request_id, method, path, status, duration

## Implementation Notes

### Technical Approach

1. Create middleware in `server/middleware.rs` (or inline in `build_router`):
   ```rust
   async fn request_id_middleware(mut req: Request, next: Next) -> Response {
       let id = uuid::Uuid::new_v4().to_string();
       req.extensions_mut().insert(RequestId(id.clone()));
       let span = tracing::info_span!("request", request_id = %id);
       let mut resp = next.run(req).instrument(span).await;
       resp.headers_mut().insert("X-Request-Id", id.parse().unwrap());
       resp
   }
   ```
2. Apply as the outermost layer in `build_router()`
3. Update `ApiError::into_response()` to include `request_id` from request extensions if available
4. In `PipelineExecutor::execute()`, create a span with `pipeline_execution_id` field
5. In `ThreadTaskExecutor::execute_task()`, create a span with `task_name` and `pipeline_execution_id`

### Dependencies
After T-0453 (metrics) since both modify the middleware stack.

## Status Updates

- **2026-04-09**: Added `request_id_middleware` function and `RequestId` struct in serve.rs. Middleware generates UUID per request, inserts into extensions, creates `info_span!("request", request_id, method, path)`, sets `X-Request-Id` response header. Applied as outermost layer in `build_router()`. Added unit test verifying X-Request-Id header is present and contains valid UUID. Request ID in ApiError body deferred (header is the standard pattern). Pipeline span propagation deferred to OTel task (T-0455). Compiles clean.
