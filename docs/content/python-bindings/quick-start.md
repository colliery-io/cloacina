---
title: "Quick Start"
description: "Get started with Cloaca"
weight: 10
reviewer: "automation"
review_date: "2025-01-07"
---

# Quick Start Guide

This guide walks you through creating a simple workflow that demonstrates the core concepts.

## Installation

Install Cloaca using pip:

{{< tabs "installation" >}}
{{< tab "SQLite (Development)" >}}
```bash
pip install cloaca[sqlite]
```
{{< /tab >}}

{{< tab "PostgreSQL (Production)" >}}
```bash
pip install cloaca[postgres]
```
{{< /tab >}}
{{< /tabs >}}

{{< hint type="important" title="Platform Support" >}}
Cloaca provides pre-built wheels for **Linux** and **macOS** on Python 3.9-3.12.

For other platforms or architectures, Cloaca will build from source, which requires:
- **Rust toolchain** (install from [rustup.rs](https://rustup.rs/))
- **System dependencies** for your chosen backend:
  - PostgreSQL: `libpq-dev` (Ubuntu/Debian) or `postgresql-devel` (RHEL/CentOS)
  - SQLite: Usually included with Python

If you encounter build issues, ensure you have the latest Rust toolchain: `rustup update`
{{< /hint >}}

## Your First Workflow

Let's create a simple data processing workflow that demonstrates task dependencies, context passing, and error handling.

Create a new file called `first_workflow.py`:

```python
import cloaca

# Define your tasks using the @task decorator
@cloaca.task(id="fetch_data")
def fetch_data(context):
    """Simulate fetching data from an external source."""
    print("Fetching data...")

    # Simulate some data
    data = [1, 2, 3, 4, 5]
    context.set("raw_data", data)
    context.set("fetch_timestamp", "2025-01-07T10:00:00Z")

    print(f"Fetched {len(data)} items")
    return context

@cloaca.task(id="process_data", dependencies=["fetch_data"])
def process_data(context):
    """Process the fetched data."""
    print("Processing data...")

    # Get data from previous task
    raw_data = context.get("raw_data")

    # Process the data (double each value)
    processed_data = [x * 2 for x in raw_data]
    context.set("processed_data", processed_data)

    print(f"Processed {len(processed_data)} items")
    return context

@cloaca.task(id="save_results", dependencies=["process_data"])
def save_results(context):
    """Save the processed results."""
    print("Saving results...")

    # Get processed data
    processed_data = context.get("processed_data")
    fetch_timestamp = context.get("fetch_timestamp")

    # Simulate saving to a file or database
    result_summary = {
        "total_items": len(processed_data),
        "sum": sum(processed_data),
        "processed_at": fetch_timestamp
    }

    context.set("result_summary", result_summary)
    print(f"Results saved: {result_summary}")
    return context

# Create workflow builder function
def create_data_processing_workflow():
    """Build and return the workflow."""
    builder = cloaca.WorkflowBuilder("data_processing_workflow")
    builder.description("A simple data processing pipeline")

    # Add tasks to the workflow
    builder.add_task("fetch_data")
    builder.add_task("process_data")
    builder.add_task("save_results")

    return builder.build()

# Register the workflow
cloaca.register_workflow_constructor(
    "data_processing_workflow",
    create_data_processing_workflow
)

# Execute the workflow
if __name__ == "__main__":
    # Create a runner (using SQLite for this example)
    runner = cloaca.DefaultRunner("sqlite:///workflow.db")

    # Create initial context
    context = cloaca.Context({"job_id": "job_001", "user": "demo"})

    # Execute the workflow
    print("Starting workflow execution...")
    result = runner.execute("data_processing_workflow", context)

    # Check results
    if result.status == "Completed":
        print("✅ Workflow completed successfully!")

        # Access the final context
        final_context = result.final_context
        summary = final_context.get("result_summary")
        print(f"Final results: {summary}")

    else:
        print(f"❌ Workflow failed with status: {result.status}")
        if hasattr(result, 'error'):
            print(f"Error: {result.error}")

    # Clean up
    runner.shutdown()
```

## Run the Workflow

Execute your workflow:

```bash
python first_workflow.py
```

You should see output like this:

```
Starting workflow execution...
Fetching data...
Fetched 5 items
Processing data...
Processed 5 items
Saving results...
Results saved: {'total_items': 5, 'sum': 30, 'processed_at': '2025-01-07T10:00:00Z'}
✅ Workflow completed successfully!
Final results: {'total_items': 5, 'sum': 30, 'processed_at': '2025-01-07T10:00:00Z'}
```

## What Just Happened?

Let's break down the key concepts:

{{< tabs "concepts" >}}
{{< tab "Tasks" >}}
**Tasks** are the building blocks of your workflow:

```python
@cloaca.task(id="fetch_data")
def fetch_data(context):
    # Your task logic here
    return context
```

- Use the `@cloaca.task` decorator
- Specify a unique `id` for each task
- Define `dependencies` to control execution order
- Tasks receive and return a `context` object
{{< /tab >}}

{{< tab "Dependencies" >}}
**Dependencies** control task execution order:

```python
@cloaca.task(id="process_data", dependencies=["fetch_data"])
def process_data(context):
    # This runs after fetch_data completes
    return context
```

- Tasks run in dependency order
- Multiple dependencies supported: `dependencies=["task1", "task2"]`
- Parallel execution when no dependencies conflict
{{< /tab >}}

{{< tab "Context" >}}
**Context** passes data between tasks:

```python
# Set data in one task
context.set("raw_data", [1, 2, 3])

# Get data in another task
raw_data = context.get("raw_data")
```

- Persistent across the entire workflow
- Type-safe data storage
- Available in final results
{{< /tab >}}

{{< tab "Workflow Builder" >}}
**WorkflowBuilder** assembles your tasks:

```python
def create_workflow():
    builder = cloaca.WorkflowBuilder("my_workflow")
    builder.description("Description here")
    builder.add_task("task_id")
    return builder.build()

cloaca.register_workflow_constructor("my_workflow", create_workflow)
```

- Register workflow constructor functions
- Builder pattern for clean workflow definition
- Automatic dependency validation
{{< /tab >}}
{{< /tabs >}}

## Database Configuration

Choose your database backend based on your needs:

{{< tabs "database-config" >}}
{{< tab "SQLite (Simple)" >}}
Perfect for development and single-machine deployments:

```python
# File-based database
runner = cloaca.DefaultRunner("sqlite:///app.db")

# In-memory database (testing)
runner = cloaca.DefaultRunner("sqlite:///:memory:")

# With options
runner = cloaca.DefaultRunner(
    "sqlite:///app.db?mode=rwc&_journal_mode=WAL"
)
```
{{< /tab >}}

{{< tab "PostgreSQL (Production)" >}}
For production deployments and multi-tenancy:

```python
# Basic connection
runner = cloaca.DefaultRunner(
    "postgresql://user:pass@localhost:5432/dbname"
)

# With schema (multi-tenancy)
runner = cloaca.DefaultRunner.with_schema(
    "postgresql://user:pass@localhost:5432/dbname",
    "tenant_schema"
)
```
{{< /tab >}}
{{< /tabs >}}

## Error Handling

Add robust error handling to your workflows:

```python
@cloaca.task(id="robust_task")
def robust_task(context):
    try:
        # Your task logic
        risky_operation()
        context.set("success", True)
    except Exception as e:
        # Log the error
        print(f"Task failed: {e}")
        context.set("error", str(e))
        # Re-raise to mark task as failed
        raise

    return context
```

## Next Steps

Now that you have a working workflow, explore more advanced features:

{{< button href="/python-bindings/tutorials/" >}}📚 Follow Tutorials{{< /button >}}
{{< button href="/python-bindings/examples/" >}}💡 See More Examples{{< /button >}}
{{< button href="/python-bindings/api-reference/" >}}📖 API Reference{{< /button >}}

### Recommended Learning Path

1. **[Tutorial 01: First Python Workflow](/python-bindings/tutorials/01-first-python-workflow/)** - Deep dive into workflow basics
2. **[Tutorial 02: Context Handling](/python-bindings/tutorials/02-context-handling/)** - Master data passing between tasks
3. **[Tutorial 03: Error Handling](/python-bindings/tutorials/03-error-handling/)** - Build resilient workflows
4. **[Tutorial 04: Async Workflows](/python-bindings/tutorials/04-async-workflows/)** - Leverage Python's async capabilities

### Common Next Tasks

- **Add retry logic**: Learn about automatic retry mechanisms
- **Schedule workflows**: Set up cron-based scheduling
- **Handle errors gracefully**: Implement recovery strategies
- **Scale with PostgreSQL**: Move to production database
- **Multi-tenancy**: Isolate workflows by tenant

{{< hint type="tip" title="Development Tip" >}}
Start with SQLite for development and testing, then migrate to PostgreSQL for production. The API is identical between backends, making migration seamless.
{{< /hint >}}
