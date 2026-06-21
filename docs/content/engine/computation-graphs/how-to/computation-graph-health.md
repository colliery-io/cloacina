---
title: "Monitoring Computation Graph Health"
description: "How to inspect accumulator and reactor health via the Cloacina API and the cloacinactl graph commands."
weight: 20
aliases:
  - "/computation-graphs/how-to-guides/computation-graph-health/"

---

# Monitoring Computation Graph Health

This guide shows how to use the Cloacina API and the `cloacinactl graph` shortcuts to monitor the health of running computation graphs, accumulators, and reactors.

## Prerequisites

- API server running (see [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}))
- A valid API key stored in `API_KEY` (or a configured `cloacinactl` profile, see [Use CLI profiles]({{< ref "/service/how-to/use-cli-profiles" >}}))
- At least one computation graph registered with the server

## Health vs readiness

Cloacina exposes two levels of health checking:

| Endpoint | Auth | Purpose |
|----------|------|---------|
| `GET /health` | None | Liveness — server is up |
| `GET /ready` | None | Readiness — DB reachable and no graphs crashed |
| `GET /v1/health/accumulators` | Required | Per-accumulator status |
| `GET /v1/health/graphs` | Required | Per-reactor status summary |
| `GET /v1/health/graphs/{name}` | Required | Single reactor detail |

The `/ready` endpoint returns `503 Service Unavailable` when any registered computation graph has crashed (its task has exited), making it suitable for Kubernetes readiness probes and load balancer health checks.

---

## Response envelope

All list endpoints under `/v1/...` use the unified `{items, total}` envelope (CLOACI-T-0594 / API-03). The single-resource `GET /v1/health/graphs/{name}` endpoint returns the bare object, not the envelope.

When parsing responses, drive off `.items[]`, not `.graphs[]` or `.reactors[]` — those were earlier-prototype field names that no longer exist.

---

## Listing accumulator health

```bash
curl -s http://localhost:8080/v1/health/accumulators \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response (each row carries typed freshness — CLOACI-T-0765):

```json
{
  "items": [
    {
      "name": "orderbook",
      "reactor": "market_pipeline_reactor",
      "state": "live",
      "last_event_at": "2026-06-21T20:21:41.283+00:00",
      "events_total": 9861,
      "error": null,
      "status": "live"
    }
  ],
  "total": 1
}
```

`state` is the health label; `last_event_at` + `events_total` are the freshness
signals. To spot a stalled source, watch the **age** of `last_event_at` — the web
UI flags a source as degraded when it stops emitting even though its socket is
still open. (`events_total` is monotonic; a per-minute rate is the delta between
two polls.)

### Accumulator health states

| State | Meaning |
|-------|---------|
| `starting` | Loading checkpoint from DAL — normal at startup |
| `connecting` | Checkpoint loaded, establishing broker connection (stream accumulators) |
| `live` | Connected and processing events normally |
| `disconnected` | Lost broker connection, retrying — data may be stale |
| `socket_only` | No active source (passthrough accumulator) — healthy by definition |

A `disconnected` accumulator continues to accept socket pushes but is not receiving from its broker topic. The reactor that depends on it will enter `degraded` state.

---

## Listing reactor health

```bash
curl -s http://localhost:8080/v1/health/graphs \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "items": [
    {
      "name": "market_pipeline",
      "health": {
        "state": "live"
      },
      "accumulators": ["orderbook", "pricing"],
      "paused": false
    },
    {
      "name": "rate_monitor",
      "health": {
        "state": "warming",
        "healthy": ["exchange_rate_poller"],
        "waiting": ["fx_stream"]
      },
      "accumulators": ["exchange_rate_poller", "fx_stream"],
      "paused": false
    }
  ],
  "total": 2
}
```

### Reactor health states

| State | Meaning |
|-------|---------|
| `starting` | Loading cache from DAL, spawning accumulators |
| `warming` | Some accumulators healthy, waiting for the rest — includes lists of `healthy` and `waiting` names |
| `live` | All accumulators healthy, evaluating reaction criteria |
| `degraded` | Was live, one or more accumulators disconnected — includes list of `disconnected` names |

The `paused` field indicates whether the reactor is accepting boundaries but skipping graph execution (useful for maintenance windows).

### The synthetic `persist` disconnected source (CLOACI-I-0108)

A reactor can also enter `degraded` because of **persistence failures** rather than an accumulator outage. When DAL writes for the reactor's cache / dirty / sequence / save paths fail 5 times in a row, the scheduler downgrades the reactor:

```json
{
  "name": "market_pipeline",
  "health": {
    "state": "degraded",
    "disconnected": ["persist"]
  },
  "accumulators": ["orderbook", "pricing"],
  "paused": false
}
```

`persist` is a **synthetic source name**, not an accumulator — it surfaces "the reactor's checkpoint writes are failing." All real accumulators may be `live`; the reactor itself is at-risk because crash recovery would lose recent boundary state.

When the next persist write succeeds, the reactor is promoted back to `live` automatically.

To distinguish this from a real accumulator disconnect, check whether `persist` appears in the `disconnected` list — if it does, look at the `cloacina_reactor_persist_failures_total` counter (broken down by `kind` label) to identify the failing branch (`cache_serialize`, `dirty_serialize`, `seq_serialize`, `save`).

---

## Getting detail for a specific reactor

```bash
curl -s http://localhost:8080/v1/health/graphs/market_pipeline \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response when healthy:

```json
{
  "name": "market_pipeline",
  "health": {
    "state": "live"
  },
  "accumulators": ["orderbook", "pricing"],
  "paused": false
}
```

Response when degraded:

```json
{
  "name": "market_pipeline",
  "health": {
    "state": "degraded",
    "disconnected": ["orderbook"]
  },
  "accumulators": ["orderbook", "pricing"],
  "paused": false
}
```

Returns `404 Not Found` if the reactor name does not exist.

---

## Inspecting reactor fires

Beyond the cumulative `fires` counter, each reactor keeps a **recent-fires log**
and a **per-minute timeseries** (CLOACI-T-0766) so you can see *what* fired, not
just *how many*.

Recent fires (newest first; `limit` defaults to 50, max 200):

```bash
curl -s "http://localhost:8080/v1/health/reactors/market_pipeline_reactor/fires?limit=5" \
  -H "Authorization: Bearer $API_KEY" | jq
```

```json
{
  "items": [
    { "fired_at": "2026-06-21T20:21:51.300+00:00", "ok": true,  "error": null, "duration_ms": 1 },
    { "fired_at": "2026-06-21T20:21:49.297+00:00", "ok": false, "error": "node 'evaluate' failed: …", "duration_ms": 4 }
  ],
  "total": 2
}
```

Each entry records the outcome (`ok`), the failure detail (`error`), and the
graph execution wall-time (`duration_ms`) — the fastest way to find *why* a
reactor's downstream graph is failing.

Per-minute fire cadence for the last 60 minutes (oldest → newest, gaps zero-filled):

```bash
curl -s "http://localhost:8080/v1/health/reactors/market_pipeline_reactor/fires/timeseries" \
  -H "Authorization: Bearer $API_KEY" | jq -c '.buckets'
# [0,0,0,2870,6906,26]
```

This backs the **fire-activity heatmap** on the graph operational view in the web
UI. For dashboards and alerting, the aggregate counter
`cloacina_reactor_fires_total` and histogram
`cloacina_reactor_fire_duration_seconds` remain the right surface (see the
[metrics catalog](/reference/metrics-catalog/)); the fires log + timeseries are
for at-a-glance operational inspection.

---

## CLI shortcut

The `cloacinactl graph` noun wraps the HTTP endpoints above and prints them in a human-friendly form (or `-o json` for the raw envelope):

```sh
# List every loaded graph (calls /v1/health/graphs)
cloacinactl --profile prod graph list

# Inspect one graph (calls /v1/health/graphs/{name})
cloacinactl --profile prod graph status market_pipeline

# List accumulators (calls /v1/health/accumulators)
cloacinactl --profile prod graph accumulators
```

For scripting, append `-o json` and parse the unified envelope:

```sh
cloacinactl --profile prod graph list -o json | jq '.items[] | select(.health.state != "live")'
```

The CLI honors the same auth tokens as direct `curl` — see [Use CLI profiles]({{< ref "/service/how-to/use-cli-profiles" >}}) for the profile setup.

---

## Readiness integration

The `/ready` endpoint checks both database connectivity and computation graph status:

```bash
curl -s http://localhost:8080/ready | jq
```

Healthy:

```json
{"status": "ready"}
```

Graph crashed:

```json
{
  "status": "not ready",
  "reason": "crashed computation graphs",
  "crashed_graphs": ["market_pipeline"]
}
```

Database unreachable:

```json
{
  "status": "not ready",
  "reason": "database unreachable"
}
```

Use `/ready` as your Kubernetes readiness probe:

```yaml
readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
  failureThreshold: 3
```

A graph enters the "crashed" state when its tokio task exits. This happens if the reactor's `run()` future returns, which normally only occurs after a shutdown signal. An unexpected crash will flip the readiness check and remove the pod from the load balancer until it restarts and the graph re-registers.

> The "crashed" condition is separate from the `Degraded { disconnected: ["persist"] }` state. A `degraded` reactor is still running and `/ready` still returns `200 OK`; readiness flips to `503` only when the reactor's task itself has exited.

---

## Metric-driven monitoring

For long-running monitoring, prefer scraping `/metrics` instead of polling health endpoints. Relevant counters and gauges (full catalog in [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}})):

| Metric | Type | Labels | Use |
|---|---|---|---|
| `cloacina_reactor_persist_failures_total` | counter | `graph`, `reactor`, `kind` | Drives the I-0108 Degraded transition. Alert on non-zero rate. |
| `cloacina_reactor_fire_duration_seconds` | histogram | `graph`, `reactor` | Per-firing latency. |
| `cloacina_reactor_fires_total` | counter | `graph`, `reactor`, `outcome` | Firings per reactor; `outcome` distinguishes success / failure. |
| `cloacina_accumulator_events_total` | counter | `accumulator` | Per-accumulator event throughput. |

Set up an alert on any reactor with persist failure rate `> 0` over a sliding window — that catches the Degraded transition before five consecutive failures.

---

## Monitoring script

Poll all reactors and alert on non-live states:

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${CLOACINA_URL:-http://localhost:8080}"
API_KEY="${API_KEY:?API_KEY must be set}"

graphs=$(curl -sf "${BASE_URL}/v1/health/graphs" \
  -H "Authorization: Bearer ${API_KEY}")

echo "$graphs" | jq -r '.items[] | select(.health.state != "live") |
  "ALERT: reactor \(.name) is \(.health.state) — disconnected: \(.health.disconnected // "none")"'
```

Save as `check-graphs.sh`, make it executable, and run from a cron job or monitoring system.

---

## What to do when a reactor is Degraded

A `degraded` reactor is still running. The action depends on what's in the `disconnected` list.

**If the list contains an accumulator name:** the reactor continues to evaluate reaction criteria and fire the graph using the last known (cached) value from the disconnected accumulator.

1. Identify the disconnected accumulator from the `disconnected` list in the reactor health.
2. Check accumulator health: `GET /v1/health/accumulators` — look for `disconnected` status.
3. Verify the broker is reachable and the topic still exists.
4. Check server logs for the accumulator reconnection attempts.

The reactor recovers automatically when the accumulator reconnects and returns to `live`.

**If the list contains `"persist"`:** see [the I-0108 section](#the-synthetic-persist-disconnected-source-cloaci-i-0108) above. Inspect `cloacina_reactor_persist_failures_total{kind=...}` to identify which DAL write path is failing; common causes are disk pressure, DB connection exhaustion, or a transient DB outage. The reactor recovers on the next successful write.

## Related

- [Monitoring Executions]({{< ref "/embed/how-to/monitoring-executions" >}})
- [Choosing and using accumulator types]({{< ref "accumulator-types" >}})
- [Metrics Catalog]({{< ref "/reference/metrics-catalog" >}})
- [Observability]({{< ref "/service/explanation/observability" >}})
- [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}})
