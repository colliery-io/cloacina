---
title: "Configuration"
description: "Configuration surface for Cloaca: DefaultRunnerConfig, runner construction, retry, cron, multi-tenancy, and environment variables"
weight: 70
---

# Configuration

This page describes the configuration surface exposed by the `cloaca` package:
the `DefaultRunnerConfig` class and its fields, how a runner is constructed with
a configuration, task-level retry options, cron-schedule registration, the
multi-tenant admin types, and the environment variables Cloaca reads. Every
field, default, and signature below is taken from the binding source.

## DefaultRunnerConfig

Holds the runner's execution and scheduling settings. The constructor accepts
every field as an optional keyword argument; any argument left unset (`None`)
takes the engine default shown in the table.

### Constructor

```python
import cloaca

# Engine defaults
config = cloaca.DefaultRunnerConfig()

# Equivalent explicit form
config = cloaca.DefaultRunnerConfig.default()

# Override selected fields
config = cloaca.DefaultRunnerConfig(
    max_concurrent_tasks=8,
    task_timeout_seconds=600,
    db_pool_size=20,
)
```

### Fields

| Field | Type | Default | Meaning |
|-------|------|---------|---------|
| `max_concurrent_tasks` | int | `4` | Maximum tasks executing simultaneously |
| `scheduler_poll_interval_ms` | int | `100` | Scheduler polling interval, in milliseconds |
| `task_timeout_seconds` | int | `300` | Per-task execution timeout, in seconds |
| `workflow_timeout_seconds` | int | `3600` | Per-workflow execution timeout, in seconds |
| `db_pool_size` | int | `10` | Database connection pool size |
| `enable_recovery` | bool | `True` | Recover in-flight work after a restart |
| `enable_cron_scheduling` | bool | `True` | Run the cron scheduler loop |
| `cron_poll_interval_seconds` | int | `30` | Cron scheduler polling interval, in seconds |
| `cron_max_catchup_executions` | int | `100` | Maximum missed runs replayed per schedule |
| `cron_enable_recovery` | bool | `True` | Recover lost cron executions |
| `cron_recovery_interval_seconds` | int | `300` | Cron recovery sweep interval, in seconds |
| `cron_lost_threshold_minutes` | int | `10` | Age after which a cron execution is considered lost |
| `cron_max_recovery_age_seconds` | int | `86400` | Oldest cron execution still eligible for recovery |
| `cron_max_recovery_attempts` | int | `3` | Maximum recovery attempts per cron execution |

Time fields are plain integers in the unit named by the field (seconds or
milliseconds); there is no `timedelta`/`Duration` surface.

### Methods

- `default()` (static) — returns a `DefaultRunnerConfig` with all engine defaults.
- `to_dict()` — returns the configuration as a `dict`.

Each field also has a matching read/write property.

## Constructing a runner with a configuration

The `DefaultRunner` constructor takes only a database URL. There is **no**
`config=` keyword argument on the constructor. To supply a configuration, use
the `with_config` static method.

```python
import cloaca

# Default configuration
runner = cloaca.DefaultRunner("sqlite:///workflows.db")

# Custom configuration
config = cloaca.DefaultRunnerConfig(max_concurrent_tasks=8, db_pool_size=20)
runner = cloaca.DefaultRunner.with_config("sqlite:///workflows.db", config)
```

### with_schema (PostgreSQL multi-tenancy)

`with_schema` pins the runner to a single PostgreSQL schema (for schema-isolated
multi-tenancy). It accepts only `postgresql://` / `postgres://` URLs and rejects
empty or non-alphanumeric schema names.

```python
runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:pass@localhost:5432/cloaca",
    "tenant_acme",
)
```

### Database URLs

The URL determines the backend; connection parameters are passed in the URL
string itself.

```python
# SQLite — in-memory (testing)
cloaca.DefaultRunner("sqlite:///:memory:")

# SQLite — file-backed, with SQLite URL parameters
cloaca.DefaultRunner("sqlite:///workflows.db?journal_mode=WAL")

# PostgreSQL
cloaca.DefaultRunner("postgresql://user:pass@localhost:5432/cloaca")
```

`DefaultRunner` is also a context manager; exiting the `with` block calls
`shutdown()`.

```python
with cloaca.DefaultRunner("sqlite:///workflows.db") as runner:
    result = runner.execute("my_workflow", cloaca.Context())
```

## Task retry configuration

Retry behaviour is configured with discrete keyword arguments on the
`@cloaca.task` decorator. There is no `retry_policy` dict argument and no
`timeout_seconds` argument on the decorator (task timeout is the runner-level
`task_timeout_seconds` above).

```python
@cloaca.task(
    id="resilient_task",
    retry_attempts=5,
    retry_backoff="exponential",   # "fixed" | "linear" | "exponential"
    retry_delay_ms=1000,
    retry_max_delay_ms=60000,
    retry_condition="transient",   # "never" | "transient" | "all"
    retry_jitter=True,
)
def resilient_task(context):
    return context
```

| Argument | Type | Meaning |
|----------|------|---------|
| `retry_attempts` | int | Number of retry attempts after the first failure |
| `retry_backoff` | str | `"fixed"`, `"linear"`, or `"exponential"` |
| `retry_delay_ms` | int | Base delay between attempts, in milliseconds |
| `retry_max_delay_ms` | int | Upper bound on backoff delay, in milliseconds |
| `retry_condition` | str | `"never"`, `"transient"` (retry only transient errors), or `"all"` |
| `retry_jitter` | bool | Add randomized jitter to backoff delays |

See the [Task Decorator]({{< ref "/reference/python-api/task/" >}}) reference for
the full argument list (including `on_success`, `on_failure`, `invokes`, and
`post_invocation`).

## Cron scheduling

There is no `CronSchedule` class. Cron schedules are registered and managed
through methods on a running `DefaultRunner`, and read operations return plain
dicts.

```python
runner = cloaca.DefaultRunner("sqlite:///workflows.db")

# Register — returns the schedule id (str)
schedule_id = runner.register_cron_workflow(
    "daily_report",      # workflow_name
    "0 9 * * *",         # cron_expression
    "UTC",               # timezone
)

# List active schedules (list[dict])
schedules = runner.list_cron_schedules(enabled_only=True)

# Enable / disable / update / delete
runner.set_cron_schedule_enabled(schedule_id, False)
runner.update_cron_schedule(schedule_id, "*/15 * * * *", "America/New_York")
runner.delete_cron_schedule(schedule_id)
```

Each schedule dict has the keys: `id`, `workflow_name`, `cron_expression`,
`timezone`, `enabled`, `catchup_policy`, `next_run_at`, `last_run_at`,
`created_at`, `updated_at`. Execution history and statistics are available via
`get_cron_execution_history(schedule_id, ...)` and
`get_cron_execution_stats(since)`. See the
[DefaultRunner]({{< ref "/reference/python-api/runner/" >}}) reference for the
full set of cron, trigger, and reactor-subscription (event-driven trigger)
methods.

## Multi-tenant configuration

The multi-tenant admin types require PostgreSQL support; they are not importable
from a SQLite-only wheel. Install with `pip install cloaca[postgres]` (see
[Installation]({{< ref "/start/install" >}})).

### TenantConfig

```python
import cloaca

# Auto-generate a secure password (password omitted)
config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
)

# Explicit password
config = cloaca.TenantConfig(
    schema_name="tenant_globex",
    username="globex_user",
    password="secure_password_123",
)
```

| Parameter | Type | Meaning |
|-----------|------|---------|
| `schema_name` | str | PostgreSQL schema to isolate the tenant in |
| `username` | str | Role to create for the tenant |
| `password` | str, optional | Tenant password; omitted/`None` means auto-generate |

`schema_name`, `username`, and `password` are exposed as read-only properties.

### DatabaseAdmin

The constructor takes only a PostgreSQL admin URL. It does **not** accept
`connection_timeout`, `command_timeout`, or `enable_ssl`; pass connection
options in the URL. Non-PostgreSQL URLs are rejected.

```python
admin = cloaca.DatabaseAdmin("postgresql://admin:pass@localhost:5432/cloaca")

# Provision a tenant — returns TenantCredentials
creds = admin.create_tenant(config)
print(creds.username, creds.connection_string)

# Decommission a tenant
admin.remove_tenant("tenant_acme", "acme_user")
```

`create_tenant(config)` returns a `TenantCredentials` object with the read-only
properties `username`, `password`, `schema_name`, and `connection_string`.
`remove_tenant(schema_name, username)` drops the tenant. See the
[DatabaseAdmin]({{< ref "/reference/python-api/database-admin/" >}}) reference
for the full multi-tenant flow.

## Environment variables

Cloaca reads the following environment variables. There are no `CLOACA_*`
variables.

| Variable | Read by | Purpose |
|----------|---------|---------|
| `CLOACINA_VAR_<NAME>` | `cloaca.var("<NAME>")`, `cloaca.var_or("<NAME>", default)` | User-defined configuration values resolved at runtime |
| `RUST_LOG` | runtime tracing initialization | Log filter (e.g. `info`, `debug`, `cloacina=debug`); defaults to `info` |

`cloaca.var(name)` reads the environment variable `CLOACINA_VAR_` + `name`
**verbatim** (no case conversion) and raises if it is unset;
`cloaca.var_or(name, default)` returns `default` when the variable is absent.
Pass the name in the exact case of the environment variable.

```python
import cloaca

# Reads CLOACINA_VAR_DATABASE_URL  (name passed in upper case, matching the env var)
database_url = cloaca.var_or("DATABASE_URL", "sqlite:///default.db")

runner = cloaca.DefaultRunner(database_url)
```

See the
[Environment Variables]({{< ref "/reference/python-environment-variables" >}})
reference for the full variable-registry conventions.

## See Also

- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** — execute workflows, cron, triggers, reactor subscriptions
- **[DatabaseAdmin]({{< ref "/reference/python-api/database-admin/" >}})** — multi-tenant provisioning
- **[Task Decorator]({{< ref "/reference/python-api/task/" >}})** — full task argument surface, including retry
