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
- `retry_policy` (dict): Retry configuration for handling failures
- `timeout` (int): Maximum execution time in seconds

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

## Context Usage

Tasks receive a [Context](/python-bindings/api-reference/context/) object for data flow:

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

- **[Context](/python-bindings/api-reference/context/)** - Data passed between tasks
- **[WorkflowBuilder](/python-bindings/api-reference/workflow-builder/)** - Combine tasks into workflows
- **[DefaultRunner](/python-bindings/api-reference/runner/)** - Execute workflows containing tasks
