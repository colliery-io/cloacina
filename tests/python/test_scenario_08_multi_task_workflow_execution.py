"""
Test Multi-Task Workflow Execution

This test file verifies a comprehensive multi-task workflow that demonstrates
sequential execution, parallel execution, and diamond dependency patterns
all combined in a single complex DAG.

Uses shared_runner fixture for actual workflow execution.
"""



class TestMultiTaskWorkflowExecution:
    """Test comprehensive multi-task workflow with complex dependencies."""

    def test_comprehensive_multi_pattern_workflow(self, shared_runner):
        """Test a comprehensive workflow combining sequential, parallel, and diamond patterns."""
        import cloaca

        # Use workflow-scoped pattern - tasks defined within WorkflowBuilder context
        with cloaca.WorkflowBuilder("comprehensive_multi_pattern_workflow") as builder:
            builder.description("Comprehensive workflow with sequential, parallel, and diamond patterns")

            # Sequential start
            @cloaca.task(id="init_task")
            def init_task(context):
                context.set("workflow_started", True)
                context.set("init_step", "completed")
                return context

            @cloaca.task(id="prepare_task", dependencies=["init_task"])
            def prepare_task(context):
                context.set("preparation_done", True)
                context.set("prepare_step", "completed")
                return context

            # Parallel fan-out from prepare_task
            @cloaca.task(id="parallel_task_a", dependencies=["prepare_task"])
            def parallel_task_a(context):
                context.set("parallel_a_executed", True)
                return context

            @cloaca.task(id="parallel_task_b", dependencies=["prepare_task"])
            def parallel_task_b(context):
                context.set("parallel_b_executed", True)
                return context

            @cloaca.task(id="parallel_task_c", dependencies=["prepare_task"])
            def parallel_task_c(context):
                context.set("parallel_c_executed", True)
                return context

            # Diamond pattern: convergence to processing, then split, then join
            @cloaca.task(id="process_task", dependencies=["parallel_task_a", "parallel_task_b", "parallel_task_c"])
            def process_task(context):
                context.set("processing_done", True)
                context.set("diamond_root", "completed")
                return context

            # Diamond split
            @cloaca.task(id="analyze_left", dependencies=["process_task"])
            def analyze_left(context):
                context.set("left_analysis", True)
                return context

            @cloaca.task(id="analyze_right", dependencies=["process_task"])
            def analyze_right(context):
                context.set("right_analysis", True)
                return context

            # Diamond join and final sequential steps
            @cloaca.task(id="combine_results", dependencies=["analyze_left", "analyze_right"])
            def combine_results(context):
                context.set("results_combined", True)
                context.set("diamond_join", "completed")
                return context

            @cloaca.task(id="finalize_task", dependencies=["combine_results"])
            def finalize_task(context):
                context.set("workflow_complete", True)
                context.set("final_step", "completed")
                return context

        # Execute the comprehensive workflow
        context = cloaca.Context({"test_type": "comprehensive_multi_pattern"})
        result = shared_runner.execute("comprehensive_multi_pattern_workflow", context)

        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
