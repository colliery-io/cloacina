---
title: "Workflows"
description: "Cloacina's workflow orchestration system — DAG-based task pipelines with retries, dependencies, triggers, and cron scheduling"
weight: 10
---

# Workflows

The workflow system is Cloacina's core orchestration engine. It schedules and executes directed acyclic graphs (DAGs) of tasks with automatic retry, conditional execution, cron scheduling, and persistent state.

## Run Modes

### Library (Embedded)

Use Cloacina as a Rust library embedded directly in your application. Define tasks with `#[task]`, compose them into workflows with `#[workflow]`, and execute them with `DefaultRunner`. Best for:

- Applications that need built-in workflow orchestration
- Microservices with internal task pipelines
- Development and testing

[Library Tutorials →]({{< ref "/workflows/tutorials/library" >}})

### Service (Server/Daemon)

Deploy Cloacina as a standalone service (`cloacinactl serve` or `cloacinactl daemon`). Upload packaged workflows via HTTP API, manage tenants, and execute via REST. Best for:

- Multi-tenant SaaS platforms
- Centralized workflow management
- Production deployments with monitoring

[Service Tutorials →]({{< ref "/workflows/tutorials/service" >}})

## Quick Navigation

| Section | Description |
|---------|-------------|
| [Tutorials]({{< ref "/workflows/tutorials" >}}) | Step-by-step learning guides |
| [How-to Guides]({{< ref "/workflows/how-to-guides" >}}) | Task-oriented recipes |
| [Reference]({{< ref "/workflows/reference" >}}) | API and configuration lookup |
| [Explanation]({{< ref "/workflows/explanation" >}}) | Architecture and design decisions |
