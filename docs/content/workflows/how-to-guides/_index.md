---
title: "How-to Guides"
description: "Task-oriented recipes for workflow development and operations"
weight: 20
---

# Workflow How-to Guides

Practical recipes for solving specific problems with the workflow system.

## Development

- [Testing Workflows]({{< ref "/workflows/how-to-guides/testing-workflows" >}}) — Unit and integration testing strategies
- [Subscribe a Workflow to a Reactor]({{< ref "/workflows/how-to-guides/subscribe-workflow-to-reactor" >}}) — Wire a workflow trigger to a reactor's firings via the DB-backed subscription fan-out (CLOACI-I-0100)
- [Invoke a Computation Graph from a Workflow Task]({{< ref "/workflows/how-to-guides/invoke-computation-graph-from-workflow" >}}) — Embed a computation graph as a single workflow task using `invokes = computation_graph(...)` (CLOACI-I-0101)

> The Sequential Input Strategy how-to is now under [Computation Graphs → How-to → Sequential Strategy]({{< ref "/computation-graphs/how-to-guides/sequential-strategy" >}}) — it is a reactor-side concern, not a workflow-task strategy.

## Configuration

- [Variable Registry]({{< ref "/workflows/how-to-guides/variable-registry" >}}) — Use CLOACINA_VAR_ env vars for runtime configuration

## Operations

- [Multi-tenant Setup]({{< ref "/workflows/how-to-guides/multi-tenant-setup" >}}) — Configure schema-based isolation
- [Multi-tenant Recovery]({{< ref "/workflows/how-to-guides/multi-tenant-recovery" >}}) — Handle tenant failures and recovery
- [Monitoring Executions]({{< ref "/workflows/how-to-guides/monitoring-executions" >}}) — Track workflow status and health
- [Cleaning Up Events]({{< ref "/workflows/how-to-guides/cleaning-up-events" >}}) — Manage execution history retention

## Migration

- [Migrating to Service Mode]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}}) — Convert an embedded workflow to a packaged deployment
