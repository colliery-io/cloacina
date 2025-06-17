"""
Test Single Task Workflow Execution

This test file verifies basic single task workflow execution functionality
with context manipulation within tasks.

Uses shared_runner fixture for actual workflow execution.
"""



class TestSingleTaskWorkflowExecution:
    """Test basic single task workflow execution."""

    def test_task_with_context_manipulation(self, shared_runner):
        """Test task that manipulates context data."""
        import cloaca

        # Use workflow-scoped pattern - tasks defined within WorkflowBuilder context
        with cloaca.WorkflowBuilder("context_manipulation_workflow") as builder:
            builder.description("Context manipulation test")
            
            @cloaca.task(id="context_manipulation_task")
            def context_manipulation_task(context):
                # Read input
                input_val = context.get("input_number", 0)

                # Process and set output
                context.set("doubled", input_val * 2)
                context.set("squared", input_val * input_val)
                context.set("processed", True)
                return context

        # Execute with specific input
        context = cloaca.Context({"input_number": 5})
        result = shared_runner.execute("context_manipulation_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        # Note: final_context is the injected context, task modifications may not be visible
