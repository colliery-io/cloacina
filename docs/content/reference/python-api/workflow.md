---
title: "Workflow"
description: "Workflow objects represent executable task pipelines"
weight: 60
aliases:
  - "/python/api-reference/workflow/"

---

# Workflow

The `Workflow` class represents a built workflow that can be executed by a runner. Workflows are typically created using the [WorkflowBuilder]({{< ref "/reference/python-api/workflow-builder/" >}}) class.

## Overview

A workflow defines the structure and execution order of tasks, including their dependencies and metadata. Once built, workflows are immutable and can be executed multiple times with different contexts.

## Creating Workflows

Workflows are created using the WorkflowBuilder:

```python
import cloaca

# Define tasks
@cloaca.task()
def task_a(context):
    context.set("step", "A completed")
    return context

@cloaca.task(dependencies=["task_a"])
def task_b(context):
    previous_step = context.get("step")
    context.set("step", f"{previous_step}, B completed")
    return context

# Create workflow
builder = cloaca.WorkflowBuilder("my_workflow")
builder.description("Example workflow with dependencies")
builder.add_task("task_a")
builder.add_task("task_b")

workflow = builder.build()
```

## Declaring input params

`@cloaca.workflow_params(...)` declares a workflow's typed, injectable
execute-time inputs — the Python parity of Rust's `#[workflow(params(...))]`.
Apply it to the workflow's entry task. Each entry is `name=Type` (required) or
`name=(Type, default)` (optional). The compiler turns these into JSON-Schema
`InputSlot`s exposed on the workflow's `declared_params` and rendered as a typed
form in the web UI's Run dialog.

```python
@cloaca.workflow_params(
    source_id=str,             # required
    batch_size=(int, 500),     # optional, default 500
)
@cloaca.task(dependencies=[])
def prepare(context):
    return context
```

| Form | Meaning |
|---|---|
| `name=Type` | Required param (`Type` is `str` / `int` / `float` / `bool`). |
| `name=(Type, default)` | Optional param with a default. |

Declared params are **validated at the execute API** — supplying an unknown or
mistyped value, or omitting a required one, is rejected. They are otherwise a
pass-through into the run context. See
[Declare workflow inputs](/embed/how-to/declare-workflow-inputs/) for the full
flow (Rust + Python + the `declared_params` API surface).

> Trigger-/cron-fired workflows are executed with no caller-supplied params, so
> declare those workflows' params with defaults (optional) to keep them firing
> unattended.

## Workflow Properties

### Basic Properties

- `name` (str): Unique identifier for the workflow
- `description` (str): Human-readable description of the workflow's purpose
- `tasks` (list): List of tasks in the workflow
- `dependencies` (dict): Task dependency mapping

### Accessing Properties

```python
# Get workflow information
print(f"Workflow name: {workflow.name}")
print(f"Description: {workflow.description}")
print(f"Number of tasks: {len(workflow.tasks)}")
```

## Execution

Workflows are executed using a [DefaultRunner]({{< ref "/reference/python-api/runner/" >}}):

```python
# Create runner
runner = cloaca.DefaultRunner("sqlite:///:memory:")

# Register workflow
cloaca.register_workflow_constructor("my_workflow", lambda: workflow)

# Execute workflow
context = cloaca.Context({"input_data": "example"})
result = runner.execute("my_workflow", context)

print(f"Execution status: {result.status}")
print(f"Final context: {result.final_context.data}")
```

## Workflow Validation

Workflows are automatically validated during the build process:

```python
try:
    workflow = builder.build()
    print("Workflow is valid")
except Exception as e:
    print(f"Validation failed: {e}")
```

Common validation errors include:
- Circular dependencies between tasks
- Missing task dependencies
- Duplicate task IDs
- Invalid task references

## See Also

- **[WorkflowBuilder]({{< ref "/reference/python-api/workflow-builder/" >}})** - Build workflows
- **[Task Decorator]({{< ref "/reference/python-api/task/" >}})** - Define workflow tasks
- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** - Execute workflows
- **[Context]({{< ref "/reference/python-api/context/" >}})** - Data flow between tasks
- **Tutorials** - For worked, end-to-end examples (ETL, parallel fan-in, error handling), see the [embedded tutorials]({{< ref "/embed/tutorials/" >}}) — in particular [Dependencies]({{< ref "/embed/tutorials/03-dependencies/" >}}) and [Error Handling]({{< ref "/embed/tutorials/04-error-handling/" >}}).
