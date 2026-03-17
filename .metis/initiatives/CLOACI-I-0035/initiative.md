---
id: server-phase-7-observability
level: initiative
title: "Server Phase 7: Observability — Prometheus + OpenTelemetry"
short_code: "CLOACI-I-0035"
created_at: 2026-03-16T01:32:38.927304+00:00
updated_at: 2026-03-17T01:52:14.872047+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/active"


exit_criteria_met: false
estimated_complexity: M
initiative_id: server-phase-7-observability
---

# Server Phase 7: Observability — Prometheus + OpenTelemetry Initiative

**Parent tracker**: [[CLOACI-I-0018]]
**Depends on**: CLOACI-I-0029 (Foundation — need HTTP server for /metrics endpoint)
**Blocks**: None

## Context

Zero metrics or tracing infrastructure exists beyond developer-facing `tracing` logs. Operators need Prometheus-compatible metrics for scaling decisions and alerting, and OpenTelemetry traces for request debugging across distributed scheduler/worker instances.

## Goals

- `GET /metrics` — Prometheus-compatible scrape endpoint
- Resource metrics: workers_active, workers_capacity, db_connections_active/idle
- System pressure metrics: scheduler_claim_batch_size, task_queue_depth, task_claim_wait, execution_duration
- Health indicators: pipelines_active/pending, tasks_failed_total, recovery_orphaned
- Per-tenant labels where cardinality is manageable
- Continuous scheduling metrics: graph_metrics() surfaced at /metrics
- OpenTelemetry: OTLP exporter configurable via cloacina.toml (optional, disabled by default)
- `tracing-opentelemetry` integration for distributed traces

## Detailed Design

### Crate Selection

- **`metrics`** crate — facade for recording metrics (counters, gauges, histograms)
- **`metrics-exporter-prometheus`** — renders metrics in Prometheus exposition format
- **`opentelemetry`** + **`opentelemetry-otlp`** — OTLP exporter for traces
- **`tracing-opentelemetry`** — bridges existing `tracing` spans to OpenTelemetry

### Metrics to Instrument

**Resource utilization (gauges):**
- `cloacina_workers_active` — currently executing tasks
- `cloacina_workers_capacity` — max concurrent tasks configured
- `cloacina_db_pool_active` — active DB connections
- `cloacina_db_pool_idle` — idle DB connections

**System pressure (counters + histograms):**
- `cloacina_scheduler_claims_total` — counter of pipeline claim operations
- `cloacina_scheduler_claim_batch_size` — histogram of batch sizes per claim
- `cloacina_task_queue_depth` — gauge of tasks in Ready state
- `cloacina_task_execution_duration_seconds` — histogram of task durations
- `cloacina_pipeline_execution_duration_seconds` — histogram of pipeline durations

**Health indicators (counters + gauges):**
- `cloacina_pipelines_active` — gauge of Running pipelines
- `cloacina_pipelines_pending` — gauge of Pending pipelines
- `cloacina_tasks_completed_total` — counter
- `cloacina_tasks_failed_total` — counter
- `cloacina_recovery_orphaned_total` — counter of recovered tasks

**Continuous scheduling (gauges):**
- `cloacina_continuous_edges_buffered` — gauge per edge (from graph_metrics())
- `cloacina_continuous_edges_max_lag_ms` — gauge per edge

### /metrics Endpoint

Public (no auth), returns Prometheus text format. The `metrics-exporter-prometheus` crate provides a `PrometheusBuilder` that installs a global recorder and provides a render function.

### OpenTelemetry Config

```toml
[observability]
otlp_endpoint = ""  # empty = disabled. e.g., "http://localhost:4317"
otlp_service_name = "cloacina"
```

When configured, `tracing-opentelemetry` layer is added to the subscriber stack. All existing `tracing::info!`/`debug!` spans become OTel spans automatically.

### Instrumentation Points

Metrics are recorded at these points in the codebase:
- **Task execution**: instrument `ThreadTaskExecutor::execute()` with duration histogram + success/failure counter
- **Pipeline lifecycle**: instrument `schedule_workflow_execution()` and completion/failure paths
- **Scheduler loop**: instrument `claim_pipeline_batch()` with batch size + claim duration
- **Recovery**: increment orphaned counter on each recovery
- **Continuous scheduling**: read `graph_metrics()` periodically and set gauges

The `metrics` crate records are zero-cost when no exporter is installed — safe to sprinkle throughout without performance concern.

## Implementation Plan

- [ ] Add `metrics`, `metrics-exporter-prometheus` dependencies to cloacinactl
- [ ] Install PrometheusBuilder recorder in serve startup
- [ ] GET /metrics public endpoint rendering Prometheus text
- [ ] Instrument core metrics: pipelines_active/pending, tasks_completed/failed
- [ ] Add `opentelemetry`, `opentelemetry-otlp`, `tracing-opentelemetry` dependencies
- [ ] OTLP config in cloacina.toml, conditional tracing layer setup
- [ ] Integration test: GET /metrics returns valid Prometheus format
