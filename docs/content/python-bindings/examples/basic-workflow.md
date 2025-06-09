---
title: "Basic Workflow Example"
description: "Simple workflow example to get started with Cloaca Python bindings"
weight: 10
---

# Basic Workflow Example

A simple, complete example demonstrating the fundamentals of Cloaca workflow creation and execution in Python.

## Overview

This example creates a basic workflow that:
1. Processes input data
2. Applies a transformation
3. Saves the result
4. Demonstrates proper error handling

## Complete Example

```python
import cloaca
from datetime import datetime

# Define tasks
@cloaca.task(id="load_data")
def load_data(context):
    """Load data from input source."""
    print("Loading data...")

    # Simulate loading data
    data = context.get("input_data", [1, 2, 3, 4, 5])
    context.set("raw_data", data)
    context.set("load_time", datetime.now().isoformat())

    print(f"Loaded {len(data)} items")
    return context

@cloaca.task(id="process_data", dependencies=["load_data"])
def process_data(context):
    """Process the loaded data."""
    print("Processing data...")

    raw_data = context.get("raw_data", [])

    # Apply transformation (double each value)
    processed_data = [x * 2 for x in raw_data]

    context.set("processed_data", processed_data)
    context.set("process_time", datetime.now().isoformat())

    print(f"Processed {len(processed_data)} items")
    return context

@cloaca.task(id="save_result", dependencies=["process_data"])
def save_result(context):
    """Save the processed result."""
    print("Saving result...")

    processed_data = context.get("processed_data", [])

    # Simulate saving (in real app, this might write to database/file)
    result = {
        "data": processed_data,
        "total_items": len(processed_data),
        "saved_at": datetime.now().isoformat()
    }

    context.set("final_result", result)
    print(f"Saved {len(processed_data)} processed items")

    return context

# Create workflow
def create_basic_workflow():
    """Create the basic data processing workflow."""
    builder = cloaca.WorkflowBuilder("basic_data_processing")
    builder.description("A simple data processing workflow")

    # Add tasks (dependencies automatically handled)
    builder.add_task("load_data")
    builder.add_task("process_data")
    builder.add_task("save_result")

    return builder.build()

# Register the workflow
cloaca.register_workflow_constructor("basic_data_processing", create_basic_workflow)

def main():
    """Run the basic workflow example."""
    print("=== Basic Workflow Example ===")

    # Create runner (SQLite for simplicity)
    runner = cloaca.DefaultRunner("sqlite:///basic_example.db")

    try:
        # Create execution context with input data
        context = cloaca.Context({
            "input_data": [10, 20, 30, 40, 50],
            "execution_id": "example_001"
        })

        print("Starting workflow execution...")

        # Execute the workflow
        result = runner.execute("basic_data_processing", context)

        # Check execution result
        if result.status == "Completed":
            print("\n✓ Workflow completed successfully!")

            final_context = result.final_context
            final_result = final_context.get("final_result")

            print(f"Final result: {final_result}")
            print(f"Load time: {final_context.get('load_time')}")
            print(f"Process time: {final_context.get('process_time')}")

        else:
            print(f"\n✗ Workflow failed with status: {result.status}")

    except Exception as e:
        print(f"\n✗ Error executing workflow: {e}")

    finally:
        # Clean up
        runner.shutdown()
        print("\nWorkflow runner shutdown complete.")

if __name__ == "__main__":
    main()
```

## Running the Example

### Step 1: Install Dependencies

```bash
pip install cloaca
```

### Step 2: Save and Run

Save the code above as `basic_workflow.py` and run:

```bash
python basic_workflow.py
```

### Expected Output

```
=== Basic Workflow Example ===
Starting workflow execution...
Loading data...
Loaded 5 items
Processing data...
Processed 5 items
Saving result...
Saved 5 processed items

✓ Workflow completed successfully!
Final result: {'data': [20, 40, 60, 80, 100], 'total_items': 5, 'saved_at': '2025-01-07T10:30:45.123456'}
Load time: 2025-01-07T10:30:45.100000
Process time: 2025-01-07T10:30:45.110000

Workflow runner shutdown complete.
```

## Key Concepts Demonstrated

### 1. Task Definition

```python
@cloaca.task(id="task_name")
def task_function(context):
    # Task logic here
    return context
```

- Tasks are decorated functions that receive and return a `Context`
- Each task has a unique ID
- Tasks can read from and write to the context

### 2. Task Dependencies

```python
@cloaca.task(id="dependent_task", dependencies=["prerequisite_task"])
def dependent_task(context):
    # This task runs after prerequisite_task completes
    return context
```

- Dependencies ensure proper execution order
- A task won't run until all its dependencies complete successfully

### 3. Workflow Building

```python
def create_workflow():
    builder = cloaca.WorkflowBuilder("workflow_name")
    builder.description("Workflow description")
    builder.add_task("task_id")
    return builder.build()
```

- Workflows are built using the `WorkflowBuilder`
- Tasks are added by their ID
- Dependencies are automatically resolved

### 4. Context Usage

```python
# Reading from context
value = context.get("key", default_value)

# Writing to context
context.set("key", value)
```

- Context carries data between tasks
- Use `get()` with defaults for safe reading
- Use `set()` to store results for subsequent tasks

## Customization Examples

### Different Input Data

```python
# Process different types of data
context = cloaca.Context({
    "input_data": ["apple", "banana", "cherry"],
    "operation": "uppercase"
})

@cloaca.task(id="process_strings")
def process_strings(context):
    data = context.get("input_data", [])
    operation = context.get("operation", "uppercase")

    if operation == "uppercase":
        result = [item.upper() for item in data]
    elif operation == "length":
        result = [len(item) for item in data]
    else:
        result = data

    context.set("processed_data", result)
    return context
```

### Error Handling

```python
@cloaca.task(id="safe_processing")
def safe_processing(context):
    try:
        data = context.get("input_data", [])

        # Process with potential for errors
        result = [risky_operation(item) for item in data]

        context.set("processed_data", result)
        context.set("success", True)

    except Exception as e:
        context.set("error", str(e))
        context.set("success", False)
        print(f"Error in processing: {e}")

    return context
```

### Conditional Logic

```python
@cloaca.task(id="conditional_processing")
def conditional_processing(context):
    data = context.get("input_data", [])

    # Apply different logic based on data characteristics
    if len(data) > 100:
        # Use batch processing for large datasets
        result = batch_process(data)
    else:
        # Use simple processing for small datasets
        result = [process_item(item) for item in data]

    context.set("processed_data", result)
    return context
```

## Next Steps

Once you understand this basic example, explore:

1. **[Tutorial 02: Context Handling]({{< ref "/python-bindings/tutorials/02-context-handling/" >}})** - Advanced data flow between tasks
2. **[Tutorial 03: Complex Workflows]({{< ref "/python-bindings/tutorials/03-complex-workflows/" >}})** - Dependencies and parallel processing
3. **[Tutorial 04: Error Handling]({{< ref "/python-bindings/tutorials/04-error-handling/" >}})** - Robust error handling strategies

## Related Resources

- [Tutorial: First Python Workflow]({{< ref "/python-bindings/tutorials/01-first-python-workflow/" >}}) - Step-by-step learning
- [API Reference: Task Decorator]({{< ref "/python-bindings/api-reference/task/" >}}) - Complete task API
- [API Reference: WorkflowBuilder]({{< ref "/python-bindings/api-reference/workflow-builder/" >}}) - Workflow construction API
