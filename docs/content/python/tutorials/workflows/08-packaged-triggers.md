---
title: "08 - Packaged Triggers"
description: "Define event-driven triggers in packaged Python workflows for deployment to the Cloacina daemon"
weight: 18
---

# Packaged Triggers

In this tutorial, you'll learn how to define event-driven triggers alongside your Python workflows so they can be packaged and deployed to the Cloacina daemon. While [Tutorial 7]({{< ref "/python/tutorials/workflows/07-event-triggers" >}}) introduced triggers running directly in your Python process, this tutorial focuses on the packaging story — how triggers and workflows are declared, bundled, and auto-registered when loaded by the reconciler (the daemon component that discovers, validates, and registers packages).

## Learning Objectives

- Define triggers alongside tasks in a packaged workflow
- Understand the relationship between `@cloaca.trigger` decorators and manifest declarations
- Package a trigger-bearing workflow as a `.cloacina` archive
- See how the reconciler wires triggers to workflows on load

## Prerequisites

- Completion of [Tutorial 7: Event Triggers]({{< ref "/python/tutorials/workflows/07-event-triggers" >}})
- Familiarity with the daemon (see [Running the Daemon]({{< ref "/platform/how-to-guides/running-the-daemon" >}}))

## Time Estimate

20-25 minutes

## Step 1: Define the Workflow

Start with a workflow that the trigger will fire. Create a file called `data_ingest/__init__.py`:

```python
import cloaca
from datetime import datetime

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
```

This is a standard workflow — nothing special about triggers yet.

## Step 2: Define the Trigger

Add a trigger at module level, **outside** the `WorkflowBuilder` context. Triggers are registered independently from the workflow's task graph:

```python
@cloaca.trigger(
    name="inbox_watcher",
    poll_interval="5s",
    allow_concurrent=False
)
def inbox_watcher():
    """
    Poll for new files in the inbox directory.

    Returns TriggerResult.fire() with context when a new file
    is detected, or TriggerResult.skip() when nothing is found.
    """
    # In a real trigger, you'd check a filesystem, API, queue, etc.
    import os
    inbox = os.environ.get("INBOX_PATH", "/data/inbox/")

    new_files = [f for f in os.listdir(inbox) if f.endswith(".parquet")]
    if new_files:
        filename = new_files[0]
        ctx = cloaca.Context({
            "filename": filename,
            "trigger_name": "inbox_watcher",
            "triggered_at": datetime.now().isoformat(),
        })
        return cloaca.TriggerResult.fire(ctx)

    return cloaca.TriggerResult.skip()
```

Notice the three parameters: `name` identifies the trigger (and must match the manifest declaration), `poll_interval` controls how often the function is called, and `allow_concurrent=False` prevents overlapping executions. See the [Package Manifest Reference]({{< ref "/platform/reference/package-manifest" >}}) for the full field listing.

## Step 3: Understand the Manifest

When this workflow is packaged as a `.cloacina` archive, the manifest (`manifest.json`) declares both the tasks and the triggers. Here's what the `triggers` section looks like:

```json
{
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
```

The `name` must match your `@cloaca.trigger(name=...)` value exactly, and `workflow` tells the reconciler which workflow to fire. See the [Package Manifest Reference]({{< ref "/platform/reference/package-manifest" >}}) for the complete schema.

## Step 4: Set Up the Package

Create a `pyproject.toml` for the package:

```toml
[project]
name = "data-ingest"
version = "1.0.0"
description = "File ingestion workflow with inbox watcher trigger"
requires-python = ">=3.10"

[tool.cloaca]
entry_module = "data_ingest"
```

The `entry_module` tells the loader which Python module to import for task and trigger discovery.

## Step 5: Test Locally

Before packaging, test the trigger and workflow in library mode:

```python
def test_trigger_and_workflow():
    """Simulate what the daemon does on trigger fire."""
    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Simulate a trigger poll that fires
        result = inbox_watcher()

        if result.is_fire_result():
            # Execute the workflow with the trigger's context
            context = cloaca.Context({
                "filename": "orders_20260328.parquet",
                "trigger_name": "inbox_watcher",
            })
            wf_result = runner.execute("data_ingest", context)
            print(f"Workflow status: {wf_result.status}")
    finally:
        runner.shutdown()
```

## Step 6: Deploy to the Daemon

Copy your `.cloacina` package into the daemon's watch directory. The reconciler will import your module, match your `@cloaca.trigger` decorator to the manifest declaration, and start polling automatically.

```bash
cp data-ingest-1.0.0.cloacina ~/.cloacina/packages/
```

{{< hint type="important" title="Name Agreement" >}}
The `name` in `@cloaca.trigger(name="inbox_watcher")` **must** match the `name` in the manifest's `triggers` array. If they disagree, the reconciler will reject the package.
{{< /hint >}}

## What You Learned

- `@cloaca.trigger` provides the poll implementation; the manifest declares it for the reconciler
- Both must agree on the trigger name
- Triggers are packaged alongside tasks in the same `.cloacina` archive
- The reconciler wires them together on package load

## Next Steps

- [Computation Graphs]({{< ref "/python/tutorials/computation-graphs/09-computation-graph" >}}) — reactive, event-driven processing
- [Package Manifest Reference]({{< ref "/platform/reference/package-manifest" >}}) — full manifest schema
- [Running the Daemon]({{< ref "/platform/how-to-guides/running-the-daemon" >}}) — deploy your package
