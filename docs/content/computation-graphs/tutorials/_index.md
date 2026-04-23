---
title: "Tutorials"
description: "Step-by-step guides for learning Cloacina's computation graph system"
weight: 10
---

# Computation Graph Tutorials

Learn Cloacina's computation graph system through progressive, hands-on tutorials.

## Library (Embedded)

Start here. These tutorials teach you to define and run computation graphs directly in your Rust application.

1. [Your First Graph]({{< ref "/computation-graphs/tutorials/library/07-computation-graph" >}}) — Define nodes, declare topology, run a compiled graph
2. [Accumulators]({{< ref "/computation-graphs/tutorials/library/08-accumulators" >}}) — Buffer events with accumulators and wire to reactors
3. [Full Pipeline]({{< ref "/computation-graphs/tutorials/library/09-full-pipeline" >}}) — Multiple accumulators, reactor, when_any, terminal outputs
4. [Routing]({{< ref "/computation-graphs/tutorials/library/10-routing" >}}) — Multi-branch graphs with conditional routing

## Service (Server Mode)

Deploy computation graphs as services with external event ingestion.

5. [Packaging]({{< ref "/computation-graphs/tutorials/service/07-packaging" >}}) — Build .cloacina packages for computation graphs
6. [WebSocket Events]({{< ref "/computation-graphs/tutorials/service/08-websocket-events" >}}) — Receive events via WebSocket endpoints
7. [Kafka Streaming]({{< ref "/computation-graphs/tutorials/service/09-kafka-stream" >}}) — Connect to Kafka for high-throughput event processing
