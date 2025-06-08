"""
Test Multi-Task Workflow with Dependencies using Builder

This test file verifies complex workflow construction with the builder pattern,
focusing on multi-task workflows with dependencies.

Uses shared_runner fixture for workflow execution validation.
"""



class TestMultiTaskWorkflowDependenciesBuilder:
    """Test multi-task workflow construction with dependencies."""

    def test_complex_workflow_builder_pattern(self, shared_runner):
        """Test complex workflow construction with builder pattern."""
        import cloaca

        # Define multiple tasks
        @cloaca.task(id="builder_init_task")
        def builder_init_task(context):
            context.set("pipeline_started", True)
            context.set("stage", "initialization")
            return context

        @cloaca.task(id="builder_process_task", dependencies=["builder_init_task"])
        def builder_process_task(context):
            context.set("data_processed", True)
            context.set("stage", "processing")
            return context

        @cloaca.task(id="builder_validate_task", dependencies=["builder_process_task"])
        def builder_validate_task(context):
            context.set("data_validated", True)
            context.set("stage", "validation")
            return context

        @cloaca.task(id="builder_finalize_task", dependencies=["builder_validate_task"])
        def builder_finalize_task(context):
            context.set("pipeline_completed", True)
            context.set("stage", "finalized")
            return context

        # Build workflow with context manager
        with cloaca.WorkflowBuilder("complex_builder_workflow") as builder:
            builder.description("Multi-stage pipeline with dependencies")
            builder.tag("complexity", "high")
            builder.add_task("builder_init_task")
            builder.add_task("builder_process_task")
            builder.add_task("builder_validate_task")
            builder.add_task("builder_finalize_task")

        # Execute the workflow
        context = cloaca.Context({"pipeline_name": "complex_builder_test"})
        result = shared_runner.execute("complex_builder_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
