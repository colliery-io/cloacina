---
title: "Tutorials"
description: "Step-by-step guides to help you learn Cloacina"
weight: 20
reviewer: "dstorey"
review_date: "2024-03-19"
---

# Tutorials

Welcome to the Cloacina tutorials. Cloacina has two core capabilities, each with library (embedded) and service (server) modes:

## Workflows (Unified Scheduler)

DAG-based task pipelines with dependencies, retries, triggers, and cron scheduling.

- **[Library Tutorials]({{< ref "workflows/library" >}})** — Define and run workflows directly in your Rust application
- **[Service Tutorials]({{< ref "workflows/service" >}})** — Package and deploy workflows on the API server

## Computation Graphs (Reactive Scheduler)

Event-driven computation graphs with accumulators, reactors, and compiled graph execution.

- **[Library Tutorials]({{< ref "computation-graphs/library" >}})** — Define graphs, wire accumulators and reactors in your application
- **[Service Tutorials]({{< ref "computation-graphs/service" >}})** — Deploy graphs with WebSocket endpoints on the API server

## Prerequisites

Before starting any tutorial, ensure you have the following installed:

- Git
- Docker Compose
- Angreal (`pip install angreal`)

## Running Tutorials with Angreal

```bash
git clone https://github.com/colliery-io/cloacina.git
cd cloacina
angreal demos tutorial-01
```

{{< hint type=tip >}}
You can run the tutorials from any directory. The `angreal` command handles everything.
{{< /hint >}}

{{< toc-tree >}}
