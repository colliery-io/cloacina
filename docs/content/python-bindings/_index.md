---
title: "Cloaca"
description: "Cloaca - Python bindings for Cloacina workflow orchestration"
weight: 25
reviewer: "automation"
review_date: "2025-01-07"
---

# Cloaca

Welcome to the Cloaca documentation. Cloaca is the Python bindings library for Cloacina's workflow execution engine, providing a Pythonic interface that allows you to build resilient task pipelines directly within your Python applications.

## What is Cloaca?

Cloaca provides Python bindings for Cloacina's workflow execution engine:

- **Pythonic API** using decorators and familiar patterns
- **Multiple database backends** (SQLite and PostgreSQL)
- **Context management** for data flow between tasks
- **Error handling** with Python exceptions
- **Workflow building** with builder pattern
- **Multi-tenancy** support via PostgreSQL schemas

## Key Features

- **Python decorators** for task definition
- **Builder pattern** for workflow construction
- **Context objects** for data flow between tasks
- **SQLite and PostgreSQL** backend support
- **Multi-tenant** PostgreSQL schema isolation
- **Error handling** with retry mechanisms
- **Cron scheduling** for periodic execution

## Feature Comparison

| Feature | Rust Native | Cloaca | Notes |
|---------|-------------|--------|-------|
| Task Definition | ✅ | ✅ | Decorators vs macros |
| Workflow Builder | ✅ | ✅ | Builder pattern |
| Context Management | ✅ | ✅ | Dict-like interface |
| Error Handling | ✅ | ✅ | Python exceptions |
| SQLite Backend | ✅ | ✅ | Same database schema |
| PostgreSQL Backend | ✅ | ✅ | Same database schema |
| Multi-tenancy | ✅ | ✅ | PostgreSQL schema isolation |
| Cron Scheduling | ✅ | ✅ | Expression parsing |
| Recovery Mechanisms | ✅ | ✅ | Configurable retry logic |

## Database Backend Selection

Choose the right backend for your use case:

{{< tabs "database-backends" >}}
{{< tab "SQLite" >}}
**Best for:**
- Development and testing
- Single-machine deployments
- Low-complexity applications
- Embedded applications

**Package:** `cloaca[sqlite]`

```bash
pip install cloaca[sqlite]
```
{{< /tab >}}

{{< tab "PostgreSQL" >}}
**Best for:**
- Production deployments
- Multi-tenant applications
- High-concurrency workloads
- Distributed systems

**Package:** `cloaca[postgres]`

```bash
pip install cloaca[postgres]
```
{{< /tab >}}
{{< /tabs >}}

## Getting Started

Ready to build your first workflow?

{{< button href="/python-bindings/quick-start/" >}}Quick Start{{< /button >}}
{{< button href="/python-bindings/tutorials/" >}}Tutorials{{< /button >}}

## Documentation Structure

This documentation is organized following the [Diátaxis framework](https://diataxis.fr/):

- **[Quick Start](/python-bindings/quick-start/)**: Run your first workflow in 5 minutes
- **[Tutorials](/python-bindings/tutorials/)**: Step-by-step learning guides
- **[API Reference](/python-bindings/api-reference/)**: Complete technical reference

## Migration from Rust

Python and Rust implementations can coexist:

- **Shared database**: Both can use the same PostgreSQL/SQLite database
- **Gradual adoption**: Migrate workflows incrementally
- **Same concepts**: Tasks, workflows, context, and runners work similarly

## Community and Support

- **Rust documentation**: [Core concepts](/tutorials/) apply to both implementations
- **Python examples**: Working code in the [examples/](https://github.com/colliery-io/cloacina/tree/main/examples) directory

---

{{< hint type="tip" title="Start Here" >}}
New to Cloacina? Begin with the **[Quick Start Guide](/python-bindings/quick-start/)** to run your first workflow in minutes.

Experienced with the Rust version? The same concepts apply - just with Python syntax instead of Rust macros.
{{< /hint >}}
