---
title: "Trigger Decorator"
description: "Define event-driven workflow triggers with the @trigger decorator"
weight: 55
---

# Trigger Decorator

The `@trigger` decorator is used to define event-driven triggers that poll user-defined conditions and fire workflows when those conditions are met. Unlike cron scheduling (time-based), event triggers allow reactive workflow execution based on custom logic.

## Basic Usage

```python
import cloaca

@cloaca.trigger(
    workflow="my_workflow",
    poll_interval="5s"
)
def my_trigger():
    """Example trigger that checks a condition."""
    if some_condition_is_met():
        return cloaca.TriggerResult.fire()
    return cloaca.TriggerResult.skip()
```

## Decorator Parameters

### Required Parameters

- `workflow` (str): Name of the workflow to trigger when the condition is met

### Optional Parameters

- `name` (str): Unique identifier for the trigger (defaults to function name)
- `poll_interval` (str): How often to poll the trigger condition (e.g., "5s", "100ms", "1m"). Defaults to "5s"
- `allow_concurrent` (bool): Whether to allow concurrent executions of the same trigger. Defaults to `False`

## TriggerResult Class

The trigger function must return a `TriggerResult` object:

### TriggerResult.skip()

Returns a Skip result indicating the condition is not met. Polling continues on the next interval.

```python
result = cloaca.TriggerResult.skip()
assert result.is_skip_result() == True
```

### TriggerResult.fire(context=None)

Returns a Fire result indicating the condition is met. The workflow will be triggered.

```python
# Fire without context
result = cloaca.TriggerResult.fire()
assert result.is_fire_result() == True

# Fire with context
ctx = cloaca.Context({"key": "value"})
result = cloaca.TriggerResult.fire(ctx)
```

## Example with Context

Pass data from the trigger to the workflow via context:

```python
@cloaca.trigger(
    workflow="file_processor",
    name="file_watcher",
    poll_interval="10s",
    allow_concurrent=False
)
def file_watcher():
    """Monitor for new files and trigger processing."""
    new_file = check_for_new_files("/data/inbox/")
    if new_file:
        ctx = cloaca.Context({
            "filename": new_file,
            "detected_at": datetime.now().isoformat()
        })
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

## Deduplication

When `allow_concurrent=False` (the default), the trigger scheduler prevents duplicate executions:

1. Context is hashed when `TriggerResult.fire()` is returned
2. Active executions are tracked by (trigger_name, context_hash)
3. If an execution with the same hash is running, the trigger skips

```python
@cloaca.trigger(
    workflow="order_processor",
    allow_concurrent=False  # Default - prevents duplicate processing
)
def order_trigger():
    """Only process each order once."""
    order = get_pending_order()
    if order:
        ctx = cloaca.Context({"order_id": order.id})
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

## Concurrent Execution

Set `allow_concurrent=True` for triggers that should scale horizontally:

```python
@cloaca.trigger(
    workflow="queue_worker",
    poll_interval="1s",
    allow_concurrent=True  # Allow parallel queue processing
)
def queue_trigger():
    """Process queue items in parallel."""
    item = peek_queue_item()
    if item:
        ctx = cloaca.Context({"item_id": item.id})
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

## Common Patterns

### Health Check Trigger

Fire recovery workflow after consecutive failures:

```python
failure_count = 0

@cloaca.trigger(
    workflow="service_recovery",
    poll_interval="30s"
)
def health_check():
    """Monitor service health and trigger recovery."""
    global failure_count

    if check_service_healthy():
        failure_count = 0
        return cloaca.TriggerResult.skip()

    failure_count += 1
    if failure_count >= 3:
        failure_count = 0
        ctx = cloaca.Context({
            "service": "api",
            "consecutive_failures": 3
        })
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

### Threshold Trigger

Fire when a metric exceeds a threshold:

```python
@cloaca.trigger(
    workflow="scale_up",
    poll_interval="10s",
    allow_concurrent=True
)
def queue_depth_trigger():
    """Scale workers when queue gets deep."""
    depth = get_queue_depth()
    if depth > 100:
        ctx = cloaca.Context({
            "queue_depth": depth,
            "action": "scale_up"
        })
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

## Best Practices

### Keep Polls Lightweight

The poll function should be quick and avoid heavy processing:

```python
# Good: Quick check
@cloaca.trigger(workflow="processor", poll_interval="5s")
def good_trigger():
    if file_exists("/inbox/trigger.flag"):
        return cloaca.TriggerResult.fire()
    return cloaca.TriggerResult.skip()

# Bad: Heavy processing in poll
@cloaca.trigger(workflow="processor", poll_interval="5s")
def bad_trigger():
    data = download_large_file()  # Don't do this!
    process_data(data)
    return cloaca.TriggerResult.fire()
```

### Use Context for Deduplication

Include identifying information in context to enable deduplication:

```python
# Good: Context identifies the specific item
ctx = cloaca.Context({
    "filename": filename,
    "file_hash": compute_hash(filename)
})
return cloaca.TriggerResult.fire(ctx)

# Bad: No identifying information
return cloaca.TriggerResult.fire()  # All fires look identical!
```

### Handle Errors Gracefully

Errors in trigger functions are logged and polling continues:

```python
@cloaca.trigger(workflow="data_sync", poll_interval="1m")
def resilient_trigger():
    """Trigger with error handling."""
    try:
        if check_for_updates():
            return cloaca.TriggerResult.fire()
    except Exception as e:
        logging.warning(f"Trigger check failed: {e}")
    return cloaca.TriggerResult.skip()
```

## Managing Triggers

Query and control triggers programmatically:

```python
runner = cloaca.DefaultRunner("sqlite://workflows.db")

# List all triggers
schedules = runner.list_trigger_schedules()
for schedule in schedules:
    print(f"{schedule['trigger_name']}: {schedule['enabled']}")

# Enable/disable triggers
runner.set_trigger_enabled("file_watcher", False)

# View execution history
history = runner.get_trigger_execution_history("file_watcher")
for execution in history:
    print(f"Started: {execution['started_at']}")
```

## See Also

- **[Context]({{< ref "/python-bindings/api-reference/context/" >}})** - Data passed from triggers to workflows
- **[WorkflowBuilder]({{< ref "/python-bindings/api-reference/workflow-builder/" >}})** - Define workflows that triggers activate
- **[DefaultRunner]({{< ref "/python-bindings/api-reference/runner/" >}})** - Execute workflows and manage triggers
- **[Tutorial: Event Triggers]({{< ref "/tutorials/09-event-triggers/" >}})** - Step-by-step trigger implementation guide
