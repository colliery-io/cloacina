---
id: install-prometheus-recorders-in
level: initiative
title: "Install Prometheus recorders in cloacina-compiler and cloacinactl daemon"
short_code: "CLOACI-I-0109"
created_at: 2026-05-06T11:05:37.562963+00:00
updated_at: 2026-05-14T15:21:32.530841+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: install-prometheus-recorders-in
---

# Install Prometheus recorders in cloacina-compiler and cloacinactl daemon Initiative

## Context

`cloacina-server` exposes Prometheus metrics on `/metrics`, validated by `promtool` in CI (CLOACI-T-0536). The same engine code runs in `cloacina-compiler` and `cloacinactl daemon`, but neither installs a Prometheus recorder, so neither emits metrics to Prometheus. Self-hosted operators monitoring a SQLite-backed daemon, or a build-worker pool, must DB-query for everything.

This initiative installs Prometheus recorders on both binaries with metrics meaningful to their operational role.

## Goals & Non-Goals

**Goals:**
- `cloacina-compiler` exposes `/metrics` with build-relevant counters/histograms/gauges.
- Compiler endpoint validated by `angreal test:metrics-format` (promtool).
- Log retention is configurable on all three deployables (compiler, server, daemon) via `--log-retention-days`.

**Non-Goals:**
- **Daemon `/metrics`** — `cloacinactl daemon` is a hobbyist-tier local process per CLOACI-A-0005's deployment-mode trust model. There's no Prometheus to scrape it, an HTTP listener adds unnecessary attack surface, and the observability story for a local daemon is logs. OPS-05 is closed as won't-fix for this reason; if a "daemon-as-service" deployment mode ever emerges, revisit.
- Distributed tracing for compiler (separate concern).
- Reworking the engine's metric schema. Reuse what exists.
- Custom dashboards/alerts. Exposition is enough.

## Source Findings (May 2026 review)

- **OPS-04 (Major)** — Compiler is blind to Prometheus.
- **OPS-05 (Major)** — Daemon is blind to Prometheus.
- **OPS-06 (Minor)** — No log retention.

## Discovery Questions

- **Compiler metric set**: `cloacina_compiler_builds_total{status}`, `cloacina_compiler_build_duration_seconds` histogram, `cloacina_compiler_queue_depth{state}` gauge (re-seeded SQL-derived), `cloacina_compiler_sweep_resets_total`, `cloacina_compiler_heartbeat_failures_total`. Right starter set?
- **Daemon metric bind**: `--metrics-bind 127.0.0.1:9091` separate from the Unix socket health endpoint? Or share?
- **Retention default**: 14 days. Operator-configurable via flag and config.
- **Dependency on I-0108**: REC-06's SQL-derived gauge pattern extends to the compiler queue depth; should we wait for I-0108 to land first, or implement queue-depth re-seed as part of this initiative?

## Initial Sketch

- Day 1: Compiler `/metrics` with the metric set above. SQL-derived re-seed on queue depth per the REC-06 pattern.
- Day 2: Daemon Prometheus recorder + `--metrics-bind` + emits engine's existing 8 metrics.
- Day 3: Add to `angreal test:metrics-format`. Wire `--log-retention-days` on all three deployables (default 14) via `tracing_appender::rolling::Builder::max_log_files(N)`.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- `curl http://compiler:9000/metrics` returns Prometheus-formatted output validated by `promtool`.
- `curl http://daemon:9091/metrics` returns the engine-emitted metrics.
- Old log files are pruned per the configured retention.
- `angreal test:metrics-format` covers compiler and daemon endpoints.

## References

- `review/06-operability.md` — OPS-04, OPS-05, OPS-06
- `review/10-recommendations.md` — REC-11
- Prior task: CLOACI-T-0453 (Prometheus metrics export for the engine, completed)
- Prior task: CLOACI-T-0536 (promtool /metrics format check in CI, completed)
- Related: CLOACI-I-0108 (gauge leak fix; REC-06 pattern extension)
