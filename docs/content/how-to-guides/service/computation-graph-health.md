---
title: "Monitoring Computation Graph Health"
description: "How to inspect accumulator and reactor health using the Cloacina API"
weight: 20
---

# Monitoring Computation Graph Health

This guide shows how to use the Cloacina API to monitor the health of running computation graphs, accumulators, and reactors.

## Prerequisites

- API server running (see [Deploying the API Server]({{< ref "deploying-the-api-server" >}}))
- A valid API key stored in `API_KEY`
- At least one computation graph registered with the server

## Health vs readiness

Cloacina exposes two levels of health checking:

| Endpoint | Auth | Purpose |
|----------|------|---------|
| `GET /health` | None | Liveness — server is up |
| `GET /ready` | None | Readiness — DB reachable and no graphs crashed |
| `GET /v1/health/accumulators` | Required | Per-accumulator status |
| `GET /v1/health/reactors` | Required | Per-reactor status summary |
| `GET /v1/health/reactors/{name}` | Required | Single reactor detail |

The `/ready` endpoint returns `503 Service Unavailable` when any registered computation graph has crashed (its task has exited), making it suitable for Kubernetes readiness probes and load balancer health checks.

---

## Listing accumulator health

```bash
curl -s http://localhost:8080/v1/health/accumulators \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "accumulators": [
    {
      "name": "orderbook",
      "status": "live"
    },
    {
      "name": "pricing",
      "status": "live"
    },
    {
      "name": "exchange_rate_poller",
      "status": "live"
    }
  ]
}
```

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
curl -s http://localhost:8080/v1/health/reactors \
  -H "Authorization: Bearer $API_KEY" | jq
```

Response:

```json
{
  "reactors": [
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
  ]
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

---

## Getting detail for a specific reactor

```bash
curl -s http://localhost:8080/v1/health/reactors/market_pipeline \
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

---

## Monitoring script

Poll all reactors and alert on non-live states:

```bash
#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${CLOACINA_URL:-http://localhost:8080}"
API_KEY="${API_KEY:?API_KEY must be set}"

reactors=$(curl -sf "${BASE_URL}/v1/health/reactors" \
  -H "Authorization: Bearer ${API_KEY}" | jq -r '.reactors[]')

echo "$reactors" | jq -r 'select(.health.state != "live") |
  "ALERT: reactor \(.name) is \(.health.state) — disconnected: \(.health.disconnected // "none")"'
```

Save as `check-graphs.sh`, make it executable, and run from a cron job or monitoring system.

---

## What to do when a reactor is Degraded

A `degraded` reactor is still running. It continues to evaluate reaction criteria and fire the graph using the last known (cached) value from the disconnected accumulator.

Steps to investigate:

1. Identify the disconnected accumulator from the `disconnected` list in the reactor health.
2. Check accumulator health: `GET /v1/health/accumulators` — look for `disconnected` status.
3. Verify the broker is reachable and the topic still exists.
4. Check server logs for the accumulator reconnection attempts.

A `degraded` reactor recovers automatically when the accumulator reconnects and returns to `live` status. No manual intervention is required unless the broker is permanently gone.

## Related

- [Monitoring Executions]({{< ref "monitoring-executions" >}})
- [Choosing and using accumulator types]({{< ref "how-to-guides/library/accumulator-types" >}})
- [Deploying the API Server]({{< ref "deploying-the-api-server" >}})
