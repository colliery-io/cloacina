---
title: "Task Decorator"
description: "Define workflow tasks with the @task decorator"
sidebar:
  order: 50
---

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

All parameters are keyword-only and optional
(`crates/cloacina-python/src/task.rs`):

- `id` (str): Unique identifier for the task within the workflow. Defaults to the decorated function's name.
- `dependencies` (list): List of task IDs (strings or task function references) that must complete before this task runs
- `retry_attempts` (int): Number of retry attempts on failure
- `retry_backoff` (str): Backoff strategy: "fixed", "linear", or "exponential"
- `retry_delay_ms` (int): Initial delay between retries in milliseconds
- `retry_max_delay_ms` (int): Maximum delay between retries
- `retry_condition` (str): When to retry: "never", "transient", or "all"
- `retry_jitter` (bool): Add random jitter to retry delays
- `timeout_seconds` (int): The task's own time limit in seconds (greater than 0; `0` raises `ValueError`). When set it replaces the executor's limit for this task: `task_timeout_seconds` in process, the server default on an execution-agent fleet, where the server maximum caps it. Default: no own limit. Python parity with Rust's `#[task(timeout_seconds = …)]`.
- `pool` (str): The [agent pool](../../../explanation/service/execution-agent-fleet/#agent-pools) the task runs in on an execution-agent fleet. A task with a pool runs only on an agent of that pool; a task with none runs only on the pool `default`. The name is lowercase letters, digits and `-`, a letter or digit at both ends, 40 characters at most; another name raises `ValueError`. Python parity with Rust's `#[task(pool = …)]`.
- `cpu` (str or number): CPU request, a Kubernetes quantity above zero: `"500m"`, `"4"`, or the number `4`. See [Pool and requests](#pool-and-requests).
- `memory` (str or number): Memory request, a Kubernetes quantity above zero: `"512Mi"`, `"32Gi"`, or a number of bytes.
- `inline` (bool): Marks the task as one that runs inside the agent process. The mark is stored with the task and sent to the agent. It changes nothing about how the task runs. Default: `False`.
- `priority` (int): The task's own priority, 0 (the highest) to 10. It orders the task among the waiting tasks of the same workflow priority. Default: the priority of the task's workflow. A value outside 0 to 10 raises `ValueError`. The workflow's priority is set with `WorkflowBuilder.priority(n)`. See [Priority](../../../explanation/priority/).
- `on_success` (callable): Callback function called when task succeeds
- `on_failure` (callable): Callback function called when task fails
- `invokes` (ComputationGraphBuilder): A **trigger-less** computation graph this task invokes: after the task body succeeds, the engine executes the graph with the task's context. The referenced `with ComputationGraphBuilder(...)` block must run before the decorator, or the decorator raises `ValueError`. Python parity with Rust's `#[task(invokes = …)]`.
- `post_invocation` (callable): Callback run after the invoked graph completes; only meaningful with `invokes=` (setting it alone raises `ValueError`).
- `trigger_rules` (rule): Conditional gate — when the rule evaluates false (and the task's dependencies are otherwise satisfied), the task lands in the real `Skipped` state and its body never runs. Python parity with Rust's `#[task(trigger_rules = …)]`. Default: always run.

## Pool and requests

`pool`, `cpu` and `memory` say where a task runs on an
[execution-agent fleet](../../../explanation/service/execution-agent-fleet/#how-a-task-is-placed)
and how much room it needs there. All three are optional.

```python
@cloaca.task(pool="large", cpu="4", memory="32Gi")
def load_history(context):
    return context

@cloaca.task(memory="512Mi")
def summarize(context):
    return context
```

- The values are checked when the decorator is called. A bad pool name or a
  quantity that is not a Kubernetes quantity above zero (`"32GB"`, `"0"`, `-1`)
  raises `ValueError`. A value that is not a string or a number raises
  `TypeError`.
- A quantity uses the Kubernetes forms: a number with no suffix (`"4"`,
  `"0.5"`), a decimal suffix (`m`, `k`, `M`, `G`, `T`) or a binary suffix
  (`Ki`, `Mi`, `Gi`, `Ti`). A Python number is read as its text, so
  `memory=1073741824` is that many bytes.
- **A request is used for placement only.** The server counts the requests of
  the tasks in flight on each agent and sends a task where its request still
  fits. Nothing limits what a task uses: a task that asks for `2Gi` and uses
  `20Gi` is not stopped, and it can take down the agent with the other tasks on
  it. Ask for what the task needs at its peak.
- A task with no request counts as one slot's share of its agent: the agent's
  total divided by its max concurrency.
- When a package is published, a task that names a pool the tenant does not
  have, or that requests more than one agent of its pool can hold, makes the
  package fail with an error that names the task. See
  [what a publish checks](../../../explanation/service/execution-agent-fleet/#what-a-publish-checks).
- The in-process executor does not read these parameters. They take effect
  when tasks run on a fleet.

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

See [Trigger rules](../../../explanation/engine/trigger-rules/) for evaluation semantics
and skip propagation.

## Documenting a task: `what:` / `why:`

A task's docstring is a documentation surface. Line-leading, case-insensitive
`what:` / `why:` markers route the following text into the task's `what` and
`why` fields; with no markers the whole docstring becomes `what` and `why` is
empty.

```python
@cloaca.task(id="extract")
def extract(context):
    """
    what: pulls rows from the source
    why: the rest of the graph needs them staged
    """
    return context
```

The compiler parses this from source at build time (see
`crates/cloacina-compiler/src/doc_parse.rs`); parsing is best-effort and never
fails the build.

:::note[Pure-Python persistence is a deferred follow-up]
The docstring is **parsed** with the same convention as Rust, but for
pure-Python packages the parsed `what` / `why` may not yet round-trip into the
persisted per-task metadata — that persistence path is tracked separately
(CLOACI-T-0754). Treat the markers as forward-compatible authoring today.
:::

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

## Task Functions Are Synchronous

Task bodies are plain (synchronous) functions. The executor runs each body on
a blocking worker thread and expects the return value to be the `Context` (or
`None` to keep the incoming context unchanged) —
`async def` bodies are not awaited, so an async task fails when its coroutine
return value cannot be converted back into a `Context`
(`crates/cloacina-python/src/task.rs`).

To wait on an external condition without holding a concurrency slot, use a
[TaskHandle](#taskhandle) with `defer_until` instead of `await`.

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

Tasks receive a [Context](../context/) object for data flow:

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

- **[Context](../context/)** - Data passed between tasks
- **[WorkflowBuilder](../workflow-builder/)** - Combine tasks into workflows
- **[DefaultRunner](../runner/)** - Execute workflows containing tasks
- **[Task Deferral Architecture](../../../explanation/engine/task-deferral/)** - Internal mechanics of slot tokens and defer_until
- **[Tutorial 08 - Task Deferral](../../../tutorials/embed/08-task-deferral/)** - Rust walkthrough of the deferred-tasks example
