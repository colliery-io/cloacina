---
title: "Performance Optimization"
description: "Tune Cloaca connection pooling and runner sizing for production workloads"
weight: 30
aliases:
  - "/python/workflows/how-to-guides/performance-optimization/"

---

# Performance Optimization

This guide covers the concrete Cloacina-specific knobs you can turn to tune a
production runner: connection-pool parameters on the database URL and
`DefaultRunnerConfig` sizing.

> **Why these knobs matter — and why workflow design matters more.** The largest
> performance lever is how you decompose work into tasks, structure
> dependencies, and size the context. Turn to
> [Workflow Performance and Design Trade-offs]({{< ref "/embed/explanation/performance" >}})
> for the rationale before reaching for the tunables below.

## Tune the connection pool

Connection-pool parameters are passed as query parameters on a PostgreSQL URL.
Size the pool to your concurrency and set timeouts so connections don't hang:

```python
import cloaca

runner = cloaca.DefaultRunner(
    "postgresql://user:pass@host:5432/cloacina?"
    "pool_min_size=5&"      # minimum idle connections
    "pool_max_size=20&"     # maximum connections
    "pool_timeout=30&"      # seconds to wait for a free connection
    "pool_recycle=3600"     # recycle connections after 1 hour
)
```

For a production runner, drive these from the environment so they can be tuned
per-deployment without code changes:

```python
import os
import cloaca

def create_runner():
    base_url = os.getenv("DATABASE_URL")
    if not base_url:
        raise ValueError("DATABASE_URL environment variable required")

    params = {
        "pool_min_size": os.getenv("DB_POOL_MIN_SIZE", "10"),
        "pool_max_size": os.getenv("DB_POOL_MAX_SIZE", "50"),
        "pool_timeout": os.getenv("DB_POOL_TIMEOUT", "30"),
        "pool_recycle": os.getenv("DB_POOL_RECYCLE", "7200"),
        "connect_timeout": os.getenv("DB_CONNECT_TIMEOUT", "10"),
        "application_name": os.getenv("APP_NAME", "cloacina_prod"),
    }
    param_string = "&".join(f"{k}={v}" for k, v in params.items())
    separator = "&" if "?" in base_url else "?"
    return cloaca.DefaultRunner(f"{base_url}{separator}{param_string}")
```

In multi-tenant deployments, cap the per-tenant pool so a single tenant cannot
exhaust the database's connection budget:

```python
tenant_url = f"{base_url}?pool_max_size=10"
runner = cloaca.DefaultRunner.with_schema(tenant_url, tenant_id)
```

## Size the runner with DefaultRunnerConfig

`DefaultRunnerConfig` controls concurrency, timeouts, and pool size at the runner
level. Pass it via `with_config`:

```python
import cloaca

config = cloaca.DefaultRunnerConfig()
config.max_concurrent_tasks = 16        # parallel task executions
config.db_pool_size = 20                # runner-side connection pool
config.task_timeout_seconds = 1800      # 30 min per task
config.pipeline_timeout_seconds = 7200  # 2 hr per workflow

runner = cloaca.DefaultRunner.with_config(database_url, config)
```

- **`max_concurrent_tasks`** — how many tasks execute simultaneously. Raise it
  for CPU- or I/O-bound workloads that can absorb the parallelism; keep it in line
  with `db_pool_size` so tasks aren't starved waiting on connections.
- **`db_pool_size`** — runner-side connection pool. Should be at least
  `max_concurrent_tasks` for high-concurrency PostgreSQL workloads.
- **`task_timeout_seconds`** / **`pipeline_timeout_seconds`** — bound how long a
  single task or an entire workflow may run before it is considered timed out.

See the [Configuration Reference]({{< ref "/reference/python-api/configuration/" >}})
for the full list of fields and defaults.

## See Also

- [Workflow Performance and Design Trade-offs]({{< ref "/embed/explanation/performance" >}}) - Why granularity, parallelism, and context size dominate performance
- [Configure a Database Connection URL]({{< ref "/embed/how-to/backend-selection/" >}}) - SQLite and PostgreSQL URL parameters
- [Configuration Reference]({{< ref "/reference/python-api/configuration/" >}}) - Every configuration field
- [Multi-Tenancy Tutorial]({{< ref "/embed/tutorials/06-multi-tenancy/" >}}) - Multi-tenant performance considerations
