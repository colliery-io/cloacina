"""
Test Workflow Performance Characteristics

This test file verifies comprehensive performance and timing characteristics
including single workflow timing and multiple sequential workflow executions.

Uses shared_runner fixture for actual workflow execution.
"""

import time


class TestPerformanceCharacteristics:
    """Test comprehensive performance and timing characteristics."""

    def test_comprehensive_workflow_performance(self, shared_runner):
        """Test comprehensive performance including timing and multiple executions."""
        import cloaca

        @cloaca.task(id="perf_task")
        def perf_task(context):
            execution_id = context.get("execution_id", 0)
            start_time = time.time()

            # Record timing information
            context.set("task_start_time", start_time)
            context.set("perf_task_executed", True)
            context.set("execution_id", execution_id)

            # Simulate some work
            time.sleep(0.01)  # 10ms of work

            end_time = time.time()
            context.set("task_end_time", end_time)
            context.set("task_duration", end_time - start_time)

            return context

        def create_workflow():
            builder = cloaca.WorkflowBuilder("comprehensive_perf_workflow")
            builder.description("Comprehensive performance test workflow")
            builder.add_task("perf_task")
            return builder.build()

        cloaca.register_workflow_constructor("comprehensive_perf_workflow", create_workflow)

        # Test 1: Single workflow execution timing
        print("\nTesting single workflow execution timing...")
        single_start_time = time.time()
        context = cloaca.Context({"execution_id": 0, "test_type": "single_timing"})
        result = shared_runner.execute("comprehensive_perf_workflow", context)
        single_execution_time = time.time() - single_start_time

        assert result is not None
        assert result.status == "Completed"
        assert single_execution_time < 5.0  # Single execution should complete within 5 seconds
        print(f"Single execution time: {single_execution_time:.3f}s")

        # Test 2: Multiple workflow executions performance
        print("\nTesting multiple workflow executions...")
        multi_start_time = time.time()
        results = []

        for i in range(5):
            context = cloaca.Context({"execution_id": i, "test_type": "multi_performance"})
            result = shared_runner.execute("comprehensive_perf_workflow", context)
            results.append(result)

        total_time = time.time() - multi_start_time
        avg_time = total_time / len(results)

        # Verify all executions succeeded
        assert len(results) == 5
        for i, result in enumerate(results):
            assert result is not None
            assert result.status == "Completed"
            assert result.error_message is None

        # Performance assertions
        assert total_time < 25.0  # Should complete all 5 within 25 seconds
        assert avg_time < 5.0     # Average time per execution should be under 5 seconds

        print(f"Total time for 5 executions: {total_time:.3f}s")
        print(f"Average time per execution: {avg_time:.3f}s")
        print("Performance test completed successfully!")
