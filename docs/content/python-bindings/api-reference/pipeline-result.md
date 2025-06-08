---
title: "PipelineResult"
description: "Results from workflow execution"
weight: 80
---

# PipelineResult

The `PipelineResult` class contains the outcome and metadata from a workflow execution. It provides information about execution status, timing, errors, and the final context state.

## Properties

### Basic Properties

- `status` (PipelineStatus): The final execution status
- `workflow_name` (str): Name of the executed workflow
- `execution_id` (str): Unique identifier for this execution
- `final_context` (Context): The context after all tasks completed
- `start_time` (datetime): When execution began
- `end_time` (datetime): When execution finished
- `duration` (timedelta): Total execution time

### Status Information

```python
import cloaca

# Execute workflow
runner = cloaca.DefaultRunner("sqlite:///:memory:")
result = runner.execute("my_workflow", context)

# Check execution status
print(f"Status: {result.status}")
print(f"Workflow: {result.workflow_name}")
print(f"Execution ID: {result.execution_id}")
print(f"Duration: {result.duration}")
```

## PipelineStatus Enum

The status indicates the outcome of workflow execution:

- `PENDING`: Execution is queued but not started
- `RUNNING`: Execution is currently in progress
- `COMPLETED`: All tasks completed successfully
- `FAILED`: One or more tasks failed
- `CANCELLED`: Execution was cancelled before completion

### Status Checking

```python
if result.status == cloaca.PipelineStatus.COMPLETED:
    print("Workflow completed successfully!")
    # Process successful result
    final_data = result.final_context.get("output_data")

elif result.status == cloaca.PipelineStatus.FAILED:
    print("Workflow failed!")
    # Handle failure
    error_info = result.final_context.get("error")

elif result.status == cloaca.PipelineStatus.CANCELLED:
    print("Workflow was cancelled")
    # Handle cancellation
```

## Context Access

Access the final context state after execution:

```python
# Get final context data
final_context = result.final_context

# Extract specific results
if result.status == cloaca.PipelineStatus.COMPLETED:
    output_data = final_context.get("processed_data")
    record_count = final_context.get("records_processed", 0)

    print(f"Processed {record_count} records")
    print(f"Output: {output_data}")
```

## Error Information

When workflows fail, error information is available:

```python
if result.status == cloaca.PipelineStatus.FAILED:
    # Check for error information in context
    error_message = result.final_context.get("error_message")
    failed_task = result.final_context.get("failed_task")

    if error_message:
        print(f"Error: {error_message}")
    if failed_task:
        print(f"Failed task: {failed_task}")
```

## Timing Information

Analyze execution performance:

```python
# Execution timing
print(f"Started: {result.start_time}")
print(f"Finished: {result.end_time}")
print(f"Duration: {result.duration}")

# Calculate performance metrics
if result.duration:
    seconds = result.duration.total_seconds()
    print(f"Execution took {seconds:.2f} seconds")

    if seconds > 300:  # 5 minutes
        print("Long-running execution detected")
```

## Task-Level Results

Access individual task results (if available):

```python
# Get task execution details
task_results = result.final_context.get("task_results", {})

for task_id, task_result in task_results.items():
    print(f"Task {task_id}:")
    print(f"  Status: {task_result.get('status')}")
    print(f"  Duration: {task_result.get('duration')}")

    if task_result.get('error'):
        print(f"  Error: {task_result['error']}")
```

## Complete Example

```python
import cloaca
from datetime import datetime

@cloaca.task(id="process_data")
def process_data(context):
    """Example task that processes data."""
    input_data = context.get("input_data", [])

    # Simulate processing
    processed = [x * 2 for x in input_data]

    context.set("processed_data", processed)
    context.set("records_processed", len(processed))
    context.set("processing_complete", True)

    return context

# Create and execute workflow
def create_workflow():
    builder = cloaca.WorkflowBuilder("data_processing")
    builder.description("Process input data")
    builder.add_task("process_data")
    return builder.build()

# Execute and analyze result
runner = cloaca.DefaultRunner("sqlite:///:memory:")
cloaca.register_workflow_constructor("data_processing", create_workflow)

input_context = cloaca.Context({"input_data": [1, 2, 3, 4, 5]})
result = runner.execute("data_processing", input_context)

# Comprehensive result analysis
def analyze_result(result):
    """Analyze workflow execution result."""
    print("=== Workflow Execution Result ===")
    print(f"Workflow: {result.workflow_name}")
    print(f"Status: {result.status}")
    print(f"Execution ID: {result.execution_id}")

    if result.start_time and result.end_time:
        duration = result.end_time - result.start_time
        print(f"Duration: {duration.total_seconds():.2f} seconds")

    if result.status == cloaca.PipelineStatus.COMPLETED:
        print("\n=== Successful Execution ===")
        records = result.final_context.get("records_processed", 0)
        processed_data = result.final_context.get("processed_data")

        print(f"Records processed: {records}")
        print(f"Output data: {processed_data}")

    elif result.status == cloaca.PipelineStatus.FAILED:
        print("\n=== Failed Execution ===")
        error = result.final_context.get("error_message")
        if error:
            print(f"Error: {error}")

    return result.status == cloaca.PipelineStatus.COMPLETED

# Analyze the result
success = analyze_result(result)
print(f"\nExecution successful: {success}")
```

## Async Results

For long-running workflows, you can check status asynchronously:

```python
import time

# Start workflow execution
result = runner.execute_async("long_workflow", context)

# Poll for completion
while result.status == cloaca.PipelineStatus.RUNNING:
    print(f"Workflow still running... (ID: {result.execution_id})")
    time.sleep(5)
    result = runner.get_execution_status(result.execution_id)

print(f"Final status: {result.status}")
```

## Best Practices

### Result Validation

Always check the execution status before processing results:

```python
def process_workflow_result(result):
    """Safely process workflow result."""
    if result.status != cloaca.PipelineStatus.COMPLETED:
        raise RuntimeError(f"Workflow failed with status: {result.status}")

    # Safe to process successful result
    return result.final_context.get("output_data")
```

### Error Handling

Implement comprehensive error handling:

```python
def handle_workflow_result(result):
    """Handle workflow result with proper error handling."""
    try:
        if result.status == cloaca.PipelineStatus.COMPLETED:
            return process_successful_result(result)
        elif result.status == cloaca.PipelineStatus.FAILED:
            return handle_failed_result(result)
        elif result.status == cloaca.PipelineStatus.CANCELLED:
            return handle_cancelled_result(result)
        else:
            raise ValueError(f"Unexpected status: {result.status}")

    except Exception as e:
        print(f"Error handling result: {e}")
        return None
```

### Performance Monitoring

Track execution performance:

```python
def monitor_performance(result):
    """Monitor workflow performance."""
    if result.duration:
        seconds = result.duration.total_seconds()

        # Performance thresholds
        if seconds > 600:  # 10 minutes
            print(f"WARNING: Slow execution ({seconds:.1f}s)")
        elif seconds < 1:
            print(f"Very fast execution ({seconds:.3f}s)")
        else:
            print(f"Normal execution time ({seconds:.1f}s)")
```

## See Also

- **[DefaultRunner](/python-bindings/api-reference/runner/)** - Execute workflows and get results
- **[Context](/python-bindings/api-reference/context/)** - Access final context data
- **[Workflow](/python-bindings/api-reference/workflow/)** - Workflows that produce results
