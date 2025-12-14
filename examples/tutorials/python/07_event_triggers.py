#!/usr/bin/env python3
"""
Cloaca Tutorial 07: Event Triggers and Task Callbacks

This tutorial demonstrates:
1. Defining event-based workflow triggers using the @trigger decorator
2. Using task callbacks (on_success, on_failure) for monitoring
3. Managing triggers through the Python API

Event triggers poll user-defined conditions and fire workflows when
those conditions are met, unlike cron scheduling which is time-based.
"""

import cloaca
from datetime import datetime
import random


# =============================================================================
# Part 1: Task Callbacks
# =============================================================================

def on_task_success(task_id, context):
    """Callback called when a task completes successfully."""
    print(f"  [SUCCESS] Task '{task_id}' completed successfully")


def on_task_failure(task_id, error, context):
    """Callback called when a task fails."""
    print(f"  [FAILURE] Task '{task_id}' failed: {error}")


# Create a workflow with callbacks
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
# Part 2: Defining Triggers with @trigger Decorator
# =============================================================================

# Simulated state for demo purposes
file_counter = 0


@cloaca.trigger(
    workflow="file_processor",
    name="file_watcher",
    poll_interval="2s",
    allow_concurrent=False
)
def file_watcher():
    """
    Poll for new files in a directory.

    This trigger fires when a new file is detected, passing the
    filename to the workflow via context.
    """
    global file_counter
    file_counter += 1

    # Simulate finding a new file every 5th poll
    if file_counter % 5 == 0:
        filename = f"data_{datetime.now().strftime('%H%M%S')}.csv"
        print(f"  [TRIGGER] Found new file: {filename}")
        ctx = cloaca.Context({"filename": filename})
        return cloaca.TriggerResult.fire(ctx)

    return cloaca.TriggerResult.skip()


# Another workflow for queue processing
with cloaca.WorkflowBuilder("queue_handler") as builder:
    builder.description("Handle queue overflow")

    @cloaca.task(id="drain_queue")
    def drain_queue(context):
        """Drain and process queue messages."""
        queue_depth = context.get("queue_depth", 0)
        print(f"  Draining {queue_depth} messages from queue")
        context.set("messages_processed", queue_depth)
        return context


@cloaca.trigger(
    workflow="queue_handler",
    poll_interval="5s",
    allow_concurrent=True  # Allow parallel queue draining
)
def queue_depth_trigger():
    """
    Fire when queue depth exceeds threshold.

    With allow_concurrent=True, multiple workflow executions can
    run in parallel for better throughput.
    """
    # Simulate queue depth check
    queue_depth = random.randint(0, 150)

    if queue_depth > 100:
        print(f"  [TRIGGER] Queue depth {queue_depth} exceeds threshold")
        ctx = cloaca.Context({"queue_depth": queue_depth})
        return cloaca.TriggerResult.fire(ctx)

    return cloaca.TriggerResult.skip()


# =============================================================================
# Part 3: Demonstrations
# =============================================================================

def demo_callbacks():
    """Demonstrate task callbacks."""
    print("\n" + "=" * 60)
    print("Part 1: Task Callbacks Demo")
    print("=" * 60)

    runner = cloaca.DefaultRunner(":memory:")

    try:
        print("\nExecuting workflow with on_success/on_failure callbacks...")
        print("-" * 40)

        context = cloaca.Context({"filename": "report_2024.csv"})
        result = runner.execute("file_processor", context)

        print("-" * 40)
        print(f"Workflow status: {result.status}")

    finally:
        runner.shutdown()


def demo_trigger_definition():
    """Demonstrate trigger definition and TriggerResult usage."""
    print("\n" + "=" * 60)
    print("Part 2: Trigger Definition Demo")
    print("=" * 60)

    print("\nTriggers are defined using the @trigger decorator:")
    print("""
    @cloaca.trigger(
        workflow="my_workflow",    # Workflow to trigger
        name="my_trigger",         # Optional: defaults to function name
        poll_interval="5s",        # How often to poll (e.g., "5s", "100ms", "1m")
        allow_concurrent=False     # Prevent duplicate executions
    )
    def my_trigger():
        if some_condition():
            ctx = cloaca.Context({"key": "value"})
            return cloaca.TriggerResult.fire(ctx)
        return cloaca.TriggerResult.skip()
    """)

    print("\nTriggerResult has two methods:")
    print("  - TriggerResult.skip()       - Condition not met, continue polling")
    print("  - TriggerResult.fire(ctx)    - Condition met, trigger the workflow")

    print("\nSimulating trigger polls:")
    print("-" * 40)

    # Simulate a few polls
    for i in range(7):
        result = file_watcher()
        if result.is_fire_result():
            print(f"  Poll {i+1}: FIRE - workflow will execute")
        else:
            print(f"  Poll {i+1}: SKIP - waiting...")


def demo_trigger_management():
    """Demonstrate trigger management through Python API."""
    print("\n" + "=" * 60)
    print("Part 3: Trigger Management API")
    print("=" * 60)

    runner = cloaca.DefaultRunner(":memory:")

    try:
        print("\nAvailable trigger management methods:")
        print("-" * 40)
        print("  runner.list_trigger_schedules()")
        print("  runner.get_trigger_schedule('trigger_name')")
        print("  runner.set_trigger_enabled('trigger_name', False)")
        print("  runner.get_trigger_execution_history('trigger_name')")

        print("\nListing registered triggers...")
        schedules = runner.list_trigger_schedules()
        if schedules:
            for schedule in schedules:
                print(f"  - {schedule['trigger_name']} -> {schedule['workflow_name']}")
        else:
            print("  (No triggers registered in this demo database)")

    finally:
        runner.shutdown()


def demo_concepts():
    """Explain key concepts."""
    print("\n" + "=" * 60)
    print("Key Concepts")
    print("=" * 60)

    concepts = [
        ("Triggers vs Cron", """
        - Triggers: Poll custom conditions, fire when true
        - Cron: Fire at specific times regardless of conditions
        - Use together for comprehensive scheduling
        """),

        ("Deduplication", """
        When allow_concurrent=False:
        - Context is hashed on TriggerResult.fire()
        - Same (trigger_name, context_hash) won't run twice
        - Prevents duplicate processing of same item
        """),

        ("Callbacks", """
        Task callbacks for monitoring:
        - on_success(task_id, context) - called on success
        - on_failure(task_id, error, context) - called on failure
        - Errors in callbacks are isolated (don't fail the task)
        """),
    ]

    for title, description in concepts:
        print(f"\n{title}")
        print("-" * 40)
        for line in description.strip().split('\n'):
            print(f"  {line.strip()}")


def main():
    """Main tutorial demonstration."""
    print("Cloaca Tutorial 07: Event Triggers and Task Callbacks")
    print("=" * 60)

    # Demonstrate callbacks
    demo_callbacks()

    # Demonstrate trigger definition
    demo_trigger_definition()

    # Demonstrate trigger management
    demo_trigger_management()

    # Explain concepts
    demo_concepts()

    print("\n" + "=" * 60)
    print("Tutorial Complete!")
    print("=" * 60)
    print("\nWhat you learned:")
    print("  - Define triggers with @cloaca.trigger decorator")
    print("  - Use TriggerResult.fire() and TriggerResult.skip()")
    print("  - Add on_success/on_failure callbacks to tasks")
    print("  - Manage triggers via the runner API")
    print("\nNext Steps:")
    print("  - See examples/features/event-triggers/ for more examples")
    print("  - Combine triggers with cron for comprehensive scheduling")


if __name__ == "__main__":
    main()
