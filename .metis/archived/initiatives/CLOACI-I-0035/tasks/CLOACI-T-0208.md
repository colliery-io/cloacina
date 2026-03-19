---
id: prometheus-metrics-get-metrics
level: task
title: "Prometheus metrics: GET /metrics endpoint with core gauges and counters"
short_code: "CLOACI-T-0208"
created_at: 2026-03-17T01:52:24.578171+00:00
updated_at: 2026-03-17T02:06:51.373701+00:00
parent: CLOACI-I-0035
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0035
---

# Prometheus metrics: GET /metrics endpoint with core gauges and counters

## Parent Initiative

[[CLOACI-I-0035]]

## Objective

Add a Prometheus-compatible `/metrics` scrape endpoint to the Cloacina HTTP server, backed by the `metrics` + `metrics-exporter-prometheus` crates. This provides production observability by exposing counters, gauges, and histograms in Prometheus text format, scrapable by any Prometheus-compatible monitoring stack.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `metrics` and `metrics-exporter-prometheus` dependencies added to `cloacinactl`
- [ ] New `observability` module in `cloacinactl` that initializes the Prometheus recorder at startup
- [ ] `GET /metrics` public route returns `text/plain; version=0.0.4` Prometheus exposition format
- [ ] Static gauge `cloacina_workers_capacity` emitted at startup with configured `max_concurrent_tasks`
- [ ] Health endpoint increments `cloacina_health_checks_total` counter to prove end-to-end pipeline
- [ ] `PrometheusHandle` stored in a `OnceLock` so the metrics route can render without passing state
- [ ] Returns 503 if the Prometheus recorder was not initialized (e.g., worker-only mode without API)
- [ ] `cargo check -p cloacinactl` passes cleanly

## Implementation Notes

### Technical Approach

1. Add `metrics = "0.24"` and `metrics-exporter-prometheus = "0.16"` to `crates/cloacinactl/Cargo.toml`
2. Create `crates/cloacinactl/src/observability.rs` with `init_prometheus()` (installs global recorder, stores handle in `OnceLock`) and `prometheus_handle()` accessor
3. Create `crates/cloacinactl/src/routes/metrics.rs` with `GET /metrics` handler that calls `prometheus_handle().render()`
4. Wire `/metrics` into the public routes in `serve.rs::app()`
5. Call `init_prometheus()` early in `serve.rs::run()` before building the router
6. Add `metrics::counter!("cloacina_health_checks_total").increment(1)` in `health.rs`
7. Call `record_static_metrics(config.worker.max_concurrent_tasks)` after init

### Dependencies

- `metrics` crate ecosystem (stable, widely used)
- No database or external service dependencies

### Risk Considerations

- The `metrics` crate installs a global recorder; only one can be active per process. Tests that call `init_prometheus()` multiple times must handle the `OnceLock` gracefully.

## Status Updates

*To be added during implementation*
