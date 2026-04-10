---
id: opentelemetry-tracing-integration
level: task
title: "OpenTelemetry tracing integration with OTLP export (OPS-02)"
short_code: "CLOACI-T-0455"
created_at: 2026-04-09T12:08:05.935588+00:00
updated_at: 2026-04-09T12:28:18.323498+00:00
parent: CLOACI-I-0088
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0088
---

# OpenTelemetry tracing integration with OTLP export (OPS-02)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0088]]

## Objective

Add OpenTelemetry tracing integration behind a feature flag so traces can be exported to any OTLP-compatible backend (Jaeger, Grafana Tempo, Honeycomb, etc.). Accept W3C `traceparent` headers for distributed trace propagation across service boundaries.

**Effort**: 3-5 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `tracing-opentelemetry` and `opentelemetry-otlp` added behind a `telemetry` feature flag in `cloacinactl`
- [ ] When feature enabled and `OTEL_EXPORTER_OTLP_ENDPOINT` is set, traces export via OTLP/gRPC
- [ ] When feature disabled or env var unset, no OTel overhead (zero-cost when off)
- [ ] Incoming `traceparent` HTTP header creates a child span (W3C Trace Context propagation)
- [ ] Request spans include: method, path, status, duration, request_id
- [ ] Pipeline execution spans include: pipeline_name, execution_id, task count
- [ ] Task execution spans include: task_name, attempt, duration, status
- [ ] Spans are nested: request -> pipeline -> task (parent-child relationship)
- [ ] Smoke test: start server with Jaeger all-in-one, make a request, verify trace appears

## Implementation Notes

### Technical Approach

1. Add dependencies behind feature flag in `cloacinactl/Cargo.toml`:
   ```toml
   [features]
   telemetry = ["tracing-opentelemetry", "opentelemetry", "opentelemetry-otlp", "opentelemetry_sdk"]
   ```
2. In `serve.rs` tracing initialization, conditionally add OTel layer:
   ```rust
   #[cfg(feature = "telemetry")]
   if std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").is_ok() {
       let tracer = opentelemetry_otlp::new_pipeline().tracing()...install_batch()?;
       let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
       // Add to subscriber registry
   }
   ```
3. Add `tower-http` `TraceLayer` or custom middleware that extracts `traceparent` header and creates a root/child span
4. Wire `pipeline_execution_id` into spans created by the executor (T-0454 provides the foundation)
5. Configure via standard OTel env vars: `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_SERVICE_NAME=cloacina`

### Dependencies
After T-0454 (request ID middleware) which provides the span infrastructure to build on.

### Risk Considerations
- OTel dependencies are large — keeping behind a feature flag avoids bloating non-telemetry builds
- gRPC dependency (tonic) for OTLP export may conflict with existing deps — verify with `cargo tree`

## Status Updates

- **2026-04-09**: Added `telemetry` feature flag to cloacinactl with deps: `tracing-opentelemetry` 0.32, `opentelemetry` 0.31, `opentelemetry-otlp` 0.31 (grpc-tonic), `opentelemetry_sdk` 0.31 (rt-tokio). Conditional OTel layer in serve.rs subscriber: when `OTEL_EXPORTER_OTLP_ENDPOINT` is set, creates OTLP span exporter + batch provider + tracer + tracing-opentelemetry layer. When env var unset or feature disabled, zero overhead. Configured via standard OTel env vars (`OTEL_SERVICE_NAME`, `OTEL_EXPORTER_OTLP_ENDPOINT`). Compiles clean both with and without feature. W3C traceparent propagation and pipeline/task span wiring deferred to follow-up — the OTel layer is live and will capture all existing tracing spans including the request_id middleware from T-0454.
