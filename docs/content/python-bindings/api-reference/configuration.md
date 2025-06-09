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

# Custom configuration
config = cloaca.DefaultRunnerConfig(
    max_concurrent_workflows=10,
    task_timeout_seconds=300,
    retry_attempts=3,
    connection_pool_size=5
)
```

### Parameters

- `max_concurrent_workflows` (int, default=5): Maximum number of workflows executing simultaneously
- `task_timeout_seconds` (int, default=3600): Default timeout for task execution in seconds
- `retry_attempts` (int, default=2): Default number of retry attempts for failed tasks
- `connection_pool_size` (int, default=10): Database connection pool size
- `enable_logging` (bool, default=True): Enable detailed execution logging
- `log_level` (str, default="INFO"): Logging level (DEBUG, INFO, WARNING, ERROR)

### Example Usage

```python
import cloaca

# Create custom configuration
config = cloaca.DefaultRunnerConfig(
    max_concurrent_workflows=20,
    task_timeout_seconds=600,  # 10 minutes
    retry_attempts=5,
    connection_pool_size=15,
    enable_logging=True,
    log_level="DEBUG"
)

# Create runner with configuration
runner = cloaca.DefaultRunner(
    database_url="sqlite:///workflows.db",
    config=config
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
    connection_pool_size=20,
    max_concurrent_workflows=15
)

runner = cloaca.DefaultRunner(
    "postgresql://user:pass@localhost:5432/cloaca",
    config=config
)
```

## Task Configuration

### Retry Configuration

Configure retry behavior for individual tasks:

```python
@cloaca.task(
    id="resilient_task",
    retry_policy={
        "max_attempts": 5,
        "retry_delay_seconds": 10,
        "exponential_backoff": True,
        "max_delay_seconds": 300
    }
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

### CronSchedule Configuration

```python
import cloaca

# Basic cron schedule
schedule = cloaca.CronSchedule(
    workflow_name="daily_report",
    cron_expression="0 9 * * *",  # Daily at 9 AM
    timezone="UTC",
    enabled=True
)

# Advanced cron configuration
schedule = cloaca.CronSchedule(
    workflow_name="complex_workflow",
    cron_expression="*/15 * * * *",  # Every 15 minutes
    timezone="America/New_York",
    enabled=True,
    context=cloaca.Context({"priority": "high"}),
    max_missed_runs=3,
    catch_up=False
)
```

### Cron Parameters

- `workflow_name` (str): Name of the workflow to execute
- `cron_expression` (str): Standard cron expression
- `timezone` (str): Timezone for schedule evaluation
- `enabled` (bool): Whether the schedule is active
- `context` (Context): Initial context for scheduled executions
- `max_missed_runs` (int): Maximum number of missed runs to catch up
- `catch_up` (bool): Whether to execute missed runs on startup

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
    max_concurrent_workflows=max_workers,
    log_level=log_level
)

runner = cloaca.DefaultRunner(database_url, config)
```

## Production Configuration

### Recommended Production Settings

```python
import cloaca

# Production configuration
production_config = cloaca.DefaultRunnerConfig(
    max_concurrent_workflows=50,
    task_timeout_seconds=1800,  # 30 minutes
    retry_attempts=3,
    connection_pool_size=25,
    enable_logging=True,
    log_level="INFO"
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
    if config.max_concurrent_workflows < 1:
        raise ValueError("max_concurrent_workflows must be positive")

    if config.task_timeout_seconds < 1:
        raise ValueError("task_timeout_seconds must be positive")

    if config.connection_pool_size < 1:
        raise ValueError("connection_pool_size must be positive")

    return True

# Usage
config = cloaca.DefaultRunnerConfig(max_concurrent_workflows=10)
validate_config(config)
```

## See Also

- **[DefaultRunner]({{< ref "/python-bindings/api-reference/runner/" >}})** - Execute workflows with configuration
- **[DatabaseAdmin]({{< ref "/python-bindings/api-reference/database-admin/" >}})** - Multi-tenant configuration
- **[Task Decorator]({{< ref "/python-bindings/api-reference/task/" >}})** - Task-level configuration
