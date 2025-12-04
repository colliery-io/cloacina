"""
Test Simple Workflow with Context Manager

This test file verifies basic workflow creation and registration
using the context manager pattern with WorkflowBuilder.

Uses shared_runner fixture for workflow execution validation.
"""



class TestSimpleWorkflowContextManager:
    """Test simple workflow creation with context manager."""

    def test_workflow_context_manager_pattern(self, shared_runner):
        """Test basic workflow creation and registration with context manager."""
        import cloaca

        # Use context manager for workflow creation with workflow-scoped task definition
        with cloaca.WorkflowBuilder("context_manager_workflow") as builder:
            builder.description("Workflow created with context manager")
            builder.tag("pattern", "context_manager")
            builder.tag("test_type", "builder_pattern")

            # Define task within workflow scope - automatically added to workflow
            @cloaca.task(id="context_manager_task")
            def context_manager_task(context):
                context.set("context_manager_used", True)
                context.set("workflow_pattern", "builder")
                return context

        # Execute the workflow
        context = cloaca.Context({"test_input": "context_manager_test"})
        result = shared_runner.execute("context_manager_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
