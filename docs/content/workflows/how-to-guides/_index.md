---
title: "How-to Guides"
description: "Task-oriented recipes for workflow development and operations"
weight: 20
---

# Workflow How-to Guides

Practical recipes for solving specific problems with the workflow system.

## Development

- [Testing Workflows]({{< ref "/workflows/how-to-guides/testing-workflows" >}}) — Unit and integration testing strategies
- [Sequential Strategy]({{< ref "/workflows/how-to-guides/sequential-strategy" >}}) — Force sequential execution of parallel tasks

## Configuration

- [Variable Registry]({{< ref "/workflows/how-to-guides/variable-registry" >}}) — Use CLOACINA_VAR_ env vars for runtime configuration
- [Custom Task Routing]({{< ref "/workflows/how-to-guides/custom-task-routing" >}}) — Route tasks to different executors by name pattern

## Operations

- [Multi-tenant Setup]({{< ref "/workflows/how-to-guides/multi-tenant-setup" >}}) — Configure schema-based isolation
- [Multi-tenant Recovery]({{< ref "/workflows/how-to-guides/multi-tenant-recovery" >}}) — Handle tenant failures and recovery
- [Monitoring Executions]({{< ref "/workflows/how-to-guides/monitoring-executions" >}}) — Track workflow status and health
- [Cleaning Up Events]({{< ref "/workflows/how-to-guides/cleaning-up-events" >}}) — Manage execution history retention

## Migration

- [Migrating to Service Mode]({{< ref "/workflows/how-to-guides/migrating-to-service-mode" >}}) — Convert an embedded workflow to a packaged deployment
