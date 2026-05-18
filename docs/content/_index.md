---
title: "Cloacina"
description: "Documentation for the Cloacina project"
---

# Cloacina Documentation

Welcome to the Cloacina documentation. This documentation is organized to help you find the information you need quickly and efficiently.

## About Cloacina

Cloacina is a workflow orchestration engine that helps you build resilient task pipelines directly within your applications. Unlike standalone orchestration services, Cloacina embeds into your existing applications to manage complex multi-step workflows with:

- Automatic retries and failure recovery
- State persistence
- Type-safe workflows
- Database-backed execution
- Async-first design
- Content versioning

Whether you're building data processing applications, background job systems, or complex integration workflows, Cloacina provides the tools you need to make your task pipelines reliable, maintainable, and scalable.

## Two execution primitives

Cloacina exposes two complementary execution primitives — pick the one that matches the work:

- **[Workflows]({{< ref "/workflows" >}})** — Durable, DB-backed DAGs. Tasks with dependencies, retries, multi-tenancy, packaging. Pick this when work needs to survive process restart and recover from failures.
- **[Computation Graphs]({{< ref "/computation-graphs" >}})** — In-process, deterministic, event-driven DAGs. Reactors fire compiled graph functions on accumulator boundaries. Pick this when work is event-driven and latency-sensitive.

Both surfaces share the runtime, the multi-tenant model, the packaging format, and the operational surface — and they compose: workflows can subscribe to reactor firings ([Subscribe a workflow to a reactor]({{< ref "/workflows/how-to-guides/subscribe-workflow-to-reactor" >}})), and workflow tasks can invoke embedded computation graphs ([Invoke a computation graph from a workflow task]({{< ref "/workflows/how-to-guides/invoke-computation-graph-from-workflow" >}})).

## Available Libraries

Cloacina provides libraries for multiple programming languages:

- **[Cloacina]({{< ref "/workflows/tutorials/" >}})** — Native Rust library for maximum performance and type safety.
- **[Cloaca]({{< ref "/python/" >}})** — Python bindings providing the same workflow + computation graph surface with Pythonic ergonomics. First-class parity (CLOACI-T-0529 / CLOACI-T-0532), not a feature flag.
- **[`cloacinactl`]({{< ref "/quick-start/install" >}})** — The operator + developer CLI; bundles the daemon as `cloacinactl daemon`. Install with one line: `curl -fsSL https://get.cloacina.dev/install.sh | bash`.

Both libraries share the same core engine and can even share the same database, allowing you to use the best tool for each part of your system.

## See also

- [Quick Start]({{< ref "/quick-start" >}}) — Pick the right tutorial track for your goal.
- [Installing `cloacinactl`]({{< ref "/quick-start/install" >}}) — CLI one-liner + Docker + Helm.
- [Glossary]({{< ref "/glossary" >}}) — Every term used in these docs.
- [Troubleshooting]({{< ref "/troubleshooting" >}}) — Common problems and resolutions.
