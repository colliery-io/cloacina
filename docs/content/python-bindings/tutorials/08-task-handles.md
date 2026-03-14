---
title: "08 - Task Handles"
description: "Release concurrency slots while waiting for external conditions with TaskHandle and defer_until"
weight: 18
reviewer: "dstorey"
review_date: "2025-03-13"
---

# Task Handles and Deferred Execution

In this tutorial, you will learn how to use `TaskHandle` and `defer_until` to build workflows that wait for external conditions without wasting concurrency slots. This is essential for Python workflows that poll for file arrivals, API readiness, or other asynchronous events.

## Learning Objectives

- Understand what `TaskHandle` is and why it matters for concurrency
- Use `defer_until` to release a concurrency slot while waiting for a condition
- Build a workflow that combines deferred tasks with downstream processing
- Recognize how handle detection works under the hood

## Prerequisites

- Completion of [Tutorial 03: Complex Workflows]({{< ref "/python-bindings/tutorials/03-complex-workflows/" >}})
- Understanding of task dependencies and workflow execution
- Familiarity with Python closures and callables

## Time Estimate

15-20 minutes

## Why Task Handles Matter

Consider a workflow with a concurrency limit of 4. If one task spends minutes polling an external API waiting for data to appear, it holds a concurrency slot the entire time. Three other tasks can run, but the fourth slot is effectively wasted on a sleeping task.

`TaskHandle` solves this problem. When a task calls `handle.defer_until(condition_fn)`, the executor:

1. **Releases the concurrency slot** so other tasks can use it
2. **Polls the condition function** at a configurable interval
3. **Re-acquires the slot** once the condition returns `True`
4. **Resumes the task** from the line after `defer_until`

This means your workflow can have many waiting tasks without starving the executor of slots for tasks that are ready to do real work.

## How Handle Detection Works

The `@task` decorator inspects your function's `__code__.co_varnames` to determine whether the task needs a handle. If the second parameter is named `handle` or `task_handle`, the executor knows to provide a `TaskHandle` instance when running the task.

```python
# No handle - executor passes only context
@cloaca.task(id="normal_task")
def normal_task(context):
    pass

# Handle detected via parameter name "handle"
@cloaca.task(id="deferred_task")
def deferred_task(context, handle):
    pass

# Also works with the name "task_handle"
@cloaca.task(id="another_deferred")
def another_deferred(context, task_handle):
    pass
```

{{< hint type="info" title="Backward Compatibility" >}}
The handle parameter is entirely optional. Tasks without it work exactly as before. You only add the parameter when you need deferred execution.
{{< /hint >}}

## The TaskHandle API

The `TaskHandle` class is importable from `cloaca` for type hints:

```python
from cloaca import TaskHandle
```

It provides two methods:

| Method | Description |
|--------|-------------|
| `handle.defer_until(condition_fn, poll_interval_ms=1000)` | Release the concurrency slot and poll `condition_fn` until it returns `True`. `poll_interval_ms` controls the polling interval in milliseconds. |
| `handle.is_slot_held()` | Returns `True` if the handle currently holds a concurrency slot, `False` if the slot has been released. |

{{< hint type="info" title="Synchronous Condition Functions" >}}
The `condition_fn` passed to `defer_until` must be a regular Python callable returning `bool`. It is called synchronously from the Rust side, not as an async Python coroutine. Do not use `async def` for your condition function.
{{< /hint >}}

## Building a Deferred Pipeline

Let's build a workflow that waits for external data to become available, then processes it. This simulates a common pattern: polling for a file upload or API response before continuing.

### Step 1: Define the Waiting Task

The first task uses `defer_until` to wait for a condition. In a real application, the condition function would check for a file on disk, query an API, or inspect a message queue. Here we simulate the wait with a counter.

```python
import cloaca

with cloaca.WorkflowBuilder("deferred_pipeline") as builder:
    builder.description("Pipeline with deferred task")

    @cloaca.task(id="wait_for_data")
    def wait_for_data(context, handle):
        """Wait for external data to become available."""
        poll_count = {"value": 0}

        def condition():
            poll_count["value"] += 1
            # Simulate data arriving after 3 polls
            return poll_count["value"] >= 3

        # Release the concurrency slot and poll every 500ms
        handle.defer_until(condition, poll_interval_ms=500)

        # Execution resumes here once condition() returns True
        context.set("data_ready", True)
        context.set("polls_needed", poll_count["value"])
        return context
```

Key points about this task:

- The second parameter is named `handle`, so the executor provides a `TaskHandle`
- `poll_count` uses a dict rather than a plain integer because the closure needs a mutable reference
- `defer_until` blocks (from the task's perspective) until `condition()` returns `True`
- The lines after `defer_until` run only after the condition is met and the slot is re-acquired

### Step 2: Define the Downstream Task

The second task depends on `wait_for_data` and processes the result. It is a normal task with no handle parameter.

```python
    @cloaca.task(id="process_data", dependencies=["wait_for_data"])
    def process_data(context):
        """Process the data once available."""
        assert context.get("data_ready") is True
        context.set("processed", True)
        return context
```

### Step 3: Execute the Workflow

```python
runner = cloaca.DefaultRunner("sqlite://:memory:")
result = runner.execute("deferred_pipeline", cloaca.Context())

assert result.status == "Completed"
assert result.final_context.get("data_ready") is True
assert result.final_context.get("processed") is True

print(f"Status: {result.status}")
print(f"Polls needed: {result.final_context.get('polls_needed')}")
print(f"Data ready: {result.final_context.get('data_ready')}")
print(f"Processed: {result.final_context.get('processed')}")

runner.shutdown()
```

Expected output:

```
Status: Completed
Polls needed: 3
Data ready: True
Processed: True
```

## Complete Runnable Example

Here is the full example in a single file:

```python
import cloaca

# =============================================================================
# Workflow definition
# =============================================================================

with cloaca.WorkflowBuilder("deferred_pipeline") as builder:
    builder.description("Pipeline with deferred task")

    @cloaca.task(id="wait_for_data")
    def wait_for_data(context, handle):
        """Wait for external data to become available."""
        poll_count = {"value": 0}

        def condition():
            poll_count["value"] += 1
            return poll_count["value"] >= 3

        handle.defer_until(condition, poll_interval_ms=500)
        context.set("data_ready", True)
        context.set("polls_needed", poll_count["value"])
        return context

    @cloaca.task(id="process_data", dependencies=["wait_for_data"])
    def process_data(context):
        """Process the data once available."""
        assert context.get("data_ready") is True
        context.set("processed", True)
        return context

# =============================================================================
# Execution
# =============================================================================

def main():
    print("Task Handles and Deferred Execution Demo")
    print("=" * 50)

    runner = cloaca.DefaultRunner("sqlite://:memory:")
    result = runner.execute("deferred_pipeline", cloaca.Context())

    print(f"Status: {result.status}")
    print(f"Polls needed: {result.final_context.get('polls_needed')}")
    print(f"Data ready: {result.final_context.get('data_ready')}")
    print(f"Processed: {result.final_context.get('processed')}")

    runner.shutdown()
    print("\nDemo complete!")

if __name__ == "__main__":
    main()
```

## Mixing Handle and Non-Handle Tasks

Handle tasks and normal tasks coexist naturally in the same workflow. The executor determines the calling convention for each task independently based on its parameter names.

```python
import cloaca

with cloaca.WorkflowBuilder("mixed_tasks") as builder:
    builder.description("Mixed handle and non-handle tasks")

    @cloaca.task(id="setup")
    def setup(context):
        """Normal task - no handle needed."""
        context.set("setup_done", True)
        return context

    @cloaca.task(id="wait_for_approval", dependencies=["setup"])
    def wait_for_approval(context, handle):
        """Deferred task - releases slot while waiting."""
        handle.defer_until(lambda: True, poll_interval_ms=10)
        context.set("approved", True)
        return context

    @cloaca.task(id="finalize", dependencies=["wait_for_approval"])
    def finalize(context):
        """Normal task - runs after deferred task completes."""
        context.set("finalized", True)
        return context
```

All three tasks execute in order. The `wait_for_approval` task releases its slot during the poll, while `setup` and `finalize` hold their slots normally for the duration of their execution.

## Real-World Use Cases

### Waiting for a File Upload

```python
import os

@cloaca.task(id="wait_for_upload")
def wait_for_upload(context, handle):
    """Wait for a file to appear in the upload directory."""
    expected_path = context.get("upload_path")

    def file_exists():
        return os.path.isfile(expected_path)

    handle.defer_until(file_exists, poll_interval_ms=2000)
    context.set("file_available", True)
    return context
```

### API Readiness Check

```python
import urllib.request

@cloaca.task(id="wait_for_api")
def wait_for_api(context, handle):
    """Wait for a dependent service to become healthy."""
    health_url = context.get("health_endpoint")

    def is_healthy():
        try:
            resp = urllib.request.urlopen(health_url, timeout=2)
            return resp.status == 200
        except Exception:
            return False

    handle.defer_until(is_healthy, poll_interval_ms=5000)
    context.set("api_ready", True)
    return context
```

### Message Queue Consumer

```python
@cloaca.task(id="wait_for_message")
def wait_for_message(context, handle):
    """Wait for a message to appear on a queue."""
    queue_name = context.get("queue_name")
    message_holder = {"msg": None}

    def message_available():
        msg = check_queue(queue_name)  # Your queue check function
        if msg is not None:
            message_holder["msg"] = msg
            return True
        return False

    handle.defer_until(message_available, poll_interval_ms=1000)
    context.set("message", message_holder["msg"])
    return context
```

## Using TaskHandle for Type Hints

If you want explicit type annotations, import `TaskHandle` from `cloaca`:

```python
from cloaca import Context, TaskHandle

@cloaca.task(id="typed_deferred")
def typed_deferred(context: Context, handle: TaskHandle) -> Context:
    """Fully type-annotated deferred task."""
    handle.defer_until(lambda: True, poll_interval_ms=100)
    context.set("done", True)
    return context
```

The type hints are purely for developer tooling and readability. The executor relies on the parameter name (`handle` or `task_handle`), not the type annotation, to decide whether to inject a `TaskHandle`.

## What You've Learned

In this tutorial, you learned:

- **TaskHandle** lets tasks release concurrency slots while waiting for external conditions
- **`defer_until`** polls a synchronous condition function at a configurable interval
- **Handle detection** works by inspecting the second parameter name (`handle` or `task_handle`)
- **Normal tasks are unaffected** - the handle parameter is entirely optional
- **Real-world applications** include file polling, API health checks, and message queue consumption

## Next Steps

{{< button relref="/python-bindings/api-reference/task/" >}}Task API Reference{{< /button >}}
{{< button relref="/python-bindings/api-reference/runner/" >}}Runner API Reference{{< /button >}}

## Related Resources

- [Tutorial 03: Complex Workflows]({{< ref "/python-bindings/tutorials/03-complex-workflows/" >}}) - Dependency patterns and parallel execution
- [Tutorial 07: Event Triggers]({{< ref "/python-bindings/tutorials/07-event-triggers/" >}}) - Event-driven workflow execution
- [API Reference: Task Decorator]({{< ref "/python-bindings/api-reference/task/" >}}) - Complete task configuration options

{{< hint type="info" title="Reference Implementation" >}}
This tutorial is based on the test suite at [`tests/python/test_scenario_31_task_handle.py`](https://github.com/colliery-io/cloacina/tree/main/tests/python/test_scenario_31_task_handle.py).
{{< /hint >}}
