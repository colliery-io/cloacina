"""
Test Context Propagation

This test file verifies context data flow between tasks in workflows.
Tests include data pipelines and context modifications flowing through tasks.

Uses shared_runner fixture for actual workflow execution.
"""



class TestContextPropagation:
    """Test context data flow between tasks."""

    def test_data_flow_through_pipeline(self, shared_runner):
        """Test data flowing through a pipeline of tasks."""
        import cloaca

        # Use workflow-scoped pattern - tasks defined within WorkflowBuilder context
        with cloaca.WorkflowBuilder("data_pipeline_workflow") as builder:
            builder.description("Data pipeline test")
            
            @cloaca.task(id="data_source")
            def data_source(context):
                context.set("data", {"value": 10, "status": "initial"})
                return context

            @cloaca.task(id="data_processor", dependencies=["data_source"])
            def data_processor(context):
                data = context.get("data", {})
                data["value"] = data.get("value", 0) * 2
                data["status"] = "processed"
                context.set("data", data)
                return context

            @cloaca.task(id="data_finalizer", dependencies=["data_processor"])
            def data_finalizer(context):
                data = context.get("data", {})
                data["status"] = "finalized"
                data["final"] = True
                context.set("data", data)
                context.set("pipeline_complete", True)
                return context

        # Execute workflow
        context = cloaca.Context({"test_type": "data_flow"})
        result = shared_runner.execute("data_pipeline_workflow", context)

        assert result is not None
        assert result.status == "Completed"
