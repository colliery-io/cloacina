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

Cloaca wraps Cloacina's Rust core, providing:

- **Full feature parity** with the Rust implementation
- **Pythonic API design** using decorators and familiar patterns
- **Type safety** with comprehensive type hints
- **Multiple database backends** (SQLite and PostgreSQL)
- **Async/await support** for modern Python applications
- **Memory safety** through Rust's underlying guarantees

## Why Use Cloaca?

{{< tabs "binding-benefits" >}}
{{< tab "Python Developers" >}}
- **Familiar syntax**: Use Python decorators, context managers, and async/await
- **Rich ecosystem**: Integrate with NumPy, Pandas, FastAPI, Django, and more
- **Rapid development**: Python's expressiveness for quick prototyping
- **Team skills**: Leverage existing Python expertise in your organization
{{< /tab >}}

{{< tab "Rust Performance" >}}
- **Native speed**: Core execution engine runs at Rust performance
- **Memory efficiency**: Rust's zero-cost abstractions and memory safety
- **Concurrent execution**: Built on Tokio for high-performance async operations
- **Database optimization**: Optimized SQL generation and connection pooling
{{< /tab >}}

{{< tab "Production Ready" >}}
- **Battle tested**: Same core engine powering Rust applications
- **Comprehensive error handling**: Detailed exception hierarchy and recovery strategies
- **Monitoring**: Built-in execution tracking and observability
- **Multi-tenancy**: Enterprise-ready isolation and security features
{{< /tab >}}
{{< /tabs >}}

## Feature Comparison

| Feature | Rust Native | Cloaca | Notes |
|---------|-------------|-----------------|-------|
| Task Definition | ✅ | ✅ | Decorators vs macros |
| Workflow Builder | ✅ | ✅ | Pythonic builder pattern |
| Context Management | ✅ | ✅ | Dict-like interface |
| Error Handling | ✅ | ✅ | Python exceptions |
| Async Support | ✅ | ✅ | Native async/await |
| SQLite Backend | ✅ | ✅ | Full parity |
| PostgreSQL Backend | ✅ | ✅ | Full parity |
| Multi-tenancy | ✅ | ✅ | Schema isolation |
| Cron Scheduling | ✅ | ✅ | Expression parsing |
| Recovery Mechanisms | ✅ | ✅ | Automatic retry logic |

## Database Backend Selection

Choose the right backend for your use case:

{{< tabs "database-backends" >}}
{{< tab "SQLite" >}}
**Best for:**
- Development and testing
- Single-machine deployments
- Low-complexity applications
- Embedded applications

**Package:** `cloaca-sqlite`

```bash
pip install cloaca-sqlite
```
{{< /tab >}}

{{< tab "PostgreSQL" >}}
**Best for:**
- Production deployments
- Multi-tenant applications
- High-concurrency workloads
- Distributed systems

**Package:** `cloaca-postgres`

```bash
pip install cloaca-postgres
```
{{< /tab >}}
{{< /tabs >}}

## Getting Started

Ready to build your first workflow? Choose your path:

{{< button href="/python-bindings/installation/" >}}📦 Installation Guide{{< /button >}}
{{< button href="/python-bindings/quick-start/" >}}🚀 Quick Start{{< /button >}}
{{< button href="/python-bindings/tutorials/" >}}📚 Tutorials{{< /button >}}

## Documentation Structure

This documentation is organized following the [Diátaxis framework](https://diataxis.fr/):

- **[Installation](/python-bindings/installation/)**: Set up Python bindings in your environment
- **[Quick Start](/python-bindings/quick-start/)**: Run your first workflow in 5 minutes
- **[Tutorials](/python-bindings/tutorials/)**: Step-by-step learning guides
- **[How-to Guides](/python-bindings/how-to-guides/)**: Solutions to specific problems
- **[API Reference](/python-bindings/api-reference/)**: Complete technical reference
- **[Examples](/python-bindings/examples/)**: Real-world code examples

## Migration from Rust

Already using Cloacina in Rust? Cloaca provides a smooth migration path:

- **Gradual adoption**: Migrate workflows incrementally
- **Shared database**: Python and Rust can share the same database
- **Concept mapping**: Direct equivalents for all Rust patterns
- **Performance retention**: Keep the same execution characteristics

{{< button href="/python-bindings/how-to-guides/migration-from-rust/" >}}📋 Migration Guide{{< /button >}}

## Community and Support

- **GitHub**: [Issues and discussions](https://github.com/username/cloacina)
- **Documentation**: [Rust documentation](/tutorials/) for core concepts
- **Examples**: Working code in the [examples repository](https://github.com/username/cloacina-examples)

---

{{< hint type="tip" title="Start Here" >}}
New to Cloacina? Begin with the **[Quick Start Guide](/python-bindings/quick-start/)** to run your first workflow in minutes.

Experienced with the Rust version? Jump to the **[Migration Guide](/python-bindings/how-to-guides/migration-from-rust/)** for API comparisons and translation patterns.
{{< /hint >}}