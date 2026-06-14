---
title: "08 - Packaged Triggers"
description: "Define event-driven triggers in packaged Python workflows for deployment to the Cloacina daemon"
weight: 18
---

# Packaged Triggers

In this tutorial, you'll learn how to define event-driven triggers alongside your Python workflows so they can be packaged and deployed to the Cloacina daemon. While [Tutorial 7]({{< ref "/python/workflows/tutorials/07-event-triggers" >}}) introduced triggers running directly in your Python process, this tutorial focuses on the packaging story — how triggers and workflows are declared, bundled, and auto-registered when loaded by the reconciler (the daemon component that discovers, validates, and registers packages).

## Learning Objectives

- Define triggers alongside tasks in a packaged workflow
- Understand that `@cloaca.trigger` decorators *are* the declaration — there is no manifest trigger section
- Package a trigger-bearing workflow as a `.cloacina` archive
- See how the reconciler wires triggers to workflows on load

## Prerequisites

- Completion of [Tutorial 7: Event Triggers]({{< ref "/python/workflows/tutorials/07-event-triggers" >}})
- Familiarity with the daemon (see [Running the Daemon]({{< ref "/platform/how-to-guides/running-the-daemon" >}}))

## Time Estimate

20-25 minutes

## Step 1: Define the Workflow

Start with a workflow that the trigger will fire. In your entry module
(`workflow/data_ingest/tasks.py`), declare tasks with **bare `@cloaca.task`
decorators at module level** — in a packaged workflow the loader builds the
workflow context from `workflow_name` in `package.toml`, so you do *not* wrap
tasks in a `WorkflowBuilder` (doing so would shadow the loader's context and the
package would load with no tasks):

```python
import cloaca
from datetime import datetime

@cloaca.task(id="validate", dependencies=[])
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

Add a trigger at module level alongside the tasks. Triggers are registered
independently from the workflow's task graph:

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

Notice the three parameters: `name` identifies the trigger, `poll_interval` controls how often the function is called, and `allow_concurrent=False` prevents overlapping executions. The decorator is the *only* place these are declared — there is no separate manifest entry to keep in sync.

## Step 3: Triggers Are Declared in Code, Not the Manifest

There is **no trigger section in the package manifest**. The `@cloaca.trigger`
decorator *is* the declaration: when the reconciler imports your `entry_module`,
the decorator registers the trigger (name, poll interval, config) and binds it to
its workflow. The manifest (`package.toml`) carries only package identity +
`[metadata]`; adding triggers to it (or a `package_type` key) is rejected at
upload.

So the only thing to get right for triggers is that the `@cloaca.trigger`
decorator runs at import time (module level) — same rule as `@cloaca.task`.

## Step 4: Set Up the Package

A `.cloacina` package is a top-level `package.toml` plus your module tree under
`workflow/`:

```
data-ingest/
├── package.toml
└── workflow/
    └── data_ingest/
        ├── __init__.py
        └── tasks.py        # @cloaca.task + @cloaca.trigger here
```

`package.toml`:

```toml
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
description = "File ingestion workflow with inbox watcher trigger"
requires_python = ">=3.10"
```

`entry_module` is the dotted path **relative to `workflow/`** that the loader
imports for task + trigger discovery. See
[Packaging Python Workflows]({{< ref "/python/workflows/how-to-guides/packaging-python-workflows" >}})
for the full format and the build/deploy steps.

## Step 5: Test Locally

Before packaging, test the trigger and workflow in library mode. Because the
module uses bare decorators, wrap the **import** in a `WorkflowBuilder` named to
match `workflow_name` — this stands in for the context the packaged loader
supplies (it is for in-process testing only, not part of the package):

```python
import cloaca

with cloaca.WorkflowBuilder("data_ingest"):
    import data_ingest.tasks   # registers validate/load + inbox_watcher

def test_trigger_and_workflow():
    """Simulate what the daemon does on trigger fire."""
    runner = cloaca.DefaultRunner(":memory:")

    try:
        # Simulate a trigger poll that fires
        result = data_ingest.tasks.inbox_watcher()

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

Copy your `.cloacina` package into the daemon's watch directory. The reconciler will import your `entry_module`, which runs your `@cloaca.trigger` decorator at import time — registering the trigger and starting the poll loop automatically.

```bash
cp data-ingest-1.0.0.cloacina ~/.cloacina/packages/
```

{{< hint type="info" title="No Manifest Sync Needed" >}}
Because the `@cloaca.trigger` decorator is the declaration, there is no `triggers`
array in `package.toml` to keep in agreement — and adding one (or a `package_type`
key) is rejected at upload. Just make sure the decorator runs at import time.
{{< /hint >}}

## What You Learned

- `@cloaca.trigger` *is* the declaration — it registers the trigger when the module is imported
- The manifest (`package.toml`) carries package identity + `[metadata]` only; no trigger section
- Triggers are packaged alongside tasks in the same `.cloacina` archive
- The reconciler imports your `entry_module` on load, which wires everything together

## Next Steps

- [Computation Graphs]({{< ref "/python/computation-graphs/tutorials/09-computation-graph" >}}) — reactive, event-driven processing
- [Packaging Python Workflows]({{< ref "/python/workflows/how-to-guides/packaging-python-workflows" >}}) — the full `package.toml` format and build steps
- [Running the Daemon]({{< ref "/platform/how-to-guides/running-the-daemon" >}}) — deploy your package
