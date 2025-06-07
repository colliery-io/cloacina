"""
Test Simple Workflow Execution

This test file demonstrates the simplest possible workflow execution,
combining import verification and basic workflow execution.

Uses shared_runner fixture for workflow execution.
"""

import pytest


class TestSimpleWorkflowExecution:
    """Test the simplest possible workflow execution."""
    
    def test_simple_workflow_execution(self, shared_runner):
        """Test executing a simple workflow with one task."""
        import cloaca
        
        # First verify WorkflowBuilder can be imported
        assert hasattr(cloaca, 'WorkflowBuilder')
        assert callable(cloaca.WorkflowBuilder)
        
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
        
        cloaca.register_workflow_constructor("simple_workflow", create_workflow)
        
        # Execute workflow using shared runner
        context = cloaca.Context({"input": "test"})
        result = shared_runner.execute("simple_workflow", context)
        
        # Verify execution
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context is not None
        # Note: final_context is the injected context, not the modified one