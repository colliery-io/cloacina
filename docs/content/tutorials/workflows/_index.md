---
title: "Workflow Tutorials"
description: "Learn Cloacina's workflow orchestration system — the unified scheduler for DAG-based task pipelines"
weight: 10
---

# Workflow Tutorials

Tutorials for Cloacina's **workflow** system — the unified scheduler that orchestrates DAG-based task pipelines with retries, dependencies, triggers, and cron scheduling.

## Library (Embedded)

Start here. These tutorials teach you to define and run workflows directly in your Rust application using the `#[workflow]` and `#[task]` macros.

{{< toc-tree >}}

## Service (Server Mode)

Once you're comfortable with the library, these tutorials cover packaging workflows for the API server, the reconciler, cron scheduling, event triggers, and multi-tenancy.
