---
title: "Python (Cloaca)"
description: "Python bindings for Cloacina — define and execute workflows and computation graphs from Python"
weight: 15
---

# Python (Cloaca)

**Cloaca** is the Python package that provides full access to Cloacina's workflow and computation graph engines. Built with PyO3, it offers native performance with Pythonic ergonomics.

## Installation

```bash
pip install cloaca              # Default (SQLite + PostgreSQL)
pip install cloaca[sqlite]      # SQLite only
pip install cloaca[postgres]    # PostgreSQL only
```

## Features

- **Workflow orchestration** — Define tasks with `@task`, build workflows with `WorkflowBuilder`, execute with `DefaultRunner`
- **Computation graphs** — Define reactive graphs with decorators, wire accumulators, push events
- **Multi-tenancy** — PostgreSQL schema isolation via `DatabaseAdmin`
- **Cron scheduling** — Time-based workflow execution
- **Event triggers** — Condition-based workflow firing

## Quick Navigation

| Section | Description |
|---------|-------------|
| [Quick Start]({{< ref "/python/quick-start" >}}) | Get running in 5 minutes |
| [Tutorials]({{< ref "/python/tutorials" >}}) | Progressive learning guides |
| [How-to Guides]({{< ref "/python/how-to-guides" >}}) | Recipes for specific tasks |
| [API Reference]({{< ref "/python/api-reference" >}}) | Complete class and function reference |
