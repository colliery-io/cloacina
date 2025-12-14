---
title: "Task Decorator"
description: "Define workflow tasks with the @task decorator"
weight: 50
---

# Task Decorator

The `@task` decorator is used to define individual tasks that can be executed as part of workflows. Tasks are the fundamental building blocks of Cloaca workflows.

## Basic Usage

```python
import cloaca

@cloaca.task(id="my_task")
def my_task(context):
    """Example task that processes data."""
    # Task implementation
    context.set("result", "Task completed successfully")
    return context
```

## Decorator Parameters

### Required Parameters

- `id` (str): Unique identifier for the task within the workflow

### Optional Parameters

- `dependencies` (list): List of task IDs that must complete before this task runs
- `retry_attempts` (int): Number of retry attempts on failure
- `retry_backoff` (str): Backoff strategy: "fixed", "linear", or "exponential"
- `retry_delay_ms` (int): Initial delay between retries in milliseconds
- `retry_max_delay_ms` (int): Maximum delay between retries
- `retry_condition` (str): When to retry: "never", "transient", or "all"
- `retry_jitter` (bool): Add random jitter to retry delays
- `on_success` (callable): Callback function called when task succeeds
- `on_failure` (callable): Callback function called when task fails

## Example with Dependencies

```python
@cloaca.task(id="fetch_data")
def fetch_data(context):
    """Fetch raw data from source."""
    data = {"raw_data": [1, 2, 3, 4, 5]}
    context.set("raw_data", data)
    return context

@cloaca.task(id="process_data", dependencies=["fetch_data"])
def process_data(context):
    """Process the fetched data."""
    raw_data = context.get("raw_data")
    processed = {"processed_data": [x * 2 for x in raw_data["raw_data"]]}
    context.set("processed_data", processed)
    return context
```

## Async Tasks

Tasks can be defined as async functions for non-blocking operations:

```python
import asyncio

@cloaca.task(id="async_task")
async def async_task(context):
    """Example async task."""
    await asyncio.sleep(1)  # Simulate async operation
    context.set("async_result", "Async operation completed")
    return context
```

## Error Handling

Tasks should handle errors gracefully and return appropriate results:

```python
@cloaca.task(id="safe_task")
def safe_task(context):
    """Task with error handling."""
    try:
        # Potentially failing operation
        result = risky_operation()
        context.set("success", True)
        context.set("result", result)
    except Exception as e:
        context.set("success", False)
        context.set("error", str(e))

    return context
```

## Task Callbacks

Use `on_success` and `on_failure` callbacks for monitoring, alerting, or cleanup:

### Callback Signatures

```python
def on_success_callback(task_id: str, context: Context) -> None:
    """Called when the task completes successfully."""
    pass

def on_failure_callback(task_id: str, error: str, context: Context) -> None:
    """Called when the task fails."""
    pass
```

### Example with Callbacks

```python
import cloaca

def log_success(task_id, context):
    """Log successful task completion."""
    print(f"Task '{task_id}' completed successfully")
    # Send metrics, update monitoring, etc.

def alert_failure(task_id, error, context):
    """Alert on task failure."""
    print(f"ALERT: Task '{task_id}' failed: {error}")
    # Send to Slack, PagerDuty, etc.

@cloaca.task(
    id="monitored_task",
    on_success=log_success,
    on_failure=alert_failure
)
def monitored_task(context):
    """Task with monitoring callbacks."""
    result = perform_operation()
    context.set("result", result)
    return context
```

### Callback Error Isolation

Errors in callbacks are isolated and logged - they don't affect task execution:

```python
def buggy_callback(task_id, context):
    raise Exception("Callback error!")  # Won't fail the task

@cloaca.task(id="resilient_task", on_success=buggy_callback)
def resilient_task(context):
    """Task completes even if callback fails."""
    return context
```

### Common Callback Patterns

```python
# Alerting pattern
def slack_alert(task_id, error, context):
    webhook_url = os.environ.get("SLACK_WEBHOOK")
    requests.post(webhook_url, json={
        "text": f"Task {task_id} failed: {error}"
    })

# Metrics pattern
def record_metrics(task_id, context):
    duration = context.get("duration_ms", 0)
    statsd.timing(f"task.{task_id}.duration", duration)

# Cleanup pattern
def cleanup_temp_files(task_id, error, context):
    temp_dir = context.get("temp_dir")
    if temp_dir and os.path.exists(temp_dir):
        shutil.rmtree(temp_dir)
```

## Context Usage

Tasks receive a [Context]({{< ref "/python-bindings/api-reference/context/" >}}) object for data flow:

```python
@cloaca.task(id="context_example")
def context_example(context):
    """Demonstrate context usage."""
    # Get data from previous tasks
    input_value = context.get("input_key", "default_value")

    # Process the data
    result = input_value.upper()

    # Set results for subsequent tasks
    context.set("output_key", result)
    context.set("processing_complete", True)

    return context
```

## Best Practices

### Idempotency

Design tasks to be idempotent when possible:

```python
@cloaca.task(id="idempotent_task")
def idempotent_task(context):
    """Task that can be safely retried."""
    # Check if already processed
    if context.get("already_processed"):
        return context

    # Perform operation
    result = perform_operation()

    # Mark as processed
    context.set("result", result)
    context.set("already_processed", True)

    return context
```

### Clear Error Messages

Provide meaningful error information:

```python
@cloaca.task(id="validation_task")
def validation_task(context):
    """Task with clear validation."""
    data = context.get("data")

    if not data:
        context.set("error", "Required 'data' field is missing")
        context.set("valid", False)
        return context

    if not isinstance(data, dict):
        context.set("error", f"Expected dict, got {type(data).__name__}")
        context.set("valid", False)
        return context

    context.set("valid", True)
    return context
```

## See Also

- **[Context]({{< ref "/python-bindings/api-reference/context/" >}})** - Data passed between tasks
- **[WorkflowBuilder]({{< ref "/python-bindings/api-reference/workflow-builder/" >}})** - Combine tasks into workflows
- **[DefaultRunner]({{< ref "/python-bindings/api-reference/runner/" >}})** - Execute workflows containing tasks
