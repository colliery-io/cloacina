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

Everything after the `sqlite://` prefix is used as the database file path (a
bare path with no prefix also works). The file is created if it does not exist.

```python
import cloaca

# File in the working directory
runner = cloaca.DefaultRunner("sqlite://workflows.db")

# Relative or absolute path
runner = cloaca.DefaultRunner("sqlite://./data/workflows.db")

# In-memory (testing only)
runner = cloaca.DefaultRunner("sqlite://:memory:")
```

`:memory:` is materialized as a per-runner temporary file so all pooled
connections share one database; it is deleted when the runner is dropped.

### WAL mode and busy timeout are automatic

Cloaca configures SQLite for concurrency itself — it sets
`PRAGMA journal_mode=WAL` and `PRAGMA busy_timeout=30000` on every pooled
connection. You do not need to (and cannot) tune these through URL query
parameters: Cloaca does not parse query parameters on SQLite URLs.

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

### SSL and timeouts

PostgreSQL URL query parameters are passed through to the PostgreSQL client
library (libpq), so its standard connection parameters work:

```python
# Require SSL
runner = cloaca.DefaultRunner(
    "postgresql://user:password@host:5432/db?sslmode=require"
)

# Connection timeout
runner = cloaca.DefaultRunner(
    "postgresql://user:password@host:5432/db?connect_timeout=10"
)
```

Connection-pool size is **not** a URL parameter — it is the `db_pool_size`
field on `DefaultRunnerConfig` (default 10), passed via
`DefaultRunner.with_config`. See
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
        return cloaca.DefaultRunner("sqlite://dev_workflows.db")
    if env == "testing":
        return cloaca.DefaultRunner("sqlite://:memory:")

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
