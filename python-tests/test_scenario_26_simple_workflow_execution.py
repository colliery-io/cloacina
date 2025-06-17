"""
Test Simple Workflow Execution

This test file demonstrates the simplest possible workflow execution,
combining import verification and basic workflow execution.

Uses shared_runner fixture for workflow execution.
"""



class TestSimpleWorkflowExecution:
    """Test the simplest possible workflow execution."""

    def test_simple_workflow_execution(self, shared_runner):
        """Test executing a simple workflow with one task."""
        import cloaca

        # First verify WorkflowBuilder can be imported
        assert hasattr(cloaca, 'WorkflowBuilder')
        assert callable(cloaca.WorkflowBuilder)

        # Create workflow using context manager
        with cloaca.WorkflowBuilder("simple_workflow") as builder:
            builder.description("Simple workflow test")
            
            @cloaca.task(id="simple_task")
            def simple_task(context):
                context.set("task_executed", True)
                return context

        # Execute workflow using shared runner
        context = cloaca.Context({"input": "test"})
        result = shared_runner.execute("simple_workflow", context)

        # Verify execution
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context is not None
        # Note: final_context is the injected context, not the modified one
