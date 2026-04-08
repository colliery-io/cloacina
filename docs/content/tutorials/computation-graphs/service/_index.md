---
title: "Service (Server Mode)"
description: "Deploy computation graphs on the Cloacina API server with WebSocket endpoints and Kafka streams"
weight: 20
---

# Computation Graph Tutorials — Service Mode

These tutorials cover deploying computation graphs as packaged plugins on the Cloacina API server. You'll compile a graph to a shared library, upload it via the REST API, and drive it with events from WebSocket connections or Kafka topics.

Assume you've completed the [library tutorials]({{< ref "/tutorials/computation-graphs/library/" >}}) and are familiar with the `#[computation_graph]` macro and the accumulator/reactor model.

## Tutorials in this section

### [07 - Packaging a Computation Graph]({{< ref "/tutorials/computation-graphs/service/07-packaging/" >}})

Build a minimal computation graph as a `cdylib`, wrap it in a `.cloacina` source archive, and upload it to the server. Watch the reconciler compile and load it, then confirm the graph is live via the health endpoints.

### [08 - WebSocket Event Injection]({{< ref "/tutorials/computation-graphs/service/08-websocket-events/" >}})

Connect to `/v1/ws/accumulator/{name}` and push JSON events into a running accumulator. Covers the WebSocket framing protocol, PAK token auth, persistent high-throughput connections, and the reactor WebSocket for manual `ForceFire`/`Pause`/`Resume` commands.

### [09 - Kafka-Sourced Computation Graphs]({{< ref "/tutorials/computation-graphs/service/09-kafka-stream/" >}})

Declare `accumulator_type = "stream"` in `package.toml` so the server subscribes to a Kafka topic automatically. Covers three patterns: passthrough (fire on every message), stateful multi-source (fire when all inputs are present), and batch (accumulate N messages before firing).
