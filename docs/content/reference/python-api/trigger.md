---
title: "Trigger Decorator"
description: "Define event-driven workflow triggers with the @trigger decorator"
weight: 55
aliases:
  - "/python/api-reference/trigger/"

---

# Trigger Decorator

The `@trigger` decorator is used to define event-driven triggers that poll user-defined conditions and fire workflows when those conditions are met, based on custom logic rather than a fixed clock.

## Basic Usage

```python
import cloaca

@cloaca.trigger(
    on="my_workflow",
    poll_interval="5s"
)
def my_trigger():
    """Example trigger that checks a condition."""
    if some_condition_is_met():
        return cloaca.TriggerResult.fire()
    return cloaca.TriggerResult.skip()
```

## Decorator Parameters

All parameters are keyword-only
(`crates/cloacina-python/src/trigger.rs`, mirroring Rust's `#[trigger]` macro,
CLOACI-T-0688):

- `on` (str): Name of the workflow this trigger fires. **Required for cron triggers**; a poll trigger may omit it when workflows bind to the trigger by subscription instead.
- `name` (str): Unique identifier for the trigger (defaults to function name)
- `poll_interval` (str): How often to poll the trigger condition (e.g., "5s", "100ms", "1m"). Defaults to "30s". Mutually exclusive with `cron`.
- `cron` (str): Cron expression — instead of polling, the framework schedules the `on` workflow on this expression and the function body is unused. Mutually exclusive with `poll_interval`.
- `timezone` (str): Timezone for the `cron` schedule.
- `allow_concurrent` (bool): Whether to allow concurrent executions of the same trigger. Defaults to `False`

Setting both `cron` and `poll_interval`, or `cron` without `on`, raises
`ValueError`.

{{< hint type="note" title="Scaffolding a cron package" >}}
`cloacinactl package new <name> --language python --kind cron` scaffolds a
ready-to-pack package with this shape: one bare `@cloaca.task` plus a
`@cloaca.trigger(on=..., cron=...)` declaration
(`crates/cloacinactl/src/nouns/package/new.rs`). A worked example lives at
`examples/features/workflows/python-cron`.
{{< /hint >}}

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
    on="file_processor",
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

## Typed event surface (manual fire)

A trigger can be fired manually, pushing a **typed event** that fans out to
every workflow subscribed to the trigger's name. The Python `@cloaca.trigger`
decorator does **not** take a `params(...)` argument — there is no per-decorator
typed-event declaration on the Python side. Instead, the trigger's event surface
is derived: it is the **union of the declared params of every subscribed
workflow** (each declared via `@cloaca.workflow_params(...)` /
`#[workflow(params(...))]`).

This surface is exposed and enforced by the server, not by the decorator:

- `GET /v1/tenants/{tenant}/triggers/{name}/interface` returns the typed slots
  (the union of subscribers' declared params). The web UI builds a typed
  "fire" form from it.
- `POST /v1/tenants/{tenant}/triggers/{name}/fire` validates the event body
  against that surface, then executes **every** subscribed workflow with the
  event merged into context (fan-out). See
  [Packaged Triggers]({{< ref "/embed/tutorials/14-packaged-triggers" >}}) for
  the trigger authoring flow.

In other words: to type a trigger's manual-fire event, declare params on the
workflows it fires — the trigger inherits their union. (CLOACI-T-0777.)

## Deduplication

When `allow_concurrent=False` (the default), the trigger scheduler prevents duplicate executions:

1. Context is hashed when `TriggerResult.fire()` is returned
2. Active executions are tracked by (trigger_name, context_hash)
3. If an execution with the same hash is running, the trigger skips

```python
@cloaca.trigger(
    on="order_processor",
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
    on="queue_worker",
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
    on="service_recovery",
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
    on="scale_up",
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
@cloaca.trigger(on="processor", poll_interval="5s")
def good_trigger():
    if file_exists("/inbox/trigger.flag"):
        return cloaca.TriggerResult.fire()
    return cloaca.TriggerResult.skip()

# Bad: Heavy processing in poll
@cloaca.trigger(on="processor", poll_interval="5s")
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
@cloaca.trigger(on="data_sync", poll_interval="1m")
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
runner = cloaca.DefaultRunner("sqlite:///workflows.db")

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

- **[Context]({{< ref "/reference/python-api/context/" >}})** - Data passed from triggers to workflows
- **[WorkflowBuilder]({{< ref "/reference/python-api/workflow-builder/" >}})** - Define workflows that triggers activate
- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** - Execute workflows and manage triggers
- **[Tutorial: Event Triggers]({{< ref "/embed/tutorials/07-event-triggers/" >}})** - Step-by-step trigger implementation guide
