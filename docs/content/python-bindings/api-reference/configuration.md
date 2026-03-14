---
title: "Configuration"
description: "Configuration options for Cloaca components"
weight: 70
---

# Configuration

Configuration classes and options for customizing Cloaca behavior, including runner settings, database connections, and execution parameters.

## DefaultRunnerConfig

Configuration for the DefaultRunner with database and execution settings.

### Constructor

```python
import cloaca

# Default configuration
config = cloaca.DefaultRunnerConfig()

# Custom configuration via constructor keyword arguments
config = cloaca.DefaultRunnerConfig(
    max_concurrent_tasks=8,
    task_timeout_seconds=600,
    db_pool_size=20
)

# Properties are also settable after construction
config.max_concurrent_tasks = 16
config.enable_cron_scheduling = False
```

### Parameters

All parameters are optional keyword arguments with sensible defaults:

- `max_concurrent_tasks` (int, default=4): Maximum number of tasks executing simultaneously
- `scheduler_poll_interval_ms` (int, default=100): How often the task scheduler polls for ready tasks (ms)
- `task_timeout_seconds` (int, default=300): Maximum time a single task can run (seconds)
- `pipeline_timeout_seconds` (int, default=3600): Maximum time an entire workflow can run (seconds)
- `db_pool_size` (int, default=10): Database connection pool size
- `enable_recovery` (bool, default=True): Enable automatic recovery of orphaned tasks
- `enable_cron_scheduling` (bool, default=True): Enable cron scheduling subsystem
- `cron_poll_interval_seconds` (int, default=30): How often to check for due cron schedules
- `cron_max_catchup_executions` (int, default=unlimited): Maximum missed cron executions to catch up
- `cron_enable_recovery` (bool, default=True): Enable cron execution recovery
- `cron_recovery_interval_seconds` (int, default=300): How often to check for lost cron executions
- `cron_lost_threshold_minutes` (int, default=10): Minutes before a cron execution is considered lost
- `cron_max_recovery_age_seconds` (int, default=86400): Maximum age of recoverable cron executions
- `cron_max_recovery_attempts` (int, default=3): Maximum recovery attempts per execution

### Methods

- `default()` — Create a config with default values (same as `DefaultRunnerConfig()`)
- `to_dict()` — Return all settings as a dictionary

### Example Usage

```python
import cloaca

# Create custom configuration
config = cloaca.DefaultRunnerConfig(
    max_concurrent_tasks=16,
    task_timeout_seconds=600,      # 10 minutes
    pipeline_timeout_seconds=7200, # 2 hours
    db_pool_size=20
)

# Create runner with configuration
runner = cloaca.DefaultRunner.with_config(
    "sqlite:///workflows.db",
    config
)
```

## Database Configuration

### SQLite Configuration

```python
# In-memory database (for testing)
runner = cloaca.DefaultRunner("sqlite:///:memory:")

# File-based SQLite
runner = cloaca.DefaultRunner("sqlite:///path/to/database.db")

# SQLite with connection parameters
runner = cloaca.DefaultRunner(
    "sqlite:///workflows.db?timeout=20&journal_mode=WAL"
)
```

### PostgreSQL Configuration

```python
# Basic PostgreSQL connection
runner = cloaca.DefaultRunner(
    "postgresql://username:password@localhost:5432/database"
)

# PostgreSQL with custom configuration
config = cloaca.DefaultRunnerConfig(
    db_pool_size=20,
    max_concurrent_tasks=15
)

runner = cloaca.DefaultRunner.with_config(
    "postgresql://user:pass@localhost:5432/cloaca",
    config
)
```

## Task Configuration

### Retry Configuration

Configure retry behavior for individual tasks:

```python
@cloaca.task(
    id="resilient_task",
    retry_attempts=5,
    retry_delay_ms=10000,
    retry_backoff="exponential",
    retry_max_delay_ms=300000,
    retry_condition="all",
    retry_jitter=True
)
def resilient_task(context):
    """Task with custom retry configuration."""
    # Task implementation
    return context
```

### Timeout Configuration

Set task-specific timeouts:

```python
@cloaca.task(
    id="long_running_task",
    timeout_seconds=1800  # 30 minutes
)
def long_running_task(context):
    """Task that may take a long time."""
    # Long-running operation
    return context
```

## Cron Configuration

### Cron Schedule Registration

Cron schedules are registered through the `DefaultRunner` rather than a standalone class:

```python
import cloaca

runner = cloaca.DefaultRunner("sqlite:///app.db")

# Register a cron schedule
schedule_id = runner.register_cron_workflow(
    "daily_report",       # workflow name
    "0 9 * * *",          # cron expression (daily at 9 AM)
    "UTC"                 # timezone
)

# Manage schedules
runner.set_cron_schedule_enabled(schedule_id, False)   # disable
runner.update_cron_schedule(schedule_id, "0 10 * * *", "UTC")  # change time
runner.delete_cron_schedule(schedule_id)               # remove
```

See the [DefaultRunner API reference]({{< ref "/python-bindings/api-reference/runner/" >}}) for full cron scheduling methods.

## Multi-Tenant Configuration

### TenantConfig

Configuration for tenant provisioning:

```python
import cloaca

# Basic tenant configuration
tenant_config = cloaca.TenantConfig(
    schema_name="tenant_acme",
    username="acme_user",
    password=""  # Auto-generate secure password
)

# Explicit password
tenant_config = cloaca.TenantConfig(
    schema_name="tenant_globex",
    username="globex_user",
    password="secure_password_123"
)
```

### DatabaseAdmin Configuration

```python
# Admin with default settings
admin = cloaca.DatabaseAdmin("postgresql://admin:pass@localhost/db")

# Admin with custom configuration
admin_config = {
    "connection_timeout": 30,
    "command_timeout": 60,
    "enable_ssl": True
}

admin = cloaca.DatabaseAdmin(
    "postgresql://admin:pass@localhost/db",
    **admin_config
)
```

## Environment Configuration

### Environment Variables

Cloaca respects several environment variables:

```bash
# Database configuration
export CLOACA_DATABASE_URL="postgresql://user:pass@localhost/cloaca"
export CLOACA_LOG_LEVEL="DEBUG"
export CLOACA_MAX_WORKERS="10"

# Multi-tenant configuration
export CLOACA_ADMIN_DATABASE_URL="postgresql://admin:pass@localhost/cloaca"
export CLOACA_DEFAULT_SCHEMA="public"

# Performance tuning
export CLOACA_CONNECTION_POOL_SIZE="20"
export CLOACA_TASK_TIMEOUT="3600"
```

### Configuration from Environment

```python
import os
import cloaca

# Use environment variables
database_url = os.environ.get(
    "CLOACA_DATABASE_URL",
    "sqlite:///default.db"
)

log_level = os.environ.get("CLOACA_LOG_LEVEL", "INFO")
max_workers = int(os.environ.get("CLOACA_MAX_WORKERS", "5"))

config = cloaca.DefaultRunnerConfig(
    max_concurrent_tasks=max_workers
)

runner = cloaca.DefaultRunner(database_url, config)
```

## Production Configuration

### Recommended Production Settings

```python
import cloaca

# Production configuration
production_config = cloaca.DefaultRunnerConfig(
    max_concurrent_tasks=50,
    task_timeout_seconds=1800,   # 30 minutes
    pipeline_timeout_seconds=7200,  # 2 hours
    db_pool_size=25,
    enable_recovery=True,
    enable_cron_scheduling=True
)

# Production database with connection pooling
database_url = "postgresql://cloaca:secure_password@db.example.com:5432/cloaca_prod?sslmode=require"

runner = cloaca.DefaultRunner(database_url, production_config)
```

### Health Check Configuration

```python
# Configure health monitoring
@cloaca.task(id="health_check")
def health_check(context):
    """Monitor system health."""
    import psutil

    health_data = {
        "cpu_percent": psutil.cpu_percent(),
        "memory_percent": psutil.virtual_memory().percent,
        "disk_percent": psutil.disk_usage('/').percent
    }

    context.set("health_metrics", health_data)
    return context
```

## Configuration Validation

Validate configuration before use:

```python
def validate_config(config):
    """Validate runner configuration."""
    if config.max_concurrent_tasks < 1:
        raise ValueError("max_concurrent_tasks must be positive")

    if config.task_timeout_seconds < 1:
        raise ValueError("task_timeout_seconds must be positive")

    if config.db_pool_size < 1:
        raise ValueError("db_pool_size must be positive")

    return True

# Usage
config = cloaca.DefaultRunnerConfig(max_concurrent_tasks=10)
validate_config(config)
```

## See Also

- **[DefaultRunner]({{< ref "/python-bindings/api-reference/runner/" >}})** - Execute workflows with configuration
- **[DatabaseAdmin]({{< ref "/python-bindings/api-reference/database-admin/" >}})** - Multi-tenant configuration
- **[Task Decorator]({{< ref "/python-bindings/api-reference/task/" >}})** - Task-level configuration
