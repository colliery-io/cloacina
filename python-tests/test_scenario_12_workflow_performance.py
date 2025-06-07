"""
Test Workflow Performance Characteristics

This test file verifies performance and timing characteristics of workflow execution.
Tests include execution timing, resource usage, and multiple workflow executions.

Uses shared_runner fixture for actual workflow execution.
"""

import pytest
import time


class TestPerformanceCharacteristics:
    """Test performance and timing characteristics."""
    
    def test_workflow_execution_timing(self, shared_runner):
        """Test workflow execution completes within reasonable time."""
        import cloaca
        
        @cloaca.task(id="timed_task")
        def timed_task(context):
            start_time = time.time()
            context.set("task_start_time", start_time)
            context.set("timed_task_executed", True)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("timed_workflow")
            builder.description("Timing test workflow")
            builder.add_task("timed_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("timed_workflow", create_workflow)
        
        # Execute workflow and measure time
        start_time = time.time()
        context = cloaca.Context({"test_type": "timing"})
        result = shared_runner.execute("timed_workflow", context)
        execution_time = time.time() - start_time
        
        assert result is not None
        assert result.status == "Completed"
        assert execution_time < 10.0  # Should complete within 10 seconds
        
    def test_multiple_workflows_performance(self, shared_runner):
        """Test executing multiple workflows in sequence performs well."""
        import cloaca
        
        @cloaca.task(id="perf_task")
        def perf_task(context):
            execution_id = context.get("execution_id", 0)
            context.set("perf_task_executed", True)
            context.set("execution_id", execution_id)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("perf_workflow")
            builder.description("Performance test workflow")
            builder.add_task("perf_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("perf_workflow", create_workflow)
        
        # Execute multiple workflows
        start_time = time.time()
        results = []
        
        for i in range(3):
            context = cloaca.Context({"execution_id": i, "test_type": "performance"})
            result = shared_runner.execute("perf_workflow", context)
            results.append(result)
        
        total_time = time.time() - start_time
        
        # Verify all executions succeeded
        assert len(results) == 3
        for result in results:
            assert result is not None
            assert result.status == "Completed"
        
        assert total_time < 30.0  # Should complete all within 30 seconds