"""
Simple Workflow Test - The simplest possible workflow execution test
"""

import pytest


def test_workflow_builder_import():
    """Test that WorkflowBuilder can be imported."""
    import cloaca
    
    # Should be able to access WorkflowBuilder
    assert hasattr(cloaca, 'WorkflowBuilder')
    assert callable(cloaca.WorkflowBuilder)


def test_simple_workflow_execution(shared_runner):
    """Test executing a simple workflow with one task."""
    import cloaca
    
    # Define a simple task
    @cloaca.task(id="simple_task")
    def simple_task(context):
        context.set("task_executed", True)
        return context
    
    # Create workflow manually (no auto-registration)
    builder = cloaca.WorkflowBuilder("simple_workflow")
    builder.description("Simple workflow test")
    builder.add_task("simple_task")
    workflow = builder.build()
    
    # Register the workflow manually
    def create_workflow():
        return workflow
    
    cloaca.register_workflow_constructor("simple_workflow", create_workflow())
    
    # Execute workflow using shared runner
    context = cloaca.Context({"input": "test"})
    result = shared_runner.execute("simple_workflow", context)
    
    # Verify execution
    assert result is not None
    assert result.status == "Completed"
    assert result.final_context is not None
    # Note: final_context is the injected context, not the modified one