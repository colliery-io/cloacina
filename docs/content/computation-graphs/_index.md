---
title: "Computation Graphs"
description: "Cloacina's reactive computation graph system — accumulators, reactors, and compiled graph execution for event-driven workloads"
weight: 20
---

# Computation Graphs

The computation graph system is Cloacina's reactive execution engine. It wires data sources through accumulators into compiled graph functions that fire automatically when new data arrives.

## Run Modes

### Library (Embedded)

Define computation graphs with the `#[computation_graph]` macro, wire accumulators and reactors in your Rust application, and push events directly. Best for:

- Real-time data processing pipelines
- Market data and signal processing
- Applications needing reactive computation

[Library Tutorials →]({{< ref "/computation-graphs/tutorials/library" >}})

### Service (Server/Daemon)

Deploy computation graphs as packaged plugins, receive events via WebSocket or Kafka, and monitor graph health via the HTTP API. Best for:

- Production streaming pipelines
- External event ingestion
- Multi-tenant reactive workloads

[Service Tutorials →]({{< ref "/computation-graphs/tutorials/service" >}})

## Quick Navigation

| Section | Description |
|---------|-------------|
| [Tutorials]({{< ref "/computation-graphs/tutorials" >}}) | Step-by-step learning guides |
| [How-to Guides]({{< ref "/computation-graphs/how-to-guides" >}}) | Task-oriented recipes |
| [Reference]({{< ref "/computation-graphs/reference" >}}) | API and configuration lookup |
| [Explanation]({{< ref "/computation-graphs/explanation" >}}) | Architecture and design decisions |
