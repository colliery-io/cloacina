"""
Test Basic Shared Runner Functionality

This test file verifies that the shared runner can execute simple workflows,
serving as a basic validation of the test harness and runner integration.

Uses shared_runner fixture to verify basic functionality.
"""



class TestBasicSharedRunnerFunctionality:
    """Test basic shared runner functionality."""

    def test_basic_shared_runner_execution(self, shared_runner):
        """Verify runner can execute a simple workflow."""
        import cloaca

        # Use workflow-scoped pattern - tasks defined within WorkflowBuilder context
        with cloaca.WorkflowBuilder("basic_runner_workflow") as builder:
            builder.description("Basic shared runner test")
            
            @cloaca.task(id="basic_runner_task")
            def basic_runner_task(context):
                context.set("runner_test_executed", True)
                context.set("runner_name", "shared_runner")
                return context

        # Execute workflow
        context = cloaca.Context({"test_type": "runner_basic"})
        result = shared_runner.execute("basic_runner_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
