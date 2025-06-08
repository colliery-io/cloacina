---
title: "Workflow"
description: "Workflow objects represent executable task pipelines"
weight: 60
---

# Workflow

The `Workflow` class represents a built workflow that can be executed by a runner. Workflows are typically created using the [WorkflowBuilder](/python-bindings/api-reference/workflow-builder/) class.

## Overview

A workflow defines the structure and execution order of tasks, including their dependencies and metadata. Once built, workflows are immutable and can be executed multiple times with different contexts.

## Creating Workflows

Workflows are created using the WorkflowBuilder:

```python
import cloaca

# Define tasks
@cloaca.task(id="task_a")
def task_a(context):
    context.set("step", "A completed")
    return context

@cloaca.task(id="task_b", dependencies=["task_a"])
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

Workflows are executed using a [DefaultRunner](/python-bindings/api-reference/runner/):

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
except cloaca.WorkflowValidationError as e:
    print(f"Validation failed: {e}")
```

Common validation errors include:
- Circular dependencies between tasks
- Missing task dependencies
- Duplicate task IDs
- Invalid task references

## Complex Workflow Example

```python
import cloaca

@cloaca.task(id="extract")
def extract_data(context):
    """Extract data from source."""
    # Simulate data extraction
    raw_data = {"users": 100, "orders": 250, "revenue": 15000}
    context.set("raw_data", raw_data)
    return context

@cloaca.task(id="transform", dependencies=["extract"])
def transform_data(context):
    """Transform the extracted data."""
    raw_data = context.get("raw_data")

    # Calculate metrics
    avg_order_value = raw_data["revenue"] / raw_data["orders"]
    metrics = {
        "total_users": raw_data["users"],
        "total_orders": raw_data["orders"],
        "total_revenue": raw_data["revenue"],
        "avg_order_value": round(avg_order_value, 2)
    }

    context.set("metrics", metrics)
    return context

@cloaca.task(id="load", dependencies=["transform"])
def load_data(context):
    """Load processed data to destination."""
    metrics = context.get("metrics")

    # Simulate loading to database/file
    print(f"Loading metrics: {metrics}")
    context.set("load_complete", True)
    return context

# Create ETL workflow
def create_etl_workflow():
    builder = cloaca.WorkflowBuilder("etl_pipeline")
    builder.description("Extract, Transform, Load data pipeline")
    builder.add_task("extract")
    builder.add_task("transform")
    builder.add_task("load")
    return builder.build()

# Usage
etl_workflow = create_etl_workflow()
```

## Parallel Execution

Workflows support parallel execution of independent tasks:

```python
@cloaca.task(id="fetch_users")
def fetch_users(context):
    # Simulate API call
    users = [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]
    context.set("users", users)
    return context

@cloaca.task(id="fetch_orders")
def fetch_orders(context):
    # Simulate API call
    orders = [{"id": 101, "user_id": 1}, {"id": 102, "user_id": 2}]
    context.set("orders", orders)
    return context

@cloaca.task(id="merge_data", dependencies=["fetch_users", "fetch_orders"])
def merge_data(context):
    users = context.get("users")
    orders = context.get("orders")

    # Merge data
    result = {"users": users, "orders": orders}
    context.set("merged_data", result)
    return context

# Create parallel workflow
def create_parallel_workflow():
    builder = cloaca.WorkflowBuilder("parallel_pipeline")
    builder.description("Fetch data in parallel, then merge")
    builder.add_task("fetch_users")
    builder.add_task("fetch_orders")
    builder.add_task("merge_data")
    return builder.build()
```

## Best Practices

### Workflow Design

1. **Single Responsibility**: Each workflow should have a clear, focused purpose
2. **Idempotency**: Design workflows to be safely re-runnable
3. **Error Handling**: Include error handling and recovery strategies
4. **Documentation**: Provide clear descriptions and task documentation

### Performance Considerations

1. **Minimize Dependencies**: Only declare necessary dependencies to maximize parallelism
2. **Context Size**: Keep context data reasonably sized for better performance
3. **Task Granularity**: Balance between too many small tasks and too few large tasks

## Error Handling

Workflows handle task failures gracefully:

```python
@cloaca.task(id="risky_task")
def risky_task(context):
    """Task that might fail."""
    try:
        # Potentially failing operation
        result = perform_risky_operation()
        context.set("success", True)
        context.set("result", result)
    except Exception as e:
        context.set("success", False)
        context.set("error", str(e))
        # Workflow can continue with error state

    return context

@cloaca.task(id="handle_errors", dependencies=["risky_task"])
def handle_errors(context):
    """Handle errors from previous tasks."""
    if context.get("success"):
        print("Task succeeded!")
    else:
        error = context.get("error")
        print(f"Task failed with error: {error}")
        # Implement recovery logic

    return context
```

## See Also

- **[WorkflowBuilder](/python-bindings/api-reference/workflow-builder/)** - Build workflows
- **[Task Decorator](/python-bindings/api-reference/task/)** - Define workflow tasks
- **[DefaultRunner](/python-bindings/api-reference/runner/)** - Execute workflows
- **[Context](/python-bindings/api-reference/context/)** - Data flow between tasks
