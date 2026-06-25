---
title: "Manually Driving Graph Surfaces"
description: "How to fire a reactor and inject accumulator events by hand for operations, testing, and incident recovery — via the HTTP API and cloacinactl — and how to discover a surface's typed input slots first."
weight: 25
aliases:
  - "/computation-graphs/how-to-guides/drive-graph-surfaces-manually/"

---

# Manually Driving Graph Surfaces

This guide shows how an operator drives a running computation graph by hand:
**firing a reactor** out-of-band and **injecting an accumulator event**, both
through the HTTP API and the `cloacinactl` shortcuts. It also covers
**discovering the typed input slots** of a surface so you know what JSON to send.

These are operational levers — for testing a graph end-to-end, replaying a known
input during an incident, or kicking a reactor whose normal source has stalled.
They are not part of the normal data path; every manual fire or inject is
**audit-logged as operator-injected** (see [Auditing](#auditing) below).

## Prerequisites

- API server running (see [Deploying the API Server]({{< ref "/service/how-to/deploying-the-api-server" >}}))
- A valid API key in `API_KEY` (or a configured `cloacinactl` profile — see [Use CLI profiles]({{< ref "/service/how-to/use-cli-profiles" >}}))
- The name of the reactor or accumulator you want to drive (list them with the
  health endpoints in [Monitoring Computation Graph Health]({{< ref "computation-graph-health" >}}))

## You send typed JSON, never raw bytes

Every endpoint here takes **typed JSON**. The server encodes that JSON to the
boundary wire format internally — operators never construct or handle the raw
`Vec<u8>` boundary frames the engine uses on its hot path. Send the value as it
appears in your domain (an object, a number, a string); the server takes care of
the encoding and validates it against the surface's declared interface first.

## Discover the typed slots first

Before firing or injecting, ask the surface what inputs it declares. Both
endpoints return a `DeclaredSurface` — its `slots` describe the typed inputs the
server will accept and validate against.

```bash
# Reactor: the per-source input slots used by a fire_with
curl -s http://localhost:8080/v1/health/reactors/market_pipeline_reactor/interface \
  -H "Authorization: Bearer $API_KEY" | jq

# Accumulator: the single event slot used by an inject
curl -s http://localhost:8080/v1/health/accumulators/orderbook/interface \
  -H "Authorization: Bearer $API_KEY" | jq
```

```json
{
  "kind": "reactor",
  "name": "market_pipeline_reactor",
  "slots": [
    { "source": "orderbook", "...": "..." }
  ]
}
```

An **empty `slots`** array means the surface is undeclared/untyped — the server
cannot validate inputs against a schema, so it accepts what you send as-is. A
populated `slots` array is what backs the typed forms in the web UI and the
server-side validation described below.

## Fire a reactor

A manual reactor fire is **full-replace only** — there is no partial/merge mode
in v1. There are two modes:

- **`force_fire`** — fire the graph with the reactor's **current cache**,
  untouched. Send no inputs.
- **`fire_with`** — **replace the entire input cache** with the inputs you
  provide, then fire. Any source not present in your inputs is gone from the
  cache for this fire; it is not merged with what was there.

### `force_fire` (use the current cache)

```bash
curl -s -X POST http://localhost:8080/v1/health/reactors/market_pipeline_reactor/fire \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"mode": "force_fire"}' | jq
```

```sh
cloacinactl --profile prod reactor force-fire market_pipeline_reactor
```

### `fire_with` (replace the cache, then fire)

The `inputs` object maps **source name → typed JSON value**. Each value is
validated against that source's slot from the interface.

```bash
curl -s -X POST http://localhost:8080/v1/health/reactors/market_pipeline_reactor/fire \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
        "mode": "fire_with",
        "inputs": {
          "orderbook": {"bid": 100.5, "ask": 100.7}
        }
      }' | jq
```

With `cloacinactl`, repeat `--input SOURCE=JSON` once per source:

```sh
cloacinactl --profile prod reactor fire market_pipeline_reactor \
  --input 'orderbook={"bid":100.5,"ask":100.7}'
```

The response reports which sources were injected:

```json
{
  "reactor": "market_pipeline_reactor",
  "mode": "fire_with",
  "sources_injected": ["orderbook"]
}
```

If an input fails validation against the reactor's interface, the call returns
`400 Bad Request` with code `reactor_input_invalid`.

## Inject an accumulator event

Injecting feeds a single event into an accumulator exactly as if it had arrived
from the accumulator's real source — the accumulator's `process()` runs, and any
boundary it emits flows to the reactor downstream.

```bash
curl -s -X POST http://localhost:8080/v1/health/accumulators/orderbook/inject \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"event": {"symbol": "ABC", "price": 42.0}}' | jq
```

```sh
cloacinactl --profile prod accumulator inject orderbook \
  --event '{"symbol":"ABC","price":42.0}'
```

The response reports how many downstream consumers received the resulting
boundary:

```json
{
  "accumulator": "orderbook",
  "delivered": 1
}
```

If the event fails validation against the accumulator's interface, the call
returns `400 Bad Request` with code `accumulator_input_invalid`.

## Auditing

Every manual fire and inject is recorded in the audit log flagged as
**operator-injected**, distinct from the engine's own firings. A reactor fire
emits a `reactor_manual_fire` audit event; an accumulator inject emits
`accumulator_manual_inject`. Both carry `operator_injected: true`, so you can
separate operator activity from organic source-driven activity when reviewing
what a graph did and why.

## Quick reference

| Action | HTTP | `cloacinactl` |
|---|---|---|
| Discover reactor slots | `GET /v1/health/reactors/{name}/interface` | — |
| Discover accumulator slots | `GET /v1/health/accumulators/{name}/interface` | — |
| Fire with current cache | `POST /v1/health/reactors/{name}/fire` `{"mode":"force_fire"}` | `reactor force-fire <name>` |
| Fire with replacement cache | `POST /v1/health/reactors/{name}/fire` `{"mode":"fire_with","inputs":{…}}` | `reactor fire <name> --input SOURCE=JSON` |
| Inject an accumulator event | `POST /v1/health/accumulators/{name}/inject` `{"event":{…}}` | `accumulator inject <name> --event JSON` |

## Related

- [Monitoring Computation Graph Health]({{< ref "computation-graph-health" >}}) — list and inspect the surfaces you can drive
- [Reactor]({{< ref "/engine/computation-graphs/reactor" >}}) · [Accumulator]({{< ref "/engine/computation-graphs/accumulator" >}}) · [Boundary event]({{< ref "/engine/computation-graphs/boundary" >}})
- [Triggering Workflows from Reactor Firings]({{< ref "reactor-triggered-workflows" >}})
