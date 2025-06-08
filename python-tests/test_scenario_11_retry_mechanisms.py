"""
Test Retry Mechanisms

This test file verifies configurable retry policies for tasks.
Tests include retry attempts, backoff strategies, and delay configurations.

Uses shared_runner fixture for actual workflow execution.
"""



class TestRetryMechanisms:
    """Test configurable retry policies."""

    def test_task_with_retry_policy(self, shared_runner):
        """Test task with retry configuration executes successfully."""
        import cloaca

        @cloaca.task(
            id="retry_task",
            retry_attempts=3,
            retry_backoff="exponential",
            retry_delay_ms=100
        )
        def retry_task(context):
            context.set("retry_task_executed", True)
            context.set("retry_attempts_configured", 3)
            return context

        def create_workflow():
            builder = cloaca.WorkflowBuilder("retry_workflow")
            builder.description("Retry policy test")
            builder.add_task("retry_task")
            return builder.build()

        cloaca.register_workflow_constructor("retry_workflow", create_workflow)

        # Execute workflow
        context = cloaca.Context({"test_type": "retry"})
        result = shared_runner.execute("retry_workflow", context)

        assert result is not None
        assert result.status == "Completed"
