---
title: "DefaultRunner"
description: "DefaultRunner class for workflow execution"
weight: 20
reviewer: "automation"
review_date: "2025-01-07"
---

# DefaultRunner

The `DefaultRunner` class is the main execution engine for Cloaca workflows. It manages database connections, task scheduling, execution, and provides cron scheduling capabilities.

## Constructors

### `DefaultRunner(database_url)`

Create a runner with default configuration.

**Parameters:**
- `database_url` (str): Database connection string

**Example:**
```python
import cloaca

# SQLite
runner = cloaca.DefaultRunner("sqlite:///app.db")

# PostgreSQL
runner = cloaca.DefaultRunner("postgresql://user:pass@localhost:5432/dbname")
```

### `DefaultRunner.with_config(database_url, config)`

Create a runner with custom configuration.

**Parameters:**
- `database_url` (str): Database connection string
- `config` (DefaultRunnerConfig): Custom configuration object

**Returns:** DefaultRunner instance

**Example:**
```python
import cloaca

# Custom configuration
config = cloaca.DefaultRunnerConfig()
config.max_concurrent_tasks = 8
config.task_timeout_seconds = 600

runner = cloaca.DefaultRunner.with_config(
    "postgresql://user:pass@localhost:5432/dbname", 
    config
)
```

### `DefaultRunner.with_schema(database_url, schema)`

Create a runner with PostgreSQL schema isolation (multi-tenancy).

**Parameters:**
- `database_url` (str): PostgreSQL connection string
- `schema` (str): Schema name for tenant isolation

**Returns:** DefaultRunner instance

**Raises:** ValueError if schema name is invalid

**Example:**
```python
import cloaca

# Multi-tenant setup
tenant_runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:pass@localhost:5432/dbname",
    "tenant_acme"
)
```

**Schema Naming Rules:**
- Must start with a letter
- Can contain letters, numbers, and underscores
- Cannot be PostgreSQL reserved names
- Maximum 63 characters

## Workflow Execution

### `execute(workflow_name, context)`

Execute a workflow with the given context.

**Parameters:**
- `workflow_name` (str): Name of the registered workflow
- `context` (Context): Initial workflow context

**Returns:** PipelineResult with execution details

**Example:**
```python
import cloaca

runner = cloaca.DefaultRunner("sqlite:///app.db")

# Execute workflow
context = cloaca.Context({"user_id": 123})
result = runner.execute("my_workflow", context)

if result.status == "Completed":
    print("Success!")
    final_data = result.final_context.to_dict()
else:
    print(f"Failed: {result.error_message}")
```

## Cron Scheduling

### `register_cron_workflow(workflow_name, cron_expression, timezone)`

Register a workflow for cron-based scheduling.

**Parameters:**
- `workflow_name` (str): Name of the workflow to schedule
- `cron_expression` (str): Cron expression (e.g., "0 2 * * *")
- `timezone` (str): Timezone name (e.g., "UTC", "America/New_York")

**Returns:** str - Schedule ID (UUID)

**Example:**
```python
import cloaca

runner = cloaca.DefaultRunner("postgresql://user:pass@localhost/db")

# Schedule daily at 2 AM UTC
schedule_id = runner.register_cron_workflow(
    "daily_report",
    "0 2 * * *",
    "UTC"
)

print(f"Scheduled with ID: {schedule_id}")
```

**Cron Expression Format:**
```
┌───────────── minute (0 - 59)
│ ┌───────────── hour (0 - 23)
│ │ ┌───────────── day of month (1 - 31)
│ │ │ ┌───────────── month (1 - 12)
│ │ │ │ ┌───────────── day of week (0 - 6, Sunday = 0)
│ │ │ │ │
* * * * *
```

**Common Examples:**
- `"0 0 * * *"` - Daily at midnight
- `"0 2 * * 1"` - Weekly on Monday at 2 AM
- `"*/15 * * * *"` - Every 15 minutes
- `"0 9-17 * * 1-5"` - Hourly during business hours, weekdays only

### `list_cron_schedules(enabled_only=None, limit=None, offset=None)`

List cron schedules with optional filtering.

**Parameters:**
- `enabled_only` (bool, optional): Filter by enabled status
- `limit` (int, optional): Maximum number of results
- `offset` (int, optional): Number of results to skip

**Returns:** List[dict] - List of schedule dictionaries

**Example:**
```python
# List all schedules
schedules = runner.list_cron_schedules()

# List only enabled schedules
enabled_schedules = runner.list_cron_schedules(enabled_only=True)

# Paginated results
recent_schedules = runner.list_cron_schedules(limit=10, offset=0)

for schedule in schedules:
    print(f"Schedule: {schedule['workflow_name']} - {schedule['cron_expression']}")
```

### `get_cron_schedule(schedule_id)`

Get details of a specific cron schedule.

**Parameters:**
- `schedule_id` (str): Schedule UUID

**Returns:** dict - Schedule details

**Example:**
```python
schedule = runner.get_cron_schedule(schedule_id)

print(f"Workflow: {schedule['workflow_name']}")
print(f"Expression: {schedule['cron_expression']}")
print(f"Next run: {schedule['next_run_at']}")
print(f"Enabled: {schedule['enabled']}")
```

### `update_cron_schedule(schedule_id, cron_expression, timezone)`

Update an existing cron schedule.

**Parameters:**
- `schedule_id` (str): Schedule UUID
- `cron_expression` (str): New cron expression
- `timezone` (str): New timezone

**Example:**
```python
# Change to run at 3 AM instead of 2 AM
runner.update_cron_schedule(
    schedule_id,
    "0 3 * * *",
    "UTC"
)
```

### `set_cron_schedule_enabled(schedule_id, enabled)`

Enable or disable a cron schedule.

**Parameters:**
- `schedule_id` (str): Schedule UUID
- `enabled` (bool): Whether schedule should be enabled

**Example:**
```python
# Disable schedule
runner.set_cron_schedule_enabled(schedule_id, False)

# Re-enable schedule
runner.set_cron_schedule_enabled(schedule_id, True)
```

### `delete_cron_schedule(schedule_id)`

Delete a cron schedule permanently.

**Parameters:**
- `schedule_id` (str): Schedule UUID

**Example:**
```python
runner.delete_cron_schedule(schedule_id)
```

### `get_cron_execution_history(schedule_id, limit=None, offset=None)`

Get execution history for a cron schedule.

**Parameters:**
- `schedule_id` (str): Schedule UUID
- `limit` (int, optional): Maximum number of results
- `offset` (int, optional): Number of results to skip

**Returns:** List[dict] - List of execution records

**Example:**
```python
# Get recent executions
history = runner.get_cron_execution_history(schedule_id, limit=20)

for execution in history:
    print(f"Scheduled: {execution['scheduled_time']}")
    print(f"Claimed: {execution['claimed_at']}")
    print(f"Pipeline: {execution['pipeline_execution_id']}")
```

### `get_cron_execution_stats(since)`

Get execution statistics since a given timestamp.

**Parameters:**
- `since` (str): ISO 8601 timestamp to calculate stats from

**Returns:** dict - Execution statistics

**Example:**
```python
# Stats for last 24 hours
since = (datetime.now() - timedelta(days=1)).isoformat()
stats = runner.get_cron_execution_stats(since)

print(f"Total executions: {stats['total_executions']}")
print(f"Successful: {stats['successful_executions']}")
print(f"Success rate: {stats['success_rate']:.2%}")
```

## Lifecycle Management

### `shutdown()`

Shutdown the runner and cleanup resources.

**Example:**
```python
runner = cloaca.DefaultRunner("sqlite:///app.db")

try:
    # Use runner
    result = runner.execute("workflow", context)
finally:
    # Always shutdown
    runner.shutdown()
```

## Context Manager Support

DefaultRunner supports Python context manager protocol for automatic cleanup.

### `with DefaultRunner(...) as runner:`

**Example:**
```python
import cloaca

# Automatic cleanup
with cloaca.DefaultRunner("sqlite:///app.db") as runner:
    context = cloaca.Context({"key": "value"})
    result = runner.execute("my_workflow", context)
    
    if result.status == "Completed":
        print("Success!")
# runner.shutdown() called automatically
```

## Configuration

See [DefaultRunnerConfig](/python-bindings/api-reference/configuration/) for detailed configuration options.

**Key Configuration Options:**
- `max_concurrent_tasks`: Number of tasks that can run simultaneously
- `task_timeout_seconds`: Maximum time a task can run
- `db_pool_size`: Database connection pool size
- `enable_recovery`: Whether to recover orphaned workflows
- `enable_cron_scheduling`: Whether to enable cron scheduling

## Database URLs

### SQLite
```python
# File database
"sqlite:///path/to/database.db"

# In-memory database (testing only)
"sqlite:///:memory:"

# With options
"sqlite:///app.db?mode=rwc&_journal_mode=WAL"
```

### PostgreSQL
```python
# Basic connection
"postgresql://username:password@localhost:5432/database"

# With SSL and options
"postgresql://user:pass@host:5432/db?sslmode=require"

# Connection pooling (handled automatically)
"postgresql://user:pass@host:5432/db?application_name=my_app"
```

## Multi-Tenancy

DefaultRunner supports multi-tenant deployments using PostgreSQL schemas:

```python
# Each tenant gets isolated schema
tenant_a = cloaca.DefaultRunner.with_schema(database_url, "tenant_a")
tenant_b = cloaca.DefaultRunner.with_schema(database_url, "tenant_b")

# Complete data isolation
context_a = cloaca.Context({"tenant": "a"})
context_b = cloaca.Context({"tenant": "b"})

result_a = tenant_a.execute("workflow", context_a)
result_b = tenant_b.execute("workflow", context_b)

# No data cross-contamination possible
```

## Error Handling

DefaultRunner operations can raise various exceptions:

```python
import cloaca

try:
    runner = cloaca.DefaultRunner("invalid://connection/string")
except ValueError as e:
    print(f"Invalid database URL: {e}")

try:
    result = runner.execute("nonexistent_workflow", context)
except ValueError as e:
    print(f"Workflow not found: {e}")

try:
    runner.register_cron_workflow("workflow", "invalid cron", "UTC")
except ValueError as e:
    print(f"Invalid cron expression: {e}")
```

## Performance Tuning

{{< tabs "performance" >}}
{{< tab "Connection Pooling" >}}
```python
# Tune pool size based on workload
config = cloaca.DefaultRunnerConfig()
config.db_pool_size = 20  # For high-concurrency PostgreSQL

runner = cloaca.DefaultRunner.with_config(database_url, config)
```
{{< /tab >}}

{{< tab "Task Concurrency" >}}
```python
# Adjust concurrent task limit
config = cloaca.DefaultRunnerConfig()
config.max_concurrent_tasks = 16  # For CPU-intensive tasks

runner = cloaca.DefaultRunner.with_config(database_url, config)
```
{{< /tab >}}

{{< tab "Timeouts" >}}
```python
# Configure timeouts
config = cloaca.DefaultRunnerConfig()
config.task_timeout_seconds = 1800      # 30 minutes per task
config.pipeline_timeout_seconds = 7200  # 2 hours per workflow

runner = cloaca.DefaultRunner.with_config(database_url, config)
```
{{< /tab >}}

{{< tab "Cron Optimization" >}}
```python
# Tune cron polling
config = cloaca.DefaultRunnerConfig()
config.cron_poll_interval_seconds = 30     # Check every 30 seconds
config.cron_max_catchup_executions = 10    # Catch up to 10 missed runs

runner = cloaca.DefaultRunner.with_config(database_url, config)
```
{{< /tab >}}
{{< /tabs >}}

## Best Practices

### Resource Management
```python
# Always use context manager or explicit shutdown
with cloaca.DefaultRunner(database_url) as runner:
    # Workflow execution
    pass
# Automatic cleanup

# Or explicit cleanup
runner = cloaca.DefaultRunner(database_url)
try:
    # Workflow execution
    pass
finally:
    runner.shutdown()
```

### Error Handling
```python
def execute_workflow_safely(runner, workflow_name, context):
    """Execute workflow with comprehensive error handling."""
    try:
        result = runner.execute(workflow_name, context)
        
        if result.status == "Completed":
            return result.final_context
        else:
            print(f"Workflow failed: {result.error_message}")
            return None
            
    except Exception as e:
        print(f"Execution error: {e}")
        return None
```

### Monitoring
```python
def monitor_cron_schedules(runner):
    """Monitor cron schedule health."""
    schedules = runner.list_cron_schedules(enabled_only=True)
    
    for schedule in schedules:
        # Check recent execution history
        history = runner.get_cron_execution_history(
            schedule['id'], 
            limit=5
        )
        
        if not history:
            print(f"Warning: No recent executions for {schedule['workflow_name']}")
        
        # Check execution stats
        since = (datetime.now() - timedelta(days=1)).isoformat()
        stats = runner.get_cron_execution_stats(since)
        
        if stats['success_rate'] < 0.9:
            print(f"Warning: Low success rate: {stats['success_rate']:.2%}")
```

## Thread Safety

DefaultRunner is thread-safe and can be shared across multiple threads:

```python
import threading
import cloaca

runner = cloaca.DefaultRunner("postgresql://user:pass@host/db")

def worker_thread(thread_id):
    """Worker thread that executes workflows."""
    for i in range(10):
        context = cloaca.Context({"thread_id": thread_id, "iteration": i})
        result = runner.execute("worker_workflow", context)
        print(f"Thread {thread_id}, iteration {i}: {result.status}")

# Start multiple worker threads
threads = []
for i in range(4):
    thread = threading.Thread(target=worker_thread, args=(i,))
    threads.append(thread)
    thread.start()

# Wait for completion
for thread in threads:
    thread.join()

runner.shutdown()
```

## Related Classes

- **[Context](/python-bindings/api-reference/context/)** - Data passed to execute()
- **[DefaultRunnerConfig](/python-bindings/api-reference/configuration/)** - Configuration options
- **[PipelineResult](/python-bindings/api-reference/pipeline-result/)** - Execution results
- **[WorkflowBuilder](/python-bindings/api-reference/workflow-builder/)** - Build workflows to execute