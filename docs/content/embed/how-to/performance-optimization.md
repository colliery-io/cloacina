---
title: "Performance Optimization"
description: "Tune Cloaca connection pooling and runner sizing for production workloads"
weight: 30
aliases:
  - "/python/workflows/how-to-guides/performance-optimization/"

---

# Performance Optimization

This guide covers the concrete Cloacina-specific knobs you can turn to tune a
production runner: `DefaultRunnerConfig` sizing and connection parameters on the
database URL.

> **Why these knobs matter — and why workflow design matters more.** The largest
> performance lever is how you decompose work into tasks, structure
> dependencies, and size the context. Turn to
> [Workflow Performance and Design Trade-offs]({{< ref "/embed/explanation/performance" >}})
> for the rationale before reaching for the tunables below.

## Tune the connection pool

The runner's connection pool is sized by **`db_pool_size` on
`DefaultRunnerConfig`** (default 10) — not by URL query parameters. Cloacina
does not read `pool_*`-style parameters from the connection URL:

```python
import os
import cloaca

config = cloaca.DefaultRunnerConfig(
    db_pool_size=int(os.getenv("DB_POOL_SIZE", "20")),
)
runner = cloaca.DefaultRunner.with_config(os.environ["DATABASE_URL"], config)
```

PostgreSQL URL query parameters *are* passed through to the PostgreSQL client
library, so standard libpq connection parameters work on the URL — for example
`sslmode=require`, `connect_timeout=10`, or `application_name=cloacina_prod`:

```python
runner = cloaca.DefaultRunner.with_config(
    "postgresql://user:pass@host:5432/cloacina?"
    "sslmode=require&connect_timeout=10&application_name=cloacina_prod",
    config,
)
```

Note for multi-tenant deployments: `DefaultRunner.with_schema(url, schema)` does
not currently take a config, so each schema-scoped runner uses the default pool
size of 10. Budget your database's `max_connections` for ~10 connections per
tenant runner.

## Size the runner with DefaultRunnerConfig

`DefaultRunnerConfig` controls concurrency, timeouts, and pool size at the runner
level. Pass it via `with_config`:

```python
import cloaca

config = cloaca.DefaultRunnerConfig()
config.max_concurrent_tasks = 16        # parallel task executions
config.db_pool_size = 20                # runner-side connection pool
config.task_timeout_seconds = 1800      # 30 min per task
config.workflow_timeout_seconds = 7200  # 2 hr per workflow

runner = cloaca.DefaultRunner.with_config(database_url, config)
```

- **`max_concurrent_tasks`** — how many tasks execute simultaneously. Raise it
  for CPU- or I/O-bound workloads that can absorb the parallelism; keep it in line
  with `db_pool_size` so tasks aren't starved waiting on connections.
- **`db_pool_size`** — runner-side connection pool. Should be at least
  `max_concurrent_tasks` for high-concurrency PostgreSQL workloads.
- **`task_timeout_seconds`** / **`workflow_timeout_seconds`** — bound how long a
  single task or an entire workflow may run before it is considered timed out.

See the [Configuration Reference]({{< ref "/reference/python-api/configuration/" >}})
for the full list of fields and defaults.

## See Also

- [Workflow Performance and Design Trade-offs]({{< ref "/embed/explanation/performance" >}}) - Why granularity, parallelism, and context size dominate performance
- [Configure a Database Connection URL]({{< ref "/embed/how-to/backend-selection/" >}}) - SQLite and PostgreSQL URL parameters
- [Configuration Reference]({{< ref "/reference/python-api/configuration/" >}}) - Every configuration field
- [Multi-Tenancy Tutorial]({{< ref "/embed/tutorials/06-multi-tenancy/" >}}) - Multi-tenant performance considerations
