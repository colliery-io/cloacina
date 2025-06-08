#!/usr/bin/env python3
"""
Cloacina Python Tutorial 01: Your First Workflow

This example demonstrates the basic concepts of creating and executing
a simple workflow using Python decorators and the workflow builder pattern.

Learning objectives:
- Define tasks using Python decorators
- Build workflows with the WorkflowBuilder
- Execute workflows with the DefaultRunner
- Pass data between tasks using Context
- Handle workflow results and cleanup

Usage:
    python python_tutorial_01_first_workflow.py

Prerequisites:
    pip install cloaca[sqlite]
"""

import cloaca

# Define tasks using the @task decorator
@cloaca.task(id="start_process")
def start_process(context):
    """Initialize the workflow with some data."""
    print("Starting the workflow...")

    # Add initial data to the context
    context.set("process_id", "proc_001")
    context.set("start_time", "2025-01-07T10:00:00Z")
    context.set("items_to_process", 10)

    print(f"Process {context.get('process_id')} initialized")
    return context

@cloaca.task(id="process_data", dependencies=["start_process"])
def process_data(context):
    """Process the data from the previous task."""
    print("Processing data...")

    # Get data from the previous task
    process_id = context.get("process_id")
    items_count = context.get("items_to_process")

    # Simulate processing
    processed_items = []
    for i in range(items_count):
        processed_items.append(f"item_{i}_processed")

    # Store results in context
    context.set("processed_items", processed_items)
    context.set("processing_complete", True)

    print(f"Processed {len(processed_items)} items for {process_id}")
    return context

@cloaca.task(id="finalize_process", dependencies=["process_data"])
def finalize_process(context):
    """Finalize the workflow and generate summary."""
    print("Finalizing process...")

    # Get all the data
    process_id = context.get("process_id")
    start_time = context.get("start_time")
    processed_items = context.get("processed_items")

    # Create summary
    summary = {
        "process_id": process_id,
        "start_time": start_time,
        "items_processed": len(processed_items),
        "status": "completed"
    }

    context.set("final_summary", summary)
    print(f"Process {process_id} completed successfully")
    return context

# Create workflow builder function
def create_simple_workflow():
    """Build and return the workflow."""
    builder = cloaca.WorkflowBuilder("simple_workflow")
    builder.description("A simple three-task workflow demonstrating basic concepts")

    # Add tasks to the workflow
    builder.add_task("start_process")
    builder.add_task("process_data")
    builder.add_task("finalize_process")

    return builder.build()

# Register the workflow
cloaca.register_workflow_constructor("simple_workflow", create_simple_workflow)

# Execute the workflow
if __name__ == "__main__":
    print("=== Cloacina Python Tutorial 01: Your First Workflow ===")
    print()
    print("This tutorial demonstrates:")
    print("- Task definition with @cloaca.task decorator")
    print("- Sequential task dependencies")
    print("- Data flow through context")
    print("- Workflow builder pattern")
    print("- Basic execution and result handling")
    print()

    # Create a runner with SQLite database
    runner = cloaca.DefaultRunner("sqlite:///python_tutorial_01.db")

    # Create initial context
    context = cloaca.Context({"tutorial": "01", "user": "learner"})

    # Execute the workflow
    print("Executing workflow...")
    result = runner.execute("simple_workflow", context)

    # Check results
    print(f"\nWorkflow Status: {result.status}")

    if result.status == "Completed":
        print("Success! Workflow completed.")

        # Access the final context and results
        final_context = result.final_context
        summary = final_context.get("final_summary")

        print(f"Final Summary: {summary}")
        print(f"Items processed: {len(final_context.get('processed_items'))}")

    else:
        print(f"Workflow failed with status: {result.status}")
        if hasattr(result, 'error'):
            print(f"Error: {result.error}")

    # Clean up
    print("\nCleaning up...")
    runner.shutdown()
    print("Tutorial 01 completed!")
    print()
    print("Next steps:")
    print("- Try python_tutorial_02_context_handling.py for advanced context usage")
    print("- Modify this example to add validation or parallel processing")
    print("- Explore the API reference documentation")
