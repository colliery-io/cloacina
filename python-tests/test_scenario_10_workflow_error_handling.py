"""
Test Workflow Error Handling

This test file verifies error handling and recovery mechanisms in workflows.
Tests include successful completion, task failures, timeouts, and recovery strategies.

Uses shared_runner fixture for actual workflow execution.
"""



class TestErrorHandling:
    """Test error handling and recovery mechanisms."""

    def test_task_success_workflow_completion(self, shared_runner):
        """Test successful task execution leads to workflow completion."""
        import cloaca

        # Use workflow-scoped pattern - tasks defined within WorkflowBuilder context
        with cloaca.WorkflowBuilder("success_workflow") as builder:
            builder.description("Success test workflow")
            
            @cloaca.task(id="success_task")
            def success_task(context):
                context.set("success", True)
                context.set("message", "Task completed successfully")
                return context

        # Execute workflow
        context = cloaca.Context({"test_type": "success"})
        result = shared_runner.execute("success_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
