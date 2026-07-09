---
title: "Configure a Database Connection URL"
description: "How to configure SQLite and PostgreSQL connection URLs for a Cloaca (Python) runner"
weight: 20
aliases:
  - "/python/workflows/how-to-guides/backend-selection/"

---

# Configure a Database Connection URL

Cloaca selects its backend at runtime from the connection URL you pass to
`DefaultRunner`. This guide gives the concrete URL forms and tuning parameters.

> **Choosing a backend?** For the SQLite-vs-PostgreSQL comparison, isolation
> guarantees, and when each backend is appropriate, see
> [Database Backends]({{< ref "/service/explanation/database-backends" >}}).

## Configure a SQLite URL

```python
import cloaca

# File-based database
runner = cloaca.DefaultRunner("sqlite:///workflows.db")

# Custom path
runner = cloaca.DefaultRunner("sqlite:///./data/workflows.db")

# In-memory (testing only)
runner = cloaca.DefaultRunner("sqlite:///:memory:")
```

### Enable WAL mode and a busy timeout

For better concurrency and to avoid immediate "database is locked" failures,
enable WAL journaling and set a busy timeout:

```python
runner = cloaca.DefaultRunner(
    "sqlite:///workflows.db?"
    "mode=rwc&"
    "_journal_mode=WAL&"
    "_synchronous=NORMAL&"
    "_busy_timeout=5000"
)
```

- `_journal_mode=WAL` — allows concurrent readers while a write is in progress.
- `_synchronous=NORMAL` — balances durability against write throughput.
- `_busy_timeout=5000` — wait up to 5s for a lock instead of failing immediately.

## Configure a PostgreSQL URL

```python
import cloaca

# Basic connection
runner = cloaca.DefaultRunner("postgresql://user:password@localhost:5432/cloacina")

# Schema isolation (multi-tenant)
runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:password@localhost:5432/cloacina",
    "tenant_schema",
)
```

### Connection pooling, SSL, and timeouts

```python
# Connection pool sizing
runner = cloaca.DefaultRunner(
    "postgresql://user:password@localhost:5432/cloacina?"
    "pool_min_size=5&pool_max_size=20&pool_timeout=30"
)

# Require SSL
runner = cloaca.DefaultRunner(
    "postgresql://user:password@host:5432/db?sslmode=require"
)

# Connection timeout
runner = cloaca.DefaultRunner(
    "postgresql://user:password@host:5432/db?connect_timeout=10"
)
```

For connection-pool and runner sizing guidance, see
[Performance Optimization]({{< ref "/embed/how-to/performance-optimization/" >}}).

## Select the URL from the environment

A common pattern is to drive the URL from an environment variable so the same
code runs against SQLite locally and PostgreSQL in deployed environments:

```python
import os
import cloaca

def create_runner():
    env = os.getenv("ENVIRONMENT", "development")
    if env == "development":
        return cloaca.DefaultRunner("sqlite:///dev_workflows.db")
    if env == "testing":
        return cloaca.DefaultRunner("sqlite:///:memory:")

    database_url = os.getenv("DATABASE_URL")
    if not database_url:
        raise ValueError(f"DATABASE_URL required for {env} environment")
    return cloaca.DefaultRunner(database_url)
```

## See Also

- [Database Backends]({{< ref "/service/explanation/database-backends" >}}) - Choosing between SQLite and PostgreSQL
- [Quick Start Guide]({{< ref "/embed/quick-start" >}}) - Getting started with either backend
- [Multi-Tenancy Tutorial]({{< ref "/embed/tutorials/06-multi-tenancy/" >}}) - PostgreSQL multi-tenant setup
- [Performance Optimization]({{< ref "/embed/how-to/performance-optimization/" >}}) - Optimize your chosen backend
- [Configuration Reference]({{< ref "/reference/python-api/configuration/" >}}) - Complete configuration options
