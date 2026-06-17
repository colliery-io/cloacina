---
title: "WorkflowBuilder"
description: "WorkflowBuilder class for creating workflows"
weight: 30
reviewer: "automation"
review_date: "2025-01-07"
aliases:
  - "/python/api-reference/workflow-builder/"

---

# WorkflowBuilder

The `WorkflowBuilder` class provides a builder pattern for constructing workflows. It allows you to add tasks, set descriptions, configure dependencies, and build executable workflow objects.

{{< hint info >}}
**Which pattern, and when**

Cloaca has three ways to define a workflow. They are not interchangeable — pick by how the workflow is deployed:

1. **Context manager — `with cloaca.WorkflowBuilder(...) as builder:`** — for **in-process** workflows you run yourself with `DefaultRunner`. Exiting the `with` block builds the workflow and registers it into the runtime your `DefaultRunner` reads from. This is the common case and what the tutorials use.
2. **Manual builder + `register_workflow_constructor`** — the same in-process scenario, for **dynamic or programmatic** construction (e.g. a factory that builds workflow variants from config). You call `build()` yourself and register a constructor function — see [Build Workflows with WorkflowBuilder]({{< ref "/engine/workflows/how-to/build-workflows-with-the-builder" >}}).
3. **Bare `@cloaca.task` decorators (no builder at all)** — for **packaged `.cloacina` workflows** loaded by a server or daemon. The package loader supplies the workflow context, so a packaged module declares tasks with `@cloaca.task` and does **not** construct a `WorkflowBuilder`.

**Pitfall:** a `WorkflowBuilder` inside a packaged workflow module fails to load (the loader has already supplied the workflow context). Packaged modules use bare decorators only — see [Packaging Python Workflows]({{< ref "/embed/how-to/packaging-python-workflows" >}}).
{{< /hint >}}

This page documents patterns 1 and 2 (in-process). For pattern 3, see the packaging guides.

## Constructor

### `WorkflowBuilder(name)`

Create a new workflow builder.

**Parameters:**
- `name` (str): Unique workflow name

**Example:**
```python
import cloaca

builder = cloaca.WorkflowBuilder("data_processing_workflow")
```

**Naming Rules:**
- Must be unique within your application
- Recommended: Use snake_case or kebab-case
- Avoid spaces and special characters
- Should be descriptive of the workflow's purpose

## Configuration Methods

### `description(description)`

Set the workflow description.

**Parameters:**
- `description` (str): Human-readable description of the workflow

**Example:**
```python
builder = cloaca.WorkflowBuilder("etl_pipeline")
builder.description("Extract data from API, transform format, and load to database")
```

### `tag(key, value)`

Add a tag to the workflow for metadata and organization.

**Parameters:**
- `key` (str): Tag key
- `value` (str): Tag value

**Example:**
```python
builder = cloaca.WorkflowBuilder("daily_report")
builder.description("Generate daily sales report")
builder.tag("department", "sales")
builder.tag("frequency", "daily")
builder.tag("priority", "high")
```

**Common Tag Patterns:**
- `department`: Team or department responsible
- `environment`: dev, staging, production
- `priority`: low, medium, high, critical
- `schedule`: daily, weekly, monthly, on-demand
- `category`: etl, reporting, monitoring, cleanup

## Task Management

### `add_task(task)`

Add a task to the workflow.

**Parameters:**
- `task` (str or callable): Task ID string or task function reference

**Example:**
```python
# Method 1: Add by task ID (string)
@cloaca.task()
def extract_data(context):
    return context

@cloaca.task(dependencies=["extract_data"])
def transform_data(context):
    return context

builder = cloaca.WorkflowBuilder("etl_workflow")
builder.add_task("extract_data")
builder.add_task("transform_data")

# Method 2: Add by function reference
builder = cloaca.WorkflowBuilder("etl_workflow")
builder.add_task(extract_data)
builder.add_task(transform_data)
```

**Task Resolution:**
- **String**: Must match the `id` parameter of a `@cloaca.task` decorated function
- **Function**: Must be a `@cloaca.task` decorated function

## Building Workflows

### `build()`

Build the workflow and validate its structure.

**Returns:** Workflow object ready for execution

**Raises:**
- `ValueError`: if the workflow has structural problems or references a task that doesn't exist (all builder failures surface as `ValueError`)

**Example:**
```python
builder = cloaca.WorkflowBuilder("my_workflow")
builder.description("Sample workflow")
builder.add_task("task_1")
builder.add_task("task_2")

# Build and validate
workflow = builder.build()
```

**Validation Checks:**
- All referenced tasks exist
- No circular dependencies
- All dependencies are resolvable
- Workflow has at least one task

## Context Manager Support

WorkflowBuilder supports context manager protocol for automatic registration.

### `with WorkflowBuilder(...) as builder:`

**Example:**
```python
import cloaca

@cloaca.task()
def hello_task(context):
    context.set("message", "Hello, World!")
    return context

# Automatic registration
with cloaca.WorkflowBuilder("hello_workflow") as builder:
    builder.description("Simple hello world workflow")
    builder.add_task("hello_task")
    # Workflow automatically registered when exiting context

# Can execute immediately
runner = cloaca.DefaultRunner("sqlite:///app.db")
context = cloaca.Context()
result = runner.execute("hello_workflow", context)
```

## Task-Oriented Guidance

For worked examples, dynamic and conditional construction, validation and
debugging, error-handling patterns, and best practices, see
[Build Workflows with WorkflowBuilder]({{< ref "/engine/workflows/how-to/build-workflows-with-the-builder" >}}).

## Related Classes

- **[Context]({{< ref "/reference/python-api/context/" >}})** - Data passed through workflows
- **[DefaultRunner]({{< ref "/reference/python-api/runner/" >}})** - Executes built workflows
- **[Task Decorator]({{< ref "/reference/python-api/task/" >}})** - Defines tasks added to workflows
- **[Workflow]({{< ref "/reference/python-api/workflow/" >}})** - Built workflow objects
