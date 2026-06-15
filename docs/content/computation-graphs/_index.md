---
title: "Computation Graphs"
description: "Cloacina's computation graph system — accumulators, reactors, and compiled graph execution for event-driven workloads"
weight: 20
---

# Computation Graphs

The computation graph system is Cloacina's event-driven execution engine. It wires data sources through accumulators into compiled graph functions that fire automatically when new data arrives.

**Who this is for:** developers building event-driven, low-latency processing that
reacts to a stream. **Prerequisites:** the
[Concepts]({{< ref "/start/concepts" >}}) (reactor, accumulator, computation
graph). These docs are Rust-first; the same surface is available in Python — see
[Python · Computation Graphs]({{< ref "/engine/computation-graphs" >}}).

## Run Modes

### Library (Embedded)

Define computation graphs with the `#[computation_graph]` macro, wire accumulators and reactors in your Rust application, and push events directly. Best for:

- Real-time data processing pipelines
- Market data and signal processing
- Applications needing event-driven computation

[Library Tutorials →]({{< ref "/embed/tutorials" >}})

### Service (Server/Daemon)

Deploy computation graphs as packaged plugins, receive events via WebSocket or Kafka, and monitor graph health via the HTTP API. Best for:

- Production streaming pipelines
- External event ingestion
- Multi-tenant event-driven workloads

[Service Tutorials →]({{< ref "/service/tutorials" >}})

## Quick Navigation

| Section | Description |
|---------|-------------|
| [Tutorials]({{< ref "/embed/tutorials" >}}) | Step-by-step learning guides |
| [How-to Guides]({{< ref "/engine/computation-graphs/how-to" >}}) | Task-oriented recipes |
| [Reference]({{< ref "/reference" >}}) | API and configuration lookup |
| [Explanation]({{< ref "/engine/explanation" >}}) | Architecture and design decisions |
