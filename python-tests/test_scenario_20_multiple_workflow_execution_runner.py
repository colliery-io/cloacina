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

        # Define tasks for different workflows
        @cloaca.task(id="workflow_a_task")
        def workflow_a_task(context):
            context.set("workflow_a_executed", True)
            return context

        @cloaca.task(id="workflow_b_task")
        def workflow_b_task(context):
            context.set("workflow_b_executed", True)
            return context

        @cloaca.task(id="workflow_c_task")
        def workflow_c_task(context):
            context.set("workflow_c_executed", True)
            return context

        # Create workflow builders
        def create_workflow_a():
            builder = cloaca.WorkflowBuilder("sequential_test_workflow_a")
            builder.description("Sequential test workflow A")
            builder.add_task("workflow_a_task")
            return builder.build()

        def create_workflow_b():
            builder = cloaca.WorkflowBuilder("sequential_test_workflow_b")
            builder.description("Sequential test workflow B")
            builder.add_task("workflow_b_task")
            return builder.build()

        def create_workflow_c():
            builder = cloaca.WorkflowBuilder("sequential_test_workflow_c")
            builder.description("Sequential test workflow C")
            builder.add_task("workflow_c_task")
            return builder.build()

        # Register all workflows
        cloaca.register_workflow_constructor("sequential_test_workflow_a", create_workflow_a)
        cloaca.register_workflow_constructor("sequential_test_workflow_b", create_workflow_b)
        cloaca.register_workflow_constructor("sequential_test_workflow_c", create_workflow_c)

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
