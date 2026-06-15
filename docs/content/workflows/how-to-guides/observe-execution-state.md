---
title: "Observe Execution State"
description: "Monitor running workflows: metrics, logs, request-id correlation, and OpenTelemetry tracing."
weight: 28
---

# How to Observe Execution State

This guide shows the four observability surfaces Cloacina exposes —
the `/metrics` endpoint, structured logs, the `x-request-id` header,
and OpenTelemetry tracing — and how to wire each into your operations
stack.

> **When to use this:** debugging a stuck workflow, building
> dashboards, correlating a failing API request to a specific
> backend log line, exporting traces to your APM tool.

## The Four Surfaces

| Surface | What it answers | Production-readiness |
|---|---|---|
| `/metrics` (Prometheus) | "How many workflows are running? What's p99 task duration?" | Production-ready. Includes counters, histograms, gauges. |
| Structured logs (stderr + JSON file) | "What did the server do during the last hour?" | Production-ready. Daily rotation, JSON-parseable. |
| `x-request-id` header | "Which log lines correspond to this failed API call?" | Production-ready. Set automatically by middleware. |
| OpenTelemetry tracing | "Where did this 5-second request spend its time?" | Production-ready when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. |

Use them in combination. A typical incident-response loop is: spot a
metric anomaly → grab a `x-request-id` from the affected request →
search logs by request ID → optionally correlate to OTel spans.

## Surface 1: Prometheus Metrics

`cloacina-server` exposes Prometheus metrics at `GET /metrics`
(unauthenticated). The format is `text/plain; version=0.0.4` —
parseable by any Prometheus-compatible scraper.

### Counters

```text
cloacina_workflows_total{status="completed",reason="ok"}
cloacina_workflows_total{status="failed",reason="dependency_failed"}
cloacina_tasks_total{status="completed",reason="ok"}
cloacina_tasks_total{status="failed",reason="task_error"}
cloacina_tasks_total{status="failed",reason="claim_lost"}
cloacina_api_requests_total{method="POST",status="201"}
```

Failure-reason labels for `cloacina_tasks_total`: `task_error`,
`timeout`, `validation_failed`, `infrastructure`, `task_not_found`,
`claim_lost`, `unknown`.

### Histograms

```text
cloacina_api_request_duration_seconds_bucket{method,status,le}
cloacina_workflow_duration_seconds_bucket{le}
cloacina_task_duration_seconds_bucket{le}
```

### Gauges

```text
cloacina_active_workflows
cloacina_active_tasks
```

### Useful Alert Queries

```promql
# Task failure rate spike
rate(cloacina_tasks_total{status="failed"}[5m])
  / rate(cloacina_tasks_total[5m]) > 0.05

# p99 task duration
histogram_quantile(0.99,
    rate(cloacina_task_duration_seconds_bucket[5m]))

# Stuck active workflows (no churn for 30 minutes)
cloacina_active_workflows > 0
  unless changes(cloacina_workflows_total[30m]) > 0
```

### Scraping

Prometheus config:

```yaml
scrape_configs:
  - job_name: cloacina
    static_configs:
      - targets: ['cloacina.internal:8080']
    metrics_path: /metrics
```

> **`/metrics` is unauthenticated.** Reverse-proxy it if you need
> access control. See [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}#operational-caveats).

## Surface 2: Structured Logs

`cloacina-server` (and `cloacina-daemon`) emit dual logs:

- **stderr** — human-readable lines, suitable for `journalctl` or
  Docker logs.
- **`~/.cloacina/logs/cloacina-server.log`** — JSON-structured,
  daily-rotated. The JSON shape is stable across releases.

Every log line includes:

| Field | Always present | Description |
|---|---|---|
| `timestamp` | yes | RFC 3339 with microsecond precision. |
| `level` | yes | `TRACE` / `DEBUG` / `INFO` / `WARN` / `ERROR`. |
| `target` | yes | Module path (e.g., `cloacina_server::routes::workflows`). |
| `message` | yes | The log message. |
| `request_id` | when in a request scope | UUID matching the `x-request-id` response header. |
| `tenant_id` | when applicable | Resolved tenant for the request. |
| `package_id` / `workflow_name` / `task_name` / `execution_id` | when applicable | Domain identifiers. |

### Filtering

Set the level globally via `RUST_LOG`:

```bash
RUST_LOG=info cloacinactl server start ...
RUST_LOG=cloacina=debug,cloacina_server=debug,axum=info \
    cloacinactl server start ...
```

Or use the daemon's `--verbose` / `-v` shortcut (sets `debug`
level).

### Searching

The JSON log is a stream of one object per line. Standard tools:

```bash
# All errors in the last hour
jq 'select(.level == "ERROR")' ~/.cloacina/logs/cloacina-server.log

# All logs for a specific request_id
jq --arg rid "$REQUEST_ID" \
    'select(.request_id == $rid)' \
    ~/.cloacina/logs/cloacina-server.log

# All logs for a specific workflow execution
jq --arg eid "$EXEC_ID" \
    'select(.execution_id == $eid)' \
    ~/.cloacina/logs/cloacina-server.log
```

## Surface 3: Request-ID Correlation

Every HTTP request gets a unique UUID assigned by middleware:

```
> POST /v1/tenants/acme/workflows/etl/execute
< HTTP/1.1 202 Accepted
< x-request-id: 3f6c3dde-8e22-4f4a-bd15-1bea4c2b4f59
```

The same UUID appears as the `request_id` field in every log line
emitted while handling that request — including any internal calls
into the reconciler, scheduler, or DAL.

### Recipe: Correlate a 500 Error to a Stack Trace

```bash
# 1. Capture the request_id from the response (or have your client
#    log it).
REQUEST_ID="3f6c3dde-8e22-4f4a-bd15-1bea4c2b4f59"

# 2. Pull every log line for that request.
jq --arg rid "$REQUEST_ID" \
    'select(.request_id == $rid)' \
    ~/.cloacina/logs/cloacina-server.log
```

The trace will include any `WARN` / `ERROR` lines emitted while the
request was being handled, including the underlying cause.

### Recipe: Surface request_id in client tooling

When your client makes a server call, log the `x-request-id`
response header alongside the result. If the request fails, the
operations team can use that ID to find the server-side root cause
without correlating timestamps.

## Surface 4: OpenTelemetry Tracing

If you set `OTEL_EXPORTER_OTLP_ENDPOINT`, `cloacina-server`
exports spans to a gRPC-based OTLP collector (Jaeger, Tempo,
Honeycomb, Datadog, etc.).

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT="http://otel-collector.internal:4317"
export OTEL_SERVICE_NAME="cloacina-prod"   # Default: "cloacina"

cloacinactl server start ...
```

Spans cover:
- HTTP request lifecycle (one span per request, named
  `request POST /v1/...`).
- Database transactions (when DAL methods are instrumented).
- Reconciler load/unload steps.

The `request_id` field is propagated as a span attribute, so you
can pivot from logs to traces (and vice versa) by ID.

### Without an Exporter

If `OTEL_EXPORTER_OTLP_ENDPOINT` is unset, the OTel pipeline is
disabled and no spans are exported. There is no overhead.

## The Daemon's `daemon status` Command

For local daemon deployments, `cloacinactl daemon status` is a
quick way to see live state:

```text
$ cloacinactl daemon status
Daemon
  Health:               OK
  Uptime:               2d 14h 32m
  Watch directories:    3
  Loaded packages:      7
  Last reconciliation:  2026-04-02T14:31:18 (12s ago)
  Workflows ready:      12
  Workflows running:    2
  Tasks running:        4
```

The values come from the Unix-socket health pulse, so this is a
local probe — no HTTP, no auth.

## Verification Checklist

After wiring observability:

- [ ] `curl http://localhost:8080/metrics | grep cloacina_` returns
  metric output.
- [ ] Log lines include `request_id` when emitted from request
  handlers.
- [ ] An HTTP response includes `x-request-id` in its headers.
- [ ] If using OTel: a request to `/v1/health/status` produces a
  span in your tracer's UI.
- [ ] Prometheus scrapes `cloacina-server` without errors.

## Related

- [HTTP API Reference]({{< ref "/platform/reference/http-api" >}}) — full endpoint surface, including `/metrics` semantics and the operational-caveats section.
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}}) — TLS termination + reverse proxy; how to gate `/metrics` if needed.
- [Daemon Status & Health]({{< ref "/platform/reference/cli" >}}#daemon-status) — full `daemon status` output reference.
- [Metrics Catalog]({{< ref "/platform/reference/metrics-catalog" >}}) — every `cloacina_*` metric with labels, descriptions, and PromQL recipes.
