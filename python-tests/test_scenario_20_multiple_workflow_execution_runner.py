"""
Test Multiple Workflow Execution with Shared Runner

This test file verifies running several workflows in sequence
using the shared runner fixture.

Uses shared_runner fixture to test sequential workflow execution.
"""



class TestMultipleWorkflowExecutionRunner:
    """Test multiple workflow execution in sequence."""

    def test_sequential_workflow_runs(self, shared_runner):
        """Run several workflows in sequence with shared runner."""
        import cloaca

        # Create workflow A using workflow-scoped pattern
        with cloaca.WorkflowBuilder("sequential_test_workflow_a") as builder:
            builder.description("Sequential test workflow A")

            @cloaca.task(id="workflow_a_task")
            def workflow_a_task(context):
                context.set("workflow_a_executed", True)
                return context

        # Create workflow B using workflow-scoped pattern
        with cloaca.WorkflowBuilder("sequential_test_workflow_b") as builder:
            builder.description("Sequential test workflow B")

            @cloaca.task(id="workflow_b_task")
            def workflow_b_task(context):
                context.set("workflow_b_executed", True)
                return context

        # Create workflow C using workflow-scoped pattern
        with cloaca.WorkflowBuilder("sequential_test_workflow_c") as builder:
            builder.description("Sequential test workflow C")

            @cloaca.task(id="workflow_c_task")
            def workflow_c_task(context):
                context.set("workflow_c_executed", True)
                return context

        # Execute workflows in sequence
        results = []

        # Run workflow A
        context_a = cloaca.Context({"workflow_id": "A"})
        result_a = shared_runner.execute("sequential_test_workflow_a", context_a)
        results.append(result_a)

        # Run workflow B
        context_b = cloaca.Context({"workflow_id": "B"})
        result_b = shared_runner.execute("sequential_test_workflow_b", context_b)
        results.append(result_b)

        # Run workflow C
        context_c = cloaca.Context({"workflow_id": "C"})
        result_c = shared_runner.execute("sequential_test_workflow_c", context_c)
        results.append(result_c)

        # Verify all workflows executed successfully
        assert len(results) == 3
        for result in results:
            assert result is not None
            assert result.status == "Completed"
            assert result.error_message is None
