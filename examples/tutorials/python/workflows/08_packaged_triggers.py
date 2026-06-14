#!/usr/bin/env python3
"""
Cloaca Tutorial 08: Packaged Triggers

This tutorial demonstrates how triggers work in packaged workflows:

1. Defining triggers alongside tasks using @cloaca.trigger
2. How a .cloacina package is described by package.toml (no trigger section)
3. Why the @cloaca.trigger decorator *is* the declaration

When a Python workflow is packaged as a .cloacina archive, triggers are:
- Defined in code via @cloaca.trigger (provides the poll implementation AND the
  declaration — name, poll interval, concurrency)
- Auto-registered when the reconciler imports the package's entry_module

There is no `triggers` array in package.toml. The decorator running at import
time is the whole story.
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

    In a .cloacina package there is nothing else to declare: the
    @cloaca.trigger decorator above carries the name, poll interval, and
    concurrency setting. When the reconciler imports this module, the
    decorator registers the trigger and binds it to the `data_ingest`
    workflow built in the same module.

    package.toml has no `triggers` section — adding one (or a `package_type`
    key) is rejected at upload. The decorator is the declaration.
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


def demo_package_explanation():
    """Explain how a trigger-bearing workflow is packaged."""
    print("\n" + "=" * 60)
    print("Part 3: Packaging — package.toml has no trigger section")
    print("=" * 60)

    print("""
A .cloacina package is a top-level package.toml plus your module tree
under workflow/. The manifest declares package identity + [metadata]
only — there is NO triggers array:

    package.toml
    workflow/
        data_ingest/
            __init__.py
            tasks.py        # @cloaca.task + @cloaca.trigger live here

    # package.toml
    [package]
    name = "data-ingest"
    version = "1.0.0"
    interface = "cloacina-workflow-plugin"
    interface_version = 1
    extension = "cloacina"

    [metadata]
    language = "python"
    workflow_name = "data_ingest"
    entry_module = "data_ingest.tasks"
    requires_python = ">=3.10"

The @cloaca.trigger decorator IS the declaration — name, poll_interval,
and allow_concurrent all live on the decorator. Adding a [[triggers]]
table (or a package_type key) to package.toml is rejected at upload.
    """)

    print("How the reconciler wires it up on load:")
    print("  1. Package is loaded -> entry_module is imported")
    print("  2. @cloaca.trigger / @cloaca.task decorators run at import")
    print("  3. Trigger registers itself (name, interval, concurrency)")
    print("  4. TriggerScheduler polls the registered trigger")
    print("  5. When the trigger fires -> bound workflow executes")


def main():
    """Main tutorial."""
    print("Cloaca Tutorial 08: Packaged Triggers")
    print("=" * 60)

    demo_trigger_polls()
    demo_workflow_execution()
    demo_package_explanation()

    print("\n" + "=" * 60)
    print("Tutorial Complete!")
    print("=" * 60)
    print("\nWhat you learned:")
    print("  - @cloaca.trigger provides the poll implementation AND declaration")
    print("  - package.toml has no triggers section — the decorator is the source")
    print("  - The reconciler imports entry_module, which registers the trigger")
    print("  - The trigger binds to the workflow built in the same module")
    print("\nSee also:")
    print("  - Tutorial 07: Event triggers basics")
    print("  - examples/features/packaged-triggers/: Rust packaged trigger")


if __name__ == "__main__":
    main()
