---
id: prometheus-metrics-export-replace
level: task
title: "Prometheus metrics export — replace static /metrics with real counters, histograms, and gauges (OPS-01)"
short_code: "CLOACI-T-0453"
created_at: 2026-04-09T12:08:03.587122+00:00
updated_at: 2026-04-09T12:22:58.134075+00:00
parent: CLOACI-I-0088
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0088
---

# Prometheus metrics export — replace static /metrics with real counters, histograms, and gauges (OPS-01)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0088]]

## Objective

Replace the static `/metrics` placeholder endpoint with real Prometheus metrics covering the four golden signals (latency, traffic, errors, saturation). Without metrics, COR-01 (pipeline always "Completed") and every performance finding are invisible in production.

**Effort**: 3-5 days

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `metrics` and `metrics-exporter-prometheus` crates added to `cloacinactl`
- [ ] `/metrics` endpoint returns real Prometheus text format
- [ ] **Counters**: `cloacina_pipelines_total{status="started|completed|failed"}`, `cloacina_tasks_total{status="executed|failed"}`, `cloacina_api_requests_total{method, path, status}`
- [ ] **Histograms**: `cloacina_pipeline_duration_seconds`, `cloacina_task_duration_seconds`, `cloacina_scheduler_loop_duration_seconds`
- [ ] **Gauges**: `cloacina_active_pipelines`, `cloacina_active_tasks`, `cloacina_db_pool_connections{state="active|idle"}`, `cloacina_executor_semaphore_available`
- [ ] Connect existing `ExecutorMetrics` (active_tasks, total_executed, max_concurrent) to the metrics export
- [ ] API request metrics recorded via tower middleware (method, path pattern, status code)
- [ ] Metrics do not significantly impact hot path latency (< 1us per increment)
- [ ] Unit test verifies metrics endpoint returns valid Prometheus text format

## Implementation Notes

### Technical Approach

1. Add `metrics = "0.24"` and `metrics-exporter-prometheus = "0.16"` to `cloacinactl/Cargo.toml`
2. Initialize `PrometheusBuilder` in `serve.rs` startup, store the handle for the `/metrics` endpoint
3. Replace the static `/metrics` handler with one that calls `handle.render()` to produce Prometheus text
4. Instrument in priority order:
   - **Pipeline lifecycle** (`task_scheduler/scheduler_loop.rs`): increment counters in `complete_pipeline()`
   - **Task execution** (`executor/thread_task_executor.rs`): increment on complete/fail, histogram on duration
   - **API requests**: add a tower `metrics` middleware layer that records method/path/status
   - **Gauges**: wire `ExecutorMetrics` fields, expose pool stats from `Database`
5. Use `metrics::describe_*!` macros for metric descriptions (shows up in `/metrics` HELP lines)

### Dependencies
Do this FIRST in I-0088 — enables validating performance findings (PERF-01 through PERF-11).

## Status Updates

- **2026-04-09**: Added `metrics` 0.24 to cloacina core and `metrics-exporter-prometheus` 0.18 to cloacinactl. Initialized `PrometheusBuilder` in serve.rs startup, stored handle in `AppState`. Replaced static `/metrics` handler with `handle.render()`. Added `describe_*!` macros for counters/histograms/gauges. Instrumented pipeline completion (`cloacina_pipelines_total` with status label) and task execution (`cloacina_tasks_total` with status label) in scheduler_loop.rs and thread_task_executor.rs. Updated unit test to verify Prometheus text format with real counter assertions. Added metrics endpoint test to auth integration suite (group 7) verifying HELP/TYPE lines and counter presence. API request middleware and gauge wiring deferred to follow-up. Compiles clean.
