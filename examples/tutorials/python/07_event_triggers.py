#!/usr/bin/env python3
"""
Cloaca Tutorial 07: Event Triggers

This tutorial demonstrates how to manage event-based workflow triggers using
the Python bindings. Event triggers poll user-defined conditions and fire
workflows when those conditions are met.

Note: Trigger definitions are currently created in Rust. This tutorial shows
how to manage triggers through the Python API, including:
- Listing trigger schedules
- Enabling/disabling triggers
- Viewing trigger execution history
- Understanding trigger-workflow relationships

For defining custom triggers, see the Rust event-triggers example.
"""

import cloaca
from datetime import datetime


# Create simple workflows that could be triggered by events
with cloaca.WorkflowBuilder("file_processor") as builder:
    builder.description("Process incoming files")

    @cloaca.task(id="process_file")
    def process_file(context):
        """Process an incoming file."""
        filename = context.get("filename", "unknown")
        print(f"Processing file: {filename}")
        context.set("processed_at", datetime.now().isoformat())
        return context


with cloaca.WorkflowBuilder("queue_handler") as builder:
    builder.description("Handle queue overflow")

    @cloaca.task(id="drain_queue")
    def drain_queue(context):
        """Drain and process queue messages."""
        queue_depth = context.get("queue_depth", 0)
        print(f"Draining {queue_depth} messages from queue")
        context.set("messages_processed", queue_depth)
        return context


with cloaca.WorkflowBuilder("alert_handler") as builder:
    builder.description("Handle system alerts")

    @cloaca.task(id="send_alert")
    def send_alert(context):
        """Send an alert notification."""
        alert_type = context.get("alert_type", "info")
        message = context.get("message", "System notification")
        print(f"[{alert_type.upper()}] {message}")
        context.set("alert_sent_at", datetime.now().isoformat())
        return context


def trigger_management_demo():
    """Demonstrate trigger management through Python API."""
    print("\n=== Event Triggers Management Demo ===\n")

    # Create runner
    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Note: In a real scenario, triggers would be registered from Rust code
        # or through a trigger registration API. The Python bindings provide
        # management capabilities for existing triggers.

        print("Event triggers enable condition-based workflow execution.")
        print("Unlike cron (time-based), triggers poll custom conditions.\n")

        # List all trigger schedules
        print("1. Listing Trigger Schedules")
        print("-" * 40)

        schedules = runner.list_trigger_schedules()
        if schedules:
            for schedule in schedules:
                print(f"  Trigger: {schedule['trigger_name']}")
                print(f"  Workflow: {schedule['workflow_name']}")
                print(f"  Poll Interval: {schedule['poll_interval_ms']}ms")
                print(f"  Enabled: {schedule['enabled']}")
                print(f"  Allow Concurrent: {schedule['allow_concurrent']}")
                print()
        else:
            print("  No triggers registered (triggers are defined in Rust)")
            print("  See: examples/features/event-triggers/ for Rust examples")
        print()

        # Demonstrate trigger management API
        print("2. Trigger Management API")
        print("-" * 40)
        print("Available methods:")
        print("  - list_trigger_schedules(enabled_only=False, limit=100, offset=0)")
        print("  - get_trigger_schedule(trigger_name)")
        print("  - set_trigger_enabled(trigger_name, enabled)")
        print("  - get_trigger_execution_history(trigger_name, limit=100, offset=0)")
        print()

        # Show how to query a specific trigger
        print("3. Querying Trigger Status")
        print("-" * 40)
        print("Example: runner.get_trigger_schedule('file_watcher')")
        print("Returns: Trigger configuration including poll interval,")
        print("         workflow association, and enabled status")
        print()

        # Show execution history query
        print("4. Trigger Execution History")
        print("-" * 40)
        print("Example: runner.get_trigger_execution_history('file_watcher')")
        print("Returns: List of executions with timestamps, context hash,")
        print("         and linked pipeline execution IDs")
        print()

        # Demonstrate enable/disable
        print("5. Enabling/Disabling Triggers")
        print("-" * 40)
        print("Example: runner.set_trigger_enabled('file_watcher', False)")
        print("Disabled triggers stop polling but retain configuration")
        print("for later re-enablement.")
        print()

    finally:
        runner.shutdown()


def trigger_concepts():
    """Explain key trigger concepts."""
    print("\n=== Event Trigger Concepts ===\n")

    concepts = [
        ("Trigger Trait", """
        Triggers implement the Trigger trait in Rust:
        - name(): Unique identifier for the trigger
        - poll_interval(): How often to check the condition
        - allow_concurrent(): Whether to allow concurrent executions
        - poll(): Check condition, return Fire or Skip
        """),

        ("TriggerResult", """
        The poll() function returns:
        - TriggerResult::Skip - Condition not met, continue polling
        - TriggerResult::Fire(Some(context)) - Fire workflow with context
        - TriggerResult::Fire(None) - Fire workflow without context
        """),

        ("Deduplication", """
        Triggers prevent duplicate executions by:
        - Hashing the context passed with TriggerResult::Fire
        - Tracking active executions per (trigger, context_hash)
        - allow_concurrent=false blocks duplicates until completion
        """),

        ("Polling vs Cron", """
        Event Triggers vs Cron Scheduling:
        - Triggers: Poll custom conditions, fire when true
        - Cron: Fire at specific times regardless of conditions
        - Both can be used together for comprehensive scheduling
        """),
    ]

    for title, description in concepts:
        print(f"{title}")
        print("-" * 40)
        for line in description.strip().split('\n'):
            print(f"  {line.strip()}")
        print()


def trigger_examples():
    """Show common trigger patterns."""
    print("\n=== Common Trigger Patterns ===\n")

    patterns = [
        ("File Watcher", """
        Poll for new files in a directory:
        - Check directory for unprocessed files
        - Fire with filename in context
        - Mark files as processing to prevent duplicates
        """),

        ("Queue Monitor", """
        Fire when queue depth exceeds threshold:
        - Query message queue depth
        - Fire when depth > threshold
        - Pass queue depth in context for processing
        """),

        ("Health Check", """
        Trigger recovery after consecutive failures:
        - Check service health endpoint
        - Track consecutive failures
        - Fire recovery workflow after N failures
        """),

        ("Database Poller", """
        Process new records as they appear:
        - Query for records with status='pending'
        - Fire with record IDs in context
        - Use deduplication to avoid reprocessing
        """),

        ("Metrics Threshold", """
        Alert when metrics exceed limits:
        - Poll metrics endpoint
        - Check against configured thresholds
        - Fire alert workflow with metric data
        """),
    ]

    for pattern_name, description in patterns:
        print(f"{pattern_name}")
        print("-" * 40)
        for line in description.strip().split('\n'):
            print(f"  {line.strip()}")
        print()


def main():
    """Main tutorial demonstration."""
    print("Cloaca Event Triggers Tutorial")
    print("=" * 50)

    # Explain concepts
    trigger_concepts()

    # Show common patterns
    trigger_examples()

    # Demonstrate management API
    trigger_management_demo()

    print("\nTutorial completed successfully!")
    print("\nWhat you learned:")
    print("- Event triggers poll conditions and fire workflows")
    print("- Triggers are defined in Rust, managed via Python API")
    print("- Deduplication prevents duplicate concurrent executions")
    print("- Common patterns: file watching, queue monitoring, health checks")

    print("\nNext Steps:")
    print("- See examples/features/event-triggers/ for Rust trigger examples")
    print("- Explore the Trigger trait for custom implementations")
    print("- Combine triggers with cron for comprehensive scheduling")


if __name__ == "__main__":
    main()
