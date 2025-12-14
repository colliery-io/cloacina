---
title: "07 - Event Triggers"
description: "Define event-driven workflow triggers and task callbacks for monitoring"
weight: 17
reviewer: "automation"
review_date: "2025-12-13"
---

# Event Triggers and Task Callbacks

In this tutorial, you'll learn how to create event-driven workflows using triggers and implement task callbacks for monitoring and alerting. While cron scheduling (Tutorial 5) handles time-based execution, triggers let you fire workflows based on custom conditions like file arrivals, queue depths, or API state changes.

## Learning Objectives

- Define custom triggers with the `@trigger` decorator
- Control trigger behavior with `TriggerResult.fire()` and `TriggerResult.skip()`
- Implement `on_success` and `on_failure` task callbacks
- Manage triggers through the runner API
- Understand deduplication and concurrent execution control

## Prerequisites

- Completion of [Tutorial 5]({{< ref "/python-bindings/tutorials/05-cron-scheduling/" >}})
- Understanding of workflow execution basics
- Familiarity with Python decorators

## Time Estimate
20-25 minutes

## Triggers vs Cron Scheduling

{{< tabs "trigger-vs-cron" >}}
{{< tab "Triggers" >}}
**Event-driven execution:**
- Poll custom conditions at intervals
- Fire when your condition returns true
- Pass dynamic context to workflows
- Support deduplication by context hash
{{< /tab >}}

{{< tab "Cron" >}}
**Time-based execution:**
- Fire at specific times (cron expressions)
- No condition checking
- Static scheduling
- Time-based deduplication
{{< /tab >}}

{{< tab "Combined" >}}
**Use both for comprehensive scheduling:**
- Cron for regular maintenance jobs
- Triggers for reactive processing
- Same workflow can have both
- Complementary approaches
{{< /tab >}}
{{< /tabs >}}

## Defining Triggers

### Basic Trigger Structure

```python
import cloaca

@cloaca.trigger(
    workflow="my_workflow",      # Workflow to trigger
    name="my_trigger",           # Optional: defaults to function name
    poll_interval="5s",          # How often to poll
    allow_concurrent=False       # Prevent duplicate executions
)
def my_trigger():
    """Poll for a condition and fire when met."""
    if some_condition():
        ctx = cloaca.Context({"key": "value"})
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

### TriggerResult

The trigger function must return a `TriggerResult`:

| Method | Description |
|--------|-------------|
| `TriggerResult.skip()` | Condition not met, continue polling |
| `TriggerResult.fire(context)` | Condition met, execute workflow with context |

### Poll Interval Format

The `poll_interval` parameter accepts duration strings:
- `"100ms"` - 100 milliseconds
- `"5s"` - 5 seconds
- `"1m"` - 1 minute
- `"1h"` - 1 hour

## File Watcher Example

A common use case is watching for new files:

```python
import cloaca
import os
from datetime import datetime

# Track processed files to avoid duplicates
processed_files = set()

@cloaca.trigger(
    workflow="file_processor",
    name="file_watcher",
    poll_interval="2s",
    allow_concurrent=False
)
def file_watcher():
    """Watch for new files in inbox directory."""
    inbox_dir = "/path/to/inbox"

    for filename in os.listdir(inbox_dir):
        filepath = os.path.join(inbox_dir, filename)

        # Skip already processed files
        if filepath in processed_files:
            continue

        # Skip non-files
        if not os.path.isfile(filepath):
            continue

        # Found a new file - fire the workflow
        processed_files.add(filepath)
        ctx = cloaca.Context({
            "filepath": filepath,
            "filename": filename,
            "detected_at": datetime.now().isoformat()
        })
        return cloaca.TriggerResult.fire(ctx)

    # No new files found
    return cloaca.TriggerResult.skip()

# Define the workflow to process files
with cloaca.WorkflowBuilder("file_processor") as builder:
    builder.description("Process incoming files")

    @cloaca.task(id="process_file")
    def process_file(context):
        filepath = context.get("filepath")
        filename = context.get("filename")

        print(f"Processing file: {filename}")
        # Process the file...

        context.set("processed", True)
        return context
```

## Queue Depth Monitor

Another common pattern is monitoring queue depths:

```python
import cloaca
import random  # Simulating queue depth check

@cloaca.trigger(
    workflow="queue_handler",
    poll_interval="5s",
    allow_concurrent=True  # Allow parallel processing
)
def queue_depth_trigger():
    """Fire when queue depth exceeds threshold."""
    # In real code, check actual queue (Redis, RabbitMQ, etc.)
    queue_depth = get_queue_depth()  # Your queue check function
    threshold = 100

    if queue_depth > threshold:
        ctx = cloaca.Context({
            "queue_depth": queue_depth,
            "threshold": threshold
        })
        return cloaca.TriggerResult.fire(ctx)

    return cloaca.TriggerResult.skip()

# Workflow for queue processing
with cloaca.WorkflowBuilder("queue_handler") as builder:
    builder.description("Handle queue overflow")

    @cloaca.task(id="drain_queue")
    def drain_queue(context):
        depth = context.get("queue_depth", 0)
        print(f"Draining {depth} messages from queue")
        # Process queue messages...
        context.set("messages_processed", depth)
        return context
```

## Deduplication

When `allow_concurrent=False`, Cloacina prevents duplicate workflow executions:

1. When `TriggerResult.fire(ctx)` is called, the context is hashed
2. If another execution with the same (trigger_name, context_hash) is running, the trigger skips
3. This prevents processing the same item twice

```python
@cloaca.trigger(
    workflow="order_processor",
    poll_interval="1s",
    allow_concurrent=False  # Deduplicate by context
)
def order_trigger():
    order = get_next_pending_order()
    if order:
        # Context includes order_id, so same order won't be processed twice
        ctx = cloaca.Context({"order_id": order.id})
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

## Task Callbacks

Task callbacks let you monitor task execution for alerting, logging, or cleanup.

### Callback Signatures

```python
def on_success_callback(task_id: str, context: cloaca.Context) -> None:
    """Called when task completes successfully."""
    pass

def on_failure_callback(task_id: str, error: str, context: cloaca.Context) -> None:
    """Called when task fails."""
    pass
```

### Adding Callbacks to Tasks

```python
import cloaca

def log_success(task_id, context):
    """Log successful task completion."""
    print(f"Task '{task_id}' completed successfully")

def alert_failure(task_id, error, context):
    """Alert on task failure."""
    print(f"ALERT: Task '{task_id}' failed: {error}")
    # Send to Slack, PagerDuty, etc.

with cloaca.WorkflowBuilder("monitored_workflow") as builder:
    builder.description("Workflow with monitoring callbacks")

    @cloaca.task(
        id="critical_task",
        on_success=log_success,
        on_failure=alert_failure
    )
    def critical_task(context):
        # Task implementation
        result = perform_critical_operation()
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
    context.set("result", "success")
    return context
```

The task will complete successfully, and the callback error will be logged to stderr.

### Common Callback Patterns

{{< tabs "callback-patterns" >}}
{{< tab "Alerting" >}}
```python
import requests
import os

def slack_alert(task_id, error, context):
    """Send failure alerts to Slack."""
    webhook_url = os.environ.get("SLACK_WEBHOOK")
    if webhook_url:
        requests.post(webhook_url, json={
            "text": f"Task {task_id} failed: {error}"
        })
```
{{< /tab >}}

{{< tab "Metrics" >}}
```python
def record_metrics(task_id, context):
    """Record task metrics."""
    duration = context.get("duration_ms", 0)
    # statsd.timing(f"task.{task_id}.duration", duration)
    print(f"Task {task_id} completed in {duration}ms")
```
{{< /tab >}}

{{< tab "Cleanup" >}}
```python
import shutil
import os

def cleanup_temp_files(task_id, error, context):
    """Clean up temporary files on failure."""
    temp_dir = context.get("temp_dir")
    if temp_dir and os.path.exists(temp_dir):
        shutil.rmtree(temp_dir)
        print(f"Cleaned up temp directory: {temp_dir}")
```
{{< /tab >}}
{{< /tabs >}}

## Managing Triggers via Runner API

The `DefaultRunner` provides methods to manage triggers:

```python
import cloaca

runner = cloaca.DefaultRunner(":memory:")

# List all trigger schedules
schedules = runner.list_trigger_schedules()
for schedule in schedules:
    print(f"Trigger: {schedule['trigger_name']} -> {schedule['workflow_name']}")

# Get specific trigger details
schedule = runner.get_trigger_schedule("file_watcher")
print(f"Poll interval: {schedule['poll_interval_ms']}ms")
print(f"Enabled: {schedule['enabled']}")

# Enable/disable a trigger
runner.set_trigger_enabled("file_watcher", False)  # Disable
runner.set_trigger_enabled("file_watcher", True)   # Re-enable

# Get execution history
history = runner.get_trigger_execution_history("file_watcher", limit=10)
for execution in history:
    print(f"Executed at: {execution['started_at']}")

runner.shutdown()
```

## Complete Example

Here's a complete example combining triggers and callbacks:

```python
import cloaca
from datetime import datetime

# =============================================================================
# Callbacks for monitoring
# =============================================================================

def on_task_success(task_id, context):
    """Log successful task completion."""
    print(f"  [SUCCESS] Task '{task_id}' completed")

def on_task_failure(task_id, error, context):
    """Alert on task failure."""
    print(f"  [FAILURE] Task '{task_id}' failed: {error}")

# =============================================================================
# Workflow with monitored tasks
# =============================================================================

with cloaca.WorkflowBuilder("file_processor") as builder:
    builder.description("Process incoming files with monitoring")

    @cloaca.task(
        id="validate_file",
        on_success=on_task_success,
        on_failure=on_task_failure
    )
    def validate_file(context):
        """Validate an incoming file."""
        filename = context.get("filename", "unknown")
        print(f"  Validating file: {filename}")
        context.set("validated", True)
        return context

    @cloaca.task(
        id="process_file",
        dependencies=["validate_file"],
        on_success=on_task_success,
        on_failure=on_task_failure
    )
    def process_file(context):
        """Process the validated file."""
        filename = context.get("filename", "unknown")
        print(f"  Processing file: {filename}")
        context.set("processed_at", datetime.now().isoformat())
        return context

# =============================================================================
# Trigger to watch for files
# =============================================================================

file_counter = 0  # Simulated state

@cloaca.trigger(
    workflow="file_processor",
    name="file_watcher",
    poll_interval="2s",
    allow_concurrent=False
)
def file_watcher():
    """Poll for new files."""
    global file_counter
    file_counter += 1

    # Simulate finding a new file every 5th poll
    if file_counter % 5 == 0:
        filename = f"data_{datetime.now().strftime('%H%M%S')}.csv"
        print(f"  [TRIGGER] Found new file: {filename}")
        ctx = cloaca.Context({"filename": filename})
        return cloaca.TriggerResult.fire(ctx)

    return cloaca.TriggerResult.skip()

# =============================================================================
# Run demonstration
# =============================================================================

def main():
    print("Event Triggers and Task Callbacks Demo")
    print("=" * 50)

    runner = cloaca.DefaultRunner(":memory:")

    # Execute workflow manually to demonstrate callbacks
    print("\nManual execution with callbacks:")
    print("-" * 40)

    context = cloaca.Context({"filename": "report_2024.csv"})
    result = runner.execute("file_processor", context)

    print("-" * 40)
    print(f"Workflow status: {result.status}")

    # Demonstrate trigger polling
    print("\nSimulating trigger polls:")
    print("-" * 40)

    for i in range(7):
        result = file_watcher()
        if result.is_fire_result():
            print(f"  Poll {i+1}: FIRE - workflow will execute")
        else:
            print(f"  Poll {i+1}: SKIP - waiting...")

    runner.shutdown()
    print("\nDemo complete!")

if __name__ == "__main__":
    main()
```

## Running the Example

Save the code above as `event_triggers_demo.py` and run:

```bash
python event_triggers_demo.py
```

Expected output:
```
Event Triggers and Task Callbacks Demo
==================================================

Manual execution with callbacks:
----------------------------------------
  Validating file: report_2024.csv
  [SUCCESS] Task 'validate_file' completed
  Processing file: report_2024.csv
  [SUCCESS] Task 'process_file' completed
----------------------------------------
Workflow status: Completed

Simulating trigger polls:
----------------------------------------
  Poll 1: SKIP - waiting...
  Poll 2: SKIP - waiting...
  Poll 3: SKIP - waiting...
  Poll 4: SKIP - waiting...
  [TRIGGER] Found new file: data_143052.csv
  Poll 5: FIRE - workflow will execute
  Poll 6: SKIP - waiting...
  Poll 7: SKIP - waiting...

Demo complete!
```

## What You've Learned

In this tutorial, you learned:

- **Trigger definition** with the `@trigger` decorator
- **TriggerResult** for controlling when workflows fire
- **Deduplication** with `allow_concurrent` parameter
- **Task callbacks** for monitoring with `on_success` and `on_failure`
- **Callback error isolation** ensuring task reliability
- **Trigger management** through the runner API

## Next Steps

{{< button relref="/python-bindings/api-reference/trigger/" >}}Trigger API Reference{{< /button >}}
{{< button relref="/python-bindings/api-reference/task/" >}}Task API Reference{{< /button >}}
{{< button relref="/python-bindings/api-reference/runner/" >}}Runner API Reference{{< /button >}}

## Related Resources

- [Explanation: Trigger Rules]({{< ref "/explanation/trigger-rules/" >}}) - Deep dive into trigger architecture
- [Tutorial 05: Cron Scheduling]({{< ref "/python-bindings/tutorials/05-cron-scheduling/" >}}) - Time-based scheduling
- [API Reference: Task Decorator]({{< ref "/python-bindings/api-reference/task/" >}}) - Complete task options

{{< hint type="info" title="Reference Implementation" >}}
This tutorial is based on the example at [`examples/tutorials/python/07_event_triggers.py`](https://github.com/colliery-io/cloacina/tree/main/examples/tutorials/python/07_event_triggers.py).
{{< /hint >}}
