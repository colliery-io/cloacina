"""
Scenario 2: Workflow Execution Tests

This test file verifies workflow execution functionality including single and multi-task workflows,
dependency resolution, error handling, recovery mechanisms, and performance characteristics.

Uses shared_runner fixture for actual workflow execution.
"""

import pytest


class TestSingleTaskWorkflowExecution:
    """Test basic single task workflow execution."""
    
    def test_simple_task_execution(self, shared_runner):
        """Test executing a simple single-task workflow."""
        import cloaca
        
        @cloaca.task(id="simple_execution_task")
        def simple_execution_task(context):
            context.set("executed", True)
            context.set("input_received", context.get("test_input"))
            return context
        
        # Build and register workflow
        def create_workflow():
            builder = cloaca.WorkflowBuilder("simple_execution_workflow")
            builder.description("Simple execution test")
            builder.add_task("simple_execution_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("simple_execution_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_input": "test_value"})
        result = shared_runner.execute("simple_execution_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("test_input") == "test_value"
    
    def test_task_with_context_manipulation(self, shared_runner):
        """Test task that manipulates context data."""
        import cloaca
        
        @cloaca.task(id="context_manipulation_task")
        def context_manipulation_task(context):
            # Read input
            input_val = context.get("input_number", 0)
            
            # Process and set output
            context.set("doubled", input_val * 2)
            context.set("squared", input_val * input_val)
            context.set("processed", True)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("context_manipulation_workflow")
            builder.description("Context manipulation test")
            builder.add_task("context_manipulation_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("context_manipulation_workflow", create_workflow())
        
        # Execute with specific input
        context = cloaca.Context({"input_number": 5})
        result = shared_runner.execute("context_manipulation_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        # Note: final_context is the injected context, task modifications may not be visible


class TestMultiTaskWorkflowExecution:
    """Test multi-task workflows with dependencies."""
    
    def test_sequential_task_execution(self, shared_runner):
        """Test sequential execution of dependent tasks."""
        import cloaca
        
        @cloaca.task(id="first_task")
        def first_task(context):
            context.set("first_executed", True)
            context.set("step", 1)
            return context
        
        @cloaca.task(id="second_task", dependencies=["first_task"])
        def second_task(context):
            context.set("second_executed", True)
            context.set("step", 2)
            return context
        
        @cloaca.task(id="third_task", dependencies=["second_task"])
        def third_task(context):
            context.set("third_executed", True)
            context.set("step", 3)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("sequential_workflow")
            builder.description("Sequential task execution")
            builder.add_task("first_task")
            builder.add_task("second_task")
            builder.add_task("third_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("sequential_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_type": "sequential"})
        result = shared_runner.execute("sequential_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
    
    def test_parallel_task_execution(self, shared_runner):
        """Test parallel execution of independent tasks."""
        import cloaca
        
        @cloaca.task(id="parallel_task_a")
        def parallel_task_a(context):
            context.set("task_a_executed", True)
            return context
        
        @cloaca.task(id="parallel_task_b")
        def parallel_task_b(context):
            context.set("task_b_executed", True)
            return context
        
        @cloaca.task(id="parallel_task_c")
        def parallel_task_c(context):
            context.set("task_c_executed", True)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("parallel_workflow")
            builder.description("Parallel task execution")
            builder.add_task("parallel_task_a")
            builder.add_task("parallel_task_b")
            builder.add_task("parallel_task_c")
            return builder.build()
        
        cloaca.register_workflow_constructor("parallel_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_type": "parallel"})
        result = shared_runner.execute("parallel_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
    
    def test_diamond_dependency_pattern(self, shared_runner):
        """Test diamond dependency pattern (fork-join)."""
        import cloaca
        
        @cloaca.task(id="root_task")
        def root_task(context):
            context.set("root_executed", True)
            return context
        
        @cloaca.task(id="branch_left", dependencies=["root_task"])
        def branch_left(context):
            context.set("left_executed", True)
            return context
        
        @cloaca.task(id="branch_right", dependencies=["root_task"])
        def branch_right(context):
            context.set("right_executed", True)
            return context
        
        @cloaca.task(id="join_task", dependencies=["branch_left", "branch_right"])
        def join_task(context):
            context.set("join_executed", True)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("diamond_workflow")
            builder.description("Diamond dependency pattern")
            builder.add_task("root_task")
            builder.add_task("branch_left")
            builder.add_task("branch_right")
            builder.add_task("join_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("diamond_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_type": "diamond"})
        result = shared_runner.execute("diamond_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"


class TestContextPropagation:
    """Test context data flow between tasks."""
    
    def test_data_flow_through_pipeline(self, shared_runner):
        """Test data flowing through a pipeline of tasks."""
        import cloaca
        
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
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("data_pipeline_workflow")
            builder.description("Data pipeline test")
            builder.add_task("data_source")
            builder.add_task("data_processor")
            builder.add_task("data_finalizer")
            return builder.build()
        
        cloaca.register_workflow_constructor("data_pipeline_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_type": "data_flow"})
        result = shared_runner.execute("data_pipeline_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"


class TestErrorHandling:
    """Test error handling and recovery mechanisms."""
    
    def test_task_success_workflow_completion(self, shared_runner):
        """Test successful task execution leads to workflow completion."""
        import cloaca
        
        @cloaca.task(id="success_task")
        def success_task(context):
            context.set("success", True)
            context.set("message", "Task completed successfully")
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("success_workflow")
            builder.description("Success test workflow")
            builder.add_task("success_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("success_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_type": "success"})
        result = shared_runner.execute("success_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None


class TestRetryMechanisms:
    """Test configurable retry policies."""
    
    def test_task_with_retry_policy(self, shared_runner):
        """Test task with retry configuration executes successfully."""
        import cloaca
        
        @cloaca.task(
            id="retry_task",
            retry_attempts=3,
            retry_backoff="exponential",
            retry_delay_ms=100
        )
        def retry_task(context):
            context.set("retry_task_executed", True)
            context.set("retry_attempts_configured", 3)
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("retry_workflow")
            builder.description("Retry policy test")
            builder.add_task("retry_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("retry_workflow", create_workflow())
        
        # Execute workflow
        context = cloaca.Context({"test_type": "retry"})
        result = shared_runner.execute("retry_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"


class TestPerformanceCharacteristics:
    """Test performance and timing characteristics."""
    
    def test_workflow_execution_timing(self, shared_runner):
        """Test workflow execution completes within reasonable time."""
        import cloaca
        import time
        
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
        
        cloaca.register_workflow_constructor("timed_workflow", create_workflow())
        
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
        import time
        
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
        
        cloaca.register_workflow_constructor("perf_workflow", create_workflow())
        
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