#!/usr/bin/env python3
"""
Cloaca Tutorial 08: Packaged Triggers

This tutorial demonstrates how triggers work in packaged workflows:

1. Defining triggers alongside tasks using @cloaca.trigger
2. How triggers are declared in ManifestV2 for .cloacina packages
3. The relationship between decorator registration and manifest declaration

When a Python workflow is packaged as a .cloacina archive, triggers are:
- Defined in code via @cloaca.trigger (provides the poll implementation)
- Declared in manifest.json (tells the reconciler about them)
- Auto-registered when the package is loaded
"""

import cloaca
from datetime import datetime


# =============================================================================
# Part 1: Define a workflow with tasks
# =============================================================================

with cloaca.WorkflowBuilder("data_ingest") as builder:
    builder.description("Ingest data files detected by trigger")

    @cloaca.task(id="validate")
    def validate(context):
        """Validate the incoming data file."""
        filename = context.get("filename", "unknown")
        print(f"  Validating: {filename}")
        context.set("valid", True)
        return context

    @cloaca.task(id="load", dependencies=["validate"])
    def load(context):
        """Load validated data into the warehouse."""
        filename = context.get("filename", "unknown")
        print(f"  Loading: {filename}")
        context.set("loaded_at", datetime.now().isoformat())
        return context


# =============================================================================
# Part 2: Define triggers that fire the workflow
# =============================================================================

# Simulated state
poll_count = 0


@cloaca.trigger(
    name="inbox_watcher",
    poll_interval="5s",
    allow_concurrent=False
)
def inbox_watcher():
    """
    Poll for new files in the inbox directory.

    In a .cloacina package, this trigger would also be declared in
    manifest.json as:

        {
            "name": "inbox_watcher",
            "trigger_type": "python",
            "workflow": "data_ingest",
            "poll_interval": "5s",
            "allow_concurrent": false,
            "config": { "path": "/data/inbox/" }
        }

    The manifest declaration tells the reconciler:
    - Which workflow to fire when the trigger activates
    - The poll interval and concurrency settings
    - Optional config for the trigger implementation

    The @cloaca.trigger decorator provides the actual poll() implementation.
    Both must agree on the trigger name.
    """
    global poll_count
    poll_count += 1

    if poll_count % 3 == 0:
        filename = f"batch_{datetime.now().strftime('%H%M%S')}.parquet"
        print(f"  [TRIGGER] New file detected: {filename}")
        ctx = cloaca.Context({"filename": filename})
        return cloaca.TriggerResult.fire(ctx)

    return cloaca.TriggerResult.skip()


# =============================================================================
# Part 3: Demonstrate the trigger
# =============================================================================

def demo_trigger_polls():
    """Show how trigger polling works."""
    print("\n" + "=" * 60)
    print("Part 1: Trigger Poll Simulation")
    print("=" * 60)

    print("\nSimulating inbox_watcher polls:")
    print("-" * 40)

    for i in range(6):
        result = inbox_watcher()
        if result.is_fire_result():
            print(f"  Poll {i+1}: FIRE -> workflow 'data_ingest' would execute")
        else:
            print(f"  Poll {i+1}: SKIP")


def demo_workflow_execution():
    """Run the workflow as if triggered."""
    print("\n" + "=" * 60)
    print("Part 2: Triggered Workflow Execution")
    print("=" * 60)

    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Simulate what happens when the trigger fires
        print("\nTrigger fired! Executing 'data_ingest' workflow...")
        print("-" * 40)

        context = cloaca.Context({
            "filename": "orders_20260328.parquet",
            "trigger_name": "inbox_watcher",
            "triggered_at": datetime.now().isoformat(),
        })
        result = runner.execute("data_ingest", context)

        print("-" * 40)
        print(f"Workflow status: {result.status}")

    finally:
        runner.shutdown()


def demo_manifest_explanation():
    """Explain the ManifestV2 trigger fields."""
    print("\n" + "=" * 60)
    print("Part 3: ManifestV2 Trigger Declaration")
    print("=" * 60)

    print("""
When packaging a Python workflow as a .cloacina archive, triggers
are declared in manifest.json alongside tasks:

    {
        "format_version": "2",
        "package": { "name": "data_ingest", ... },
        "language": "python",
        "tasks": [ ... ],
        "triggers": [
            {
                "name": "inbox_watcher",
                "trigger_type": "python",
                "workflow": "data_ingest",
                "poll_interval": "5s",
                "allow_concurrent": false,
                "config": { "path": "/data/inbox/" }
            }
        ]
    }

Field reference:
  name             Unique trigger name (must match @cloaca.trigger name)
  trigger_type     "python" for @cloaca.trigger, "rust" for #[trigger]
  workflow         Which workflow to fire (package name or task ID)
  poll_interval    Duration string: "100ms", "5s", "2m", "1h"
  allow_concurrent Allow parallel runs with same context hash
  config           Optional JSON for trigger-specific settings
    """)

    print("The reconciler uses both the manifest and the decorator:")
    print("  1. Package is loaded -> Python module imported")
    print("  2. @cloaca.trigger decorators fire -> triggers registered")
    print("  3. Reconciler reads manifest -> verifies triggers exist")
    print("  4. TriggerScheduler polls registered triggers")
    print("  5. When a trigger fires -> associated workflow executes")


def main():
    """Main tutorial."""
    print("Cloaca Tutorial 08: Packaged Triggers")
    print("=" * 60)

    demo_trigger_polls()
    demo_workflow_execution()
    demo_manifest_explanation()

    print("\n" + "=" * 60)
    print("Tutorial Complete!")
    print("=" * 60)
    print("\nWhat you learned:")
    print("  - @cloaca.trigger provides the poll implementation")
    print("  - ManifestV2 'triggers' field declares triggers for packages")
    print("  - Both must agree on the trigger name")
    print("  - The reconciler wires them together on package load")
    print("\nSee also:")
    print("  - Tutorial 07: Event triggers basics")
    print("  - examples/features/packaged-triggers/: Rust packaged trigger")


if __name__ == "__main__":
    main()
