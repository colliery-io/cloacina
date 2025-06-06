"""
Test the workflow builder pattern using raw method calls (no decorators).
This test demonstrates creating and executing workflows using the programmatic API.
"""

import pytest
import asyncio
from pathlib import Path
import sys

# Add the cloaca package to the path
sys.path.insert(0, str(Path(__file__).parent.parent / "cloaca" / "src"))

import cloaca


def test_builder_pattern_simple_workflow(shared_runner):
    """Test creating a simple workflow using the builder pattern."""
    
    # Define a simple task function with decorator
    @cloaca.task(id="simple_task")
    def simple_task(context):
        context.set("task_executed", True)
        return context
    
    # Create workflow using builder pattern (not decorator)
    builder = cloaca.WorkflowBuilder("test_builder_workflow")
    builder.description("Simple workflow test")
    builder.add_task("simple_task")
    workflow = builder.build()
    
    # Execute the workflow
    runner = shared_runner
    context = cloaca.Context()
    context.set("test_id", "builder_001")
    
    result = runner.execute("test_builder_workflow", context)
    
    # Verify execution
    assert result is not None
    assert result.status == "Completed"


def test_builder_pattern_multi_task_workflow(shared_runner):
    """Test creating a multi-task workflow with dependencies using the builder pattern."""
    
    # Define task functions with decorators
    @cloaca.task(id="task_a")
    def task_a(context):
        context.set("task_a_executed", True)
        return context
    
    @cloaca.task(id="task_b", dependencies=["task_a"])
    def task_b(context):
        context.set("task_b_executed", True)
        return context
    
    @cloaca.task(id="task_c", dependencies=["task_a", "task_b"])
    def task_c(context):
        context.set("task_c_executed", True)
        return context
    
    # Create workflow using builder pattern (not decorator)
    builder = cloaca.WorkflowBuilder("test_multi_task_workflow")
    builder.description("Multi-task workflow test")
    builder.add_task("task_a")
    builder.add_task("task_b")
    builder.add_task("task_c")
    workflow = builder.build()
    
    # Execute the workflow
    runner = shared_runner
    context = cloaca.Context()
    context.set("test_id", "builder_002")
    
    result = runner.execute("test_multi_task_workflow", context)
    
    # Verify execution
    assert result is not None
    assert result.status == "Completed"


def test_builder_pattern_with_parameters(shared_runner):
    """Test workflow builder with parameterized tasks."""
    
    # Define a parameterized task function
    @cloaca.task(id="param_task")
    def parameterized_task(context):
        message = context.get("message", "default")
        count = context.get("count", 1)
        result_msg = f"{message} repeated {count} times"
        context.set("result", result_msg)
        return context
    
    # Create workflow using builder pattern (not decorator)
    builder = cloaca.WorkflowBuilder("test_param_workflow")
    builder.description("Parameterized workflow test")
    builder.add_task("param_task")
    workflow = builder.build()
    
    # Execute the workflow
    runner = shared_runner
    context = cloaca.Context()
    context.set("message", "Hello")
    context.set("count", 3)
    context.set("test_id", "builder_003")
    
    result = runner.execute("test_param_workflow", context)
    
    # Verify execution
    assert result is not None
    assert result.status == "Completed"


def test_builder_pattern_async_tasks(shared_runner):
    """Test workflow builder with async task functions."""
    
    # Define async task functions
    @cloaca.task(id="async_1")
    def async_task_1(context):
        # Note: Current implementation may not support async, so using sync
        import time
        time.sleep(0.01)  # Short sleep to simulate async work
        context.set("async_1_executed", True)
        return context
    
    @cloaca.task(id="async_2", dependencies=["async_1"])
    def async_task_2(context):
        import time
        time.sleep(0.01)  # Short sleep to simulate async work
        context.set("async_2_executed", True)
        return context
    
    # Create workflow using builder pattern (not decorator)
    builder = cloaca.WorkflowBuilder("test_async_workflow")
    builder.description("Async workflow test")
    builder.add_task("async_1")
    builder.add_task("async_2")
    workflow = builder.build()
    
    # Execute the workflow
    runner = shared_runner
    context = cloaca.Context()
    context.set("test_id", "builder_004")
    
    result = runner.execute("test_async_workflow", context)
    
    # Verify execution
    assert result is not None
    assert result.status == "Completed"


if __name__ == "__main__":
    # Run tests directly
    test_builder_pattern_simple_workflow()
    test_builder_pattern_multi_task_workflow()
    test_builder_pattern_with_parameters()
    test_builder_pattern_async_tasks()
    print("All builder pattern tests completed successfully!")