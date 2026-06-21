---
title: "Task Decorator"
description: "Define workflow tasks with the @task decorator"
weight: 50
aliases:
  - "/python/api-reference/task/"

---

# Task Decorator

The `@task` decorator is used to define individual tasks that can be executed as part of workflows. Tasks are the fundamental building blocks of Cloaca workflows.

## Basic Usage

```python
import cloaca

@cloaca.task()
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
- `trigger_rules` (rule): Conditional gate — when the rule evaluates false (and the task's dependencies are otherwise satisfied), the task lands in the real `Skipped` state and its body never runs. Python parity with Rust's `#[task(trigger_rules = …)]`.

## Trigger rules

A task's `trigger_rules` gate whether it runs once its dependencies resolve. When
the rule is unsatisfied the task is **skipped** (not failed), and the skip
propagates to downstream dependents like any other branch-not-taken.

Build rules with the `cloaca` helpers:

| Builder | Meaning |
|---|---|
| `cloaca.always()` | Always run (the default). |
| `cloaca.task_success(name)` / `task_failed(name)` / `task_skipped(name)` | Gate on an upstream task's outcome. |
| `cloaca.context_value(key, op, value)` | Gate on a context value; `op` is `"Equals"`, `"NotEquals"`, `"GreaterThan"`, … |
| `cloaca.all_of(*rules)` / `any_of(*rules)` / `none_of(*rules)` | Combine conditions. |

```python
@cloaca.task(
    dependencies=["poll"],
    trigger_rules=cloaca.context_value("do_audit", "Equals", True),
)
def audit(context):
    # Runs only when the upstream set do_audit = True; otherwise Skipped.
    return context
```

See [Trigger rules](/engine/explanation/trigger-rules/) for evaluation semantics
and skip propagation.

## Example with Dependencies

```python
@cloaca.task()
def fetch_data(context):
    """Fetch raw data from source."""
    data = {"raw_data": [1, 2, 3, 4, 5]}
    context.set("raw_data", data)
    return context

@cloaca.task(dependencies=["fetch_data"])
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

@cloaca.task()
async def async_task(context):
    """Example async task."""
    await asyncio.sleep(1)  # Simulate async operation
    context.set("async_result", "Async operation completed")
    return context
```

## Error Handling

Tasks should handle errors gracefully and return appropriate results:

```python
@cloaca.task()
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

@cloaca.task(on_success=buggy_callback)
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

Tasks receive a [Context]({{< ref "/reference/python-api/context/" >}}) object for data flow:

```python
@cloaca.task()
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
@cloaca.task()
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
@cloaca.task()
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

## TaskHandle

Tasks can accept an optional second parameter named `handle` or `task_handle` to gain access to concurrency slot management. The `@task` decorator inspects the function signature at registration time and, when a handle parameter is detected, arranges for the executor to provide a `TaskHandle` instance at runtime.

### Requesting a TaskHandle

Add a second parameter named `handle` or `task_handle` to the task function:

```python
@cloaca.task()
def wait_for_ready(context, handle):
    """Task that defers until an external condition is met."""
    def check_ready():
        # Return True when the task should resume
        return some_external_check()

    handle.defer_until(check_ready, poll_interval_ms=1000)

    context.set("ready", True)
    return context
```

The parameter name matters: it must be exactly `handle` or `task_handle`. Any other name will not trigger handle injection.

### TaskHandle Methods

#### `defer_until(condition, poll_interval_ms=1000)`

Release the concurrency slot while polling an external condition.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `condition` | `callable` | required | A function returning `bool`. Called repeatedly at `poll_interval_ms` intervals. |
| `poll_interval_ms` | `int` | `1000` | Milliseconds between condition checks. |

**Behavior:**

1. Releases the executor concurrency slot (freeing it for other tasks)
2. Polls `condition()` every `poll_interval_ms` milliseconds
3. Reclaims a slot when `condition()` returns `True`
4. Returns control to the task

**Raises:** `ValueError` if the handle has already been consumed or the semaphore is closed.

```python
@cloaca.task()
def wait_for_file(context, handle):
    import os

    target = "/data/input.csv"

    handle.defer_until(
        lambda: os.path.exists(target),
        poll_interval_ms=5000,
    )

    context.set("file_path", target)
    return context
```

#### `is_slot_held()`

Returns whether the handle currently holds a concurrency slot.

**Returns:** `bool`

**Raises:** `ValueError` if the handle has already been consumed.

### Mixing Handle and Non-Handle Tasks

Handle-aware and regular tasks work together in the same workflow:

```python
with cloaca.WorkflowBuilder("mixed_pipeline") as builder:
    @cloaca.task()
    def normal_task(context):
        context.set("step_1", True)
        return context

    @cloaca.task(dependencies=["normal_task"])
    def deferred_task(context, handle):
        handle.defer_until(lambda: True, poll_interval_ms=10)
        context.set("step_2", True)
        return context
```

### Direct Calls (Testing)

When calling a handle-aware task directly (outside the executor), pass `None` for the handle parameter:

```python
ctx = cloaca.Context()
result = my_handle_task(ctx, None)
```

The `defer_until` call will fail if there is no real `TaskHandle`, so direct calls are useful only for testing the non-deferral parts of the function logic.

## See Also

- **[Context]({{< ref "/reference/python-api/context/" >}})** - Data passed between tasks
- **[WorkflowBuilder]({{< ref "/reference/python-api/workflow-builder/" >}})** - Combine tasks into workflows
- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** - Execute workflows containing tasks
- **[Task Deferral Architecture]({{< ref "/engine/explanation/task-deferral" >}})** - Internal mechanics of slot tokens and defer_until
- **[Tutorial 08 - Task Deferral]({{< ref "/embed/tutorials/08-task-deferral" >}})** - Rust walkthrough of the deferred-tasks example
