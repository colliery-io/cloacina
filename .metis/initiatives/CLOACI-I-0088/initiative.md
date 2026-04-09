---
id: observability-prometheus-metrics
level: initiative
title: "Observability — Prometheus Metrics and Distributed Tracing"
short_code: "CLOACI-I-0088"
created_at: 2026-04-08T10:46:50.655076+00:00
updated_at: 2026-04-09T12:07:56.875442+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/decompose"


exit_criteria_met: false
estimated_complexity: M
initiative_id: observability-prometheus-metrics
---

# Observability — Prometheus Metrics and Distributed Tracing Initiative

*Source: Architecture Review (review/10-recommendations.md) — Phase 4: Observability*

## Context

The `/metrics` endpoint is a static placeholder with no real Prometheus metrics. There is no distributed tracing or request correlation. The four golden signals (latency, traffic, errors, saturation) are unmeasured. COR-01 (pipeline always "Completed") would be invisible in production without metrics. When an HTTP request triggers a pipeline, there is no way to correlate HTTP logs with execution logs.

## Goals & Non-Goals

**Goals:**
- Replace static `/metrics` with real Prometheus metrics export (OPS-01)
- Instrument critical paths: pipeline/task counters, execution duration histograms, pool gauges
- Add distributed tracing via `tracing-opentelemetry` with OTLP export (OPS-02)
- Add request-scoped spans with request IDs in axum middleware
- Propagate `pipeline_execution_id` through executor and task log spans

**Non-Goals:**
- Custom dashboards or Grafana configs (operator concern)
- Application-level business metrics (future work)

## Detailed Design

### REC-11: Metrics Export (OPS-01) — 3-5 days

Add `prometheus` or `metrics` crate with `metrics-exporter-prometheus` backend. Instrument in priority order:
1. **Counters**: pipelines_started/completed/failed, tasks_executed/failed, api_requests (by method, path, status)
2. **Histograms**: pipeline_execution_duration_seconds, task_execution_duration_seconds, scheduler_loop_duration_seconds
3. **Gauges**: active_pipelines, active_tasks, connection_pool_size/available, executor_semaphore_available

The `ExecutorMetrics` struct already tracks `active_tasks`, `total_executed`, `max_concurrent` internally.

### REC-17: Distributed Tracing (OPS-02) — 3-5 days

1. Add `tracing-opentelemetry` and OTLP exporter behind a feature flag.
2. Create request-scoped span in axum middleware with generated request ID.
3. Propagate `pipeline_execution_id` through executor/task logs as span field.
4. Accept `traceparent` headers for distributed trace propagation.
5. Configure via `OTEL_EXPORTER_OTLP_ENDPOINT` env var.

## Implementation Plan

Metrics first (enables validation of performance findings PERF-01 through PERF-11), then tracing. Target: 1-2 weeks.
