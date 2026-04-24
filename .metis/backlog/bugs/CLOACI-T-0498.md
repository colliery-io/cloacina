---
id: audit-metrics-endpoints-coverage
level: task
title: "Audit metrics endpoints — coverage, accuracy, and Prometheus compatibility"
short_code: "CLOACI-T-0498"
created_at: 2026-04-16T12:41:49.569237+00:00
updated_at: 2026-04-23T16:42:14.729611+00:00
parent:
blocked_by: []
archived: false

tags:
  - "#task"
  - "#bug"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: NULL
---

# Audit metrics endpoints — coverage, accuracy, and Prometheus compatibility

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Objective

Deep review of the `/metrics` endpoint and all Prometheus instrumentation. The metrics system was built in I-0088 (CLOACI-T-0453) but has not been validated against real Prometheus scraping, Grafana dashboards, or production load patterns. Need to verify that counters increment correctly, histograms capture meaningful latencies, labels are consistent, and the endpoint is compatible with standard Prometheus tooling.

## Backlog Item Details

### Type
- [x] Bug - Production issue that needs fixing

### Priority
- [x] P2 - Medium (nice to have)

### Impact Assessment
- **Affected Users**: Operators relying on metrics for observability
- **Expected vs Actual**: Unknown — metrics were implemented but never validated against real scraping. Some counters may be phantom (registered but never incremented), label cardinality may be too high, CG-specific metrics (graph fires, accumulator throughput, reactor latency) may be missing entirely.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] Audit all registered metrics — verify each counter/histogram/gauge is actually incremented
- [ ] Verify `/metrics` output parses correctly with `promtool check metrics`
- [ ] Validate CG-specific metrics exist: graph fire count, accumulator events/sec, reactor cache age
- [ ] Validate workflow metrics: task throughput, execution latency, claim rate, failure rate
- [ ] Check label cardinality — no unbounded labels (e.g., task IDs as labels)
- [ ] Verify metrics survive across graph reload / package upgrade
- [ ] Document which metrics exist and what they mean

## Implementation Notes

### Areas to audit
- `crates/cloacina/src/metrics/` — metric definitions
- Server handler middleware — request duration histograms
- Reactor fire path — graph execution counters and latencies
- Accumulator runtime — event throughput counters
- Scheduler loop — claim/heartbeat/sweep counters
- WebSocket handler — connection counts, message throughput

## Status Updates

### 2026-04-22 — Audit pass 1 (code inventory)

**Stack**: `metrics = 0.24` facade (core + server), `metrics-exporter-prometheus = 0.18` installed via `PrometheusBuilder::install_recorder()` in `cloacina-server/src/lib.rs:197-200`. Global recorder, so metrics from the `cloacina` core crate flow into the server's exporter automatically. `/metrics` handler at `lib.rs:544-555` renders via `PrometheusHandle::render()` with `Content-Type: text/plain; version=0.0.4; charset=utf-8`.

**Registered metrics (all 7)**:

| Name | Kind | Labels | Emit site |
|------|------|--------|-----------|
| `cloacina_workflows_total` | counter | `status` | `execution_planner/scheduler_loop.rs:354,364` |
| `cloacina_tasks_total` | counter | `status` | `executor/thread_task_executor.rs:376,404` |
| `cloacina_api_requests_total` | counter | `method`, `status` | `cloacina-server/src/lib.rs:502` |
| `cloacina_workflow_duration_seconds` | histogram | — | `scheduler_loop.rs:374` |
| `cloacina_task_duration_seconds` | histogram | — | `thread_task_executor.rs:907` |
| `cloacina_active_workflows` | gauge | — | inc `execution_planner/mod.rs:422`, dec `scheduler_loop.rs:376` |
| `cloacina_active_tasks` | gauge | — | inc `thread_task_executor.rs:858`, dec `:908` |

**Cardinality** — all labels bounded (status ∈ {completed, failed}; method ∈ HTTP verbs; HTTP status code). No task IDs, tenant IDs, workflow names, or package names leak as labels. ✓

**Gauge parity**:
- `cloacina_active_tasks`: inc/dec in same function body around execution; balanced on all normal return paths. Panic between inc and dec leaks one.
- `cloacina_active_workflows`: inc on scheduling, dec only in `finalize_workflow_execution`. If a workflow is abandoned (crash, claim lost, stale-claim sweeper recovery hands it to a new scheduler), the original inc is never paired with a dec → gauge drifts upward across process lifetime. **Likely leak on crash-recovery path.** Stale-claim sweeper should re-balance or own its own gauge.

**Description/label mismatch**:
- `cloacina_api_requests_total` describe text says "method, path, and status" (`lib.rs:210`) but the emit site only sets `method` + `status` — no path/route label. Either the description is wrong or the label is missing. Given cardinality concerns, the description should be corrected to drop "path".

**Missing coverage (biggest gap)** — task asked for CG-specific metrics; **none exist**. Zero emit sites in `crates/cloacina/src/computation_graph/`, no reactor metrics, no accumulator runtime metrics, no websocket metrics. Specifically absent:
- Graph fire count (per-graph or total)
- Accumulator event throughput (events/sec, events by type)
- Reactor cache age / snapshot staleness gauge
- Reactor fire latency histogram
- Supervisor restart counter (accumulator / reactor crashes)
- WebSocket connection gauge + message throughput
- Scheduler loop health: claim rate, heartbeat emit count, stale-claim sweep count, scan iterations
- Registry/reconciler: package load count, compile duration, reconcile lag
- API request duration histogram (we count requests but don't time them)
- Workflow failure cause breakdown — `status="failed"` has no sub-reason label

**`promtool` validation** — not run. Requires a live server, `curl /metrics`, pipe to `promtool check metrics`. No CI check exists.

**Survives reload/upgrade** — recorder is process-global and independent of runtime state, so package reload / CG reload does not lose metric handles. Process restart resets counters (normal Prometheus semantics). ✓

**Docs** — no user-facing metrics documentation exists.

**Existing tests** — `lib.rs:794 test_metrics_returns_prometheus_format` — only checks the endpoint returns text after three manual increments. Does not exercise real emit sites, does not validate format, does not check label schema.

### Follow-ups created

- **CLOACI-T-0533** — Fix `cloacina_api_requests_total` description mismatch and add request duration histogram.
- **CLOACI-T-0534** — Fix `cloacina_active_workflows` gauge leak on crash-recovery.
- **CLOACI-T-0535** — Add bounded `reason` label to `*_total{status="failed"}` counters.
- **CLOACI-T-0536** — `angreal check metrics-format` in CI (promtool).
- **CLOACI-T-0537** — Operator-facing metrics documentation (`docs/operations/metrics.md`).
- **CLOACI-I-0099** — Initiative: Computation Graph Observability (reactor + accumulator + scheduler + WebSocket metrics; covers the largest audit gap).

### Status
Audit complete. No code changes made — this task is audit + documentation. Remaining ACs that require implementation (promtool CI check, CG metrics, user docs) are larger than a single task and should be split into follow-ups (see recommendations above).
