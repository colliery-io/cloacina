"""
Test Context Passing with Shared Runner

This test file ensures context data flows correctly through execution
when using the shared runner fixture.

Uses shared_runner fixture to verify context handling.
"""



class TestContextPassingRunner:
    """Test context passing through shared runner."""

    def test_context_data_flow_through_runner(self, shared_runner):
        """Ensure context data flows correctly through execution."""
        import cloaca

        @cloaca.task(id="context_pass_task")
        def context_pass_task(context):
            # Read initial data
            initial_value = context.get("initial_data", "none")
            counter = context.get("counter", 0)

            # Modify context
            context.set("initial_data_received", initial_value)
            context.set("counter", counter + 1)
            context.set("processed_by", "shared_runner")

            # Add nested data
            context.set("nested_data", {
                "level1": {
                    "level2": {
                        "value": "deep_value"
                    }
                }
            })
            return context

        def create_workflow():
            builder = cloaca.WorkflowBuilder("context_pass_workflow")
            builder.description("Context passing test")
            builder.add_task("context_pass_task")
            return builder.build()

        cloaca.register_workflow_constructor("context_pass_workflow", create_workflow)

        # Execute with rich context
        context = cloaca.Context({
            "initial_data": "test_value",
            "counter": 5,
            "metadata": {"source": "test"}
        })
        result = shared_runner.execute("context_pass_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        # Verify initial context is preserved and modified correctly
        assert result.final_context.get("initial_data") == "test_value"
        assert result.final_context.get("counter") == 6  # incremented from 5 to 6
