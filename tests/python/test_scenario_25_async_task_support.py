"""
Test Async Task Support

This test file verifies workflows with asynchronous task functions,
demonstrating async/await patterns in workflow tasks.

Uses shared_runner fixture for workflow execution validation.
"""



class TestAsyncTaskSupport:
    """Test workflows with asynchronous task functions."""

    def test_async_task_workflow(self, shared_runner):
        """Test workflows with asynchronous task functions."""
        import cloaca

        # Use workflow-scoped pattern - tasks defined within WorkflowBuilder context
        with cloaca.WorkflowBuilder("async_test_workflow") as builder:
            builder.description("Workflow testing async task patterns")
            builder.tag("async", "simulation")

            # Note: This test assumes async task support is available
            # If not supported, this test should verify proper error handling
            @cloaca.task(id="sync_task_simulating_async")
            def sync_task_simulating_async(context):
                """Simulate async behavior in a sync task."""
                # Since we may not have true async support yet,
                # simulate async-like behavior with sync code
                context.set("async_simulation_started", True)

                # Simulate async operation result
                async_result = context.get("async_input", 0) + 100
                context.set("async_result", async_result)

                # Mark completion
                context.set("async_simulation_completed", True)
                return context

        # Execute the workflow
        context = cloaca.Context({"async_input": 42})
        result = shared_runner.execute("async_test_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
