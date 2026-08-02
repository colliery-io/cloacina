---
title: "DefaultRunner"
description: "DefaultRunner class for workflow execution"
weight: 20
reviewer: "automation"
review_date: "2025-01-07"
aliases:
  - "/python/api-reference/runner/"

---

# DefaultRunner

The `DefaultRunner` class is the main execution engine for Cloaca workflows. It manages database connections, task scheduling, execution, and provides cron scheduling capabilities.

{{< hint type="note" title="Wheel-only" >}}
`DefaultRunner` (with `WorkflowResult` and `DefaultRunnerConfig`) exists only in
the pip-installed `cloaca` wheel. It is not available inside packaged
`.cloacina` workflows — there the server or agent is the runner. The runner
runs its async runtime on a dedicated OS thread; every method releases the GIL
while waiting, and an `atexit` hook joins all runner threads at interpreter
exit even if you forget `shutdown()`.
{{< /hint >}}

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

**Raises:** ValueError if the URL is not PostgreSQL or the schema name is invalid

**Example:**
```python
import cloaca

# Multi-tenant setup
tenant_runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:pass@localhost:5432/dbname",
    "tenant_acme"
)
```

**Schema Naming Rules** (validated by the binding):
- Must not be empty
- May contain only alphanumeric characters and underscores
- The database URL must start with `postgres://` or `postgresql://` — SQLite is rejected

## Workflow Execution

### `execute(workflow_name, context)`

Execute a workflow with the given context.

**Parameters:**
- `workflow_name` (str): Name of the registered workflow
- `context` (Context): Initial workflow context

**Returns:** [WorkflowResult]({{< ref "/reference/python-api/pipeline-result/" >}}) with execution details

**Raises:** ValueError if the workflow is unknown or execution is rejected

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

Cron expressions are validated by the engine's `CronEvaluator`
(`crates/cloacina-workflow/src/cron_evaluator.rs`): standard **5-field**
expressions (`minute hour day month weekday`), with an optional leading
seconds field also accepted.

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

### `register_workflow_instance(workflow_name, instance_name, cron_expression, timezone, params)`

Register a **named, parameterized** cron instance (CLOACI-I-0116). `params` is
a `Context` whose contents are the instance's fully-resolved bound parameters,
merged into the run context under flat top-level keys at every fire.

**Parameters:**
- `workflow_name` (str): Name of the workflow to schedule
- `instance_name` (str): Name for this schedule instance
- `cron_expression` (str): Cron expression
- `timezone` (str): Timezone name
- `params` (Context): Bound parameters for this instance

**Returns:** str - Schedule ID (UUID)

**Example:**
```python
schedule_id = runner.register_workflow_instance(
    "daily_report",
    "daily_report_emea",
    "0 6 * * *",
    "Europe/Berlin",
    cloaca.Context({"region": "emea"}),
)
```

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

**Returns:** List[dict] - List of execution records. Each dict has the keys
`id`, `schedule_id`, `scheduled_time`, `claimed_at`, `workflow_execution_id`,
`created_at`, `updated_at`.

**Example:**
```python
# Get recent executions
history = runner.get_cron_execution_history(schedule_id, limit=20)

for execution in history:
    print(f"Scheduled: {execution['scheduled_time']}")
    print(f"Claimed: {execution['claimed_at']}")
    print(f"Workflow execution: {execution['workflow_execution_id']}")
```

### `get_cron_execution_stats(since)`

Get execution statistics since a given timestamp.

**Parameters:**
- `since` (str): RFC 3339 timestamp to calculate stats from (invalid formats raise `ValueError`)

**Returns:** dict with the keys `total_executions`, `successful_executions`,
`lost_executions`, `success_rate`

**Example:**
```python
# Stats for last 24 hours
since = (datetime.now() - timedelta(days=1)).isoformat()
stats = runner.get_cron_execution_stats(since)

print(f"Total executions: {stats['total_executions']}")
print(f"Successful: {stats['successful_executions']}")
print(f"Success rate: {stats['success_rate']:.2%}")
```

## Trigger Management

Event triggers poll custom conditions and fire workflows when conditions are met. See [Trigger Decorator]({{< ref "/reference/python-api/trigger/" >}}) for defining triggers.

### `list_trigger_schedules(enabled_only=None, limit=None, offset=None)`

List all registered trigger schedules.

**Parameters:**
- `enabled_only` (bool, optional): Filter by enabled status
- `limit` (int, optional): Maximum number of results
- `offset` (int, optional): Number of results to skip

**Returns:** List[dict] - List of trigger schedule dictionaries

**Example:**
```python
# List all triggers
triggers = runner.list_trigger_schedules()

# List only enabled triggers
enabled_triggers = runner.list_trigger_schedules(enabled_only=True)

# Paginated results
recent_triggers = runner.list_trigger_schedules(limit=10, offset=0)

for trigger in triggers:
    print(f"Trigger: {trigger['trigger_name']} -> {trigger['workflow_name']}")
    print(f"  Poll interval: {trigger['poll_interval_ms']}ms")
    print(f"  Enabled: {trigger['enabled']}")
```

### `get_trigger_schedule(trigger_name)`

Get details of a specific trigger schedule.

**Parameters:**
- `trigger_name` (str): Name of the trigger

**Returns:** dict with the keys `id`, `trigger_name`, `workflow_name`,
`poll_interval_ms`, `allow_concurrent`, `enabled`, `last_poll_at`,
`created_at`, `updated_at` — or `None` if no trigger by that name exists

**Example:**
```python
schedule = runner.get_trigger_schedule("file_watcher")

print(f"Trigger: {schedule['trigger_name']}")
print(f"Workflow: {schedule['workflow_name']}")
print(f"Poll interval: {schedule['poll_interval_ms']}ms")
print(f"Allow concurrent: {schedule['allow_concurrent']}")
print(f"Enabled: {schedule['enabled']}")
print(f"Last poll: {schedule['last_poll_at']}")
```

### `set_trigger_enabled(trigger_name, enabled)`

Enable or disable a trigger.

**Parameters:**
- `trigger_name` (str): Name of the trigger
- `enabled` (bool): Whether trigger should be enabled

**Example:**
```python
# Disable trigger during maintenance
runner.set_trigger_enabled("file_watcher", False)

# Re-enable trigger
runner.set_trigger_enabled("file_watcher", True)
```

### `get_trigger_execution_history(trigger_name, limit=None, offset=None)`

Get execution history for a trigger.

**Parameters:**
- `trigger_name` (str): Name of the trigger
- `limit` (int, optional): Maximum number of results
- `offset` (int, optional): Number of results to skip

**Returns:** List[dict] - List of execution records. Each dict has the keys
`id`, `schedule_id`, `context_hash`, `workflow_execution_id`, `started_at`,
`completed_at`, `created_at`.

**Example:**
```python
# Get recent trigger executions
history = runner.get_trigger_execution_history("file_watcher", limit=20)

for execution in history:
    print(f"Started: {execution['started_at']}")
    print(f"Completed: {execution['completed_at']}")
    print(f"Context hash: {execution['context_hash']}")
    print(f"Workflow execution: {execution['workflow_execution_id']}")
```

## Reactor Subscriptions

A workflow can subscribe to a [reactor]({{< ref "/reference/python-api/computation-graphs/" >}})
so that every reactor firing executes the workflow, optionally filtered by a
CEL predicate over the firing payload.

### `subscribe_workflow_to_reactor(reactor, workflow, tenant=None, *, when=None)`

Subscribe a workflow to a reactor's firings.

**Parameters:**
- `reactor` (str): Reactor name
- `workflow` (str): Workflow name to execute on each firing
- `tenant` (str, optional): Tenant id (defaults to the runner's tenant)
- `when` (str, optional, keyword-only): CEL filter over the firing payload; the workflow runs only when it evaluates true

**Returns:** str - Subscription ID

**Example:**
```python
# Fire on every reactor firing
runner.subscribe_workflow_to_reactor("pricing", "alert")

# Fire only when the boundary's quote source carries a price above $100
runner.subscribe_workflow_to_reactor(
    "pricing", "alert",
    when="payload.quote.price > 100 && payload.quote.region == 'us-east'",
)
```

### `unsubscribe_workflow_from_reactor(reactor, workflow, tenant=None)`

Remove a subscription.

**Returns:** bool - `True` if a subscription was deleted, `False` if none matched

### `list_reactor_subscriptions(tenant=None)`

List enabled reactor subscriptions for a tenant.

**Returns:** List[dict] - Each dict has the keys `id`, `reactor_name`,
`workflow_name`, `tenant_id`, `enabled`, `last_seen_fired_at`, `created_at`,
`updated_at`.

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

See [DefaultRunnerConfig]({{< ref "/reference/python-api/configuration/" >}}) for detailed configuration options.

**Key Configuration Options:**
- `max_concurrent_tasks`: Number of tasks that can run simultaneously
- `task_timeout_seconds`: Maximum time a task can run
- `db_pool_size`: Database connection pool size
- `enable_recovery`: Whether to recover orphaned workflows
- `enable_cron_scheduling`: Whether to enable cron scheduling

## Database URLs

### SQLite
```python
# Relative path
"sqlite://app.db"

# Absolute path
"sqlite:///path/to/database.db"

# In-memory database (testing only) — replaced internally with a per-runner
# tempfile so all pool connections share one database
"sqlite://:memory:"
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

Construction failures raise `RuntimeError`; rejected operations raise
`ValueError` (see [Exceptions]({{< ref "/reference/python-api/exceptions/" >}})):

```python
import cloaca

try:
    runner = cloaca.DefaultRunner("invalid://connection/string")
except RuntimeError as e:
    print(f"Could not start runner: {e}")

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
config.workflow_timeout_seconds = 7200  # 2 hours per workflow

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

- **[Context]({{< ref "/reference/python-api/context/" >}})** - Data passed to execute()
- **[DefaultRunnerConfig]({{< ref "/reference/python-api/configuration/" >}})** - Configuration options
- **[WorkflowResult]({{< ref "/reference/python-api/pipeline-result/" >}})** - Execution results
- **[WorkflowBuilder]({{< ref "/reference/python-api/workflow-builder/" >}})** - Build workflows to execute
