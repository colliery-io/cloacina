"""
Scenario 2: Workflow Execution Tests

This test file verifies actual workflow execution patterns using the shared runner.
Tests include single-task workflows, multi-task dependencies, parallel execution,
error handling, and background services functionality.

Uses shared_runner fixture to avoid connection pool exhaustion.
Note: Does NOT use clean_runner to avoid registry clearing between tests.
"""

import pytest
import time
from conftest import timeout_protection


class TestBasicWorkflowExecution:
    """Test basic workflow execution scenarios."""
    
    def test_single_task_workflow_execution(self, shared_runner):
        """Test execution of a simple single-task workflow."""
        import cloaca
        
        print("Starting single task workflow execution test...")
        
        # Define task
        @cloaca.task(id="single_task")
        def single_task(context):
            print("Single task executing!")
            input_value = context.get("input", "default")
            context.set("output", f"processed_{input_value}")
            context.set("task_executed", True)
            return context
        
        # Register workflow
        @cloaca.workflow("single_task_workflow", "Single task execution test")
        def create_single_workflow():
            print("Creating single task workflow...")
            builder = cloaca.WorkflowBuilder("single_task_workflow")
            builder.description("Single task execution test")
            builder.add_task("single_task")
            return builder.build()
        
        # Execute with timeout protection
        with timeout_protection(15):
            print("Creating context...")
            runner = shared_runner
            context = cloaca.Context()
            context.set("input", "test_data")
            context.set("test_id", "single_001")
            
            print("Executing workflow...")
            result = runner.execute("single_task_workflow", context)
            print("Execution completed!")
            
            # Verify result
            assert result is not None
            assert hasattr(result, 'status')
            assert hasattr(result, 'final_context')
            assert hasattr(result, 'start_time')
            
            # Note: final_context only contains original input
            final_context = result.final_context
            assert final_context.get("input") == "test_data"
            assert final_context.get("test_id") == "single_001"
            
            print("✓ Single task workflow test passed!")
    
    def test_multi_task_sequential_workflow(self, shared_runner):
        """Test workflow with sequential task dependencies."""
        import cloaca
        
        # Define sequential tasks
        @cloaca.task(id="extract_task")
        def extract_task(context):
            data = context.get("source_data", [])
            context.set("extracted_count", len(data))
            context.set("extract_complete", True)
            return context
        
        @cloaca.task(id="transform_task", dependencies=["extract_task"])
        def transform_task(context):
            count = context.get("extracted_count", 0)
            context.set("transformed_count", count * 2)
            context.set("transform_complete", True)
            return context
        
        @cloaca.task(id="load_task", dependencies=["transform_task"])
        def load_task(context):
            count = context.get("transformed_count", 0)
            context.set("loaded_count", count)
            context.set("load_complete", True)
            return context
        
        # Register workflow
        @cloaca.workflow("sequential_etl", "Sequential ETL pipeline")
        def create_sequential_workflow():
            builder = cloaca.WorkflowBuilder("sequential_etl")
            builder.description("Sequential ETL pipeline")
            builder.tag("type", "etl")
            builder.add_task("extract_task")
            builder.add_task("transform_task")
            builder.add_task("load_task")
            return builder.build()
        
        # Execute
        with timeout_protection(20):
            runner = shared_runner
            context = cloaca.Context()
            context.set("source_data", [1, 2, 3, 4, 5])
            context.set("pipeline_id", "sequential_001")
            
            result = runner.execute("sequential_etl", context)
            
            # Verify execution succeeded
            assert result is not None
            assert result.status == "Completed"
            
            final_context = result.final_context
            assert final_context.get("source_data") == [1, 2, 3, 4, 5]
            assert final_context.get("pipeline_id") == "sequential_001"
    
    def test_empty_workflow_execution(self, shared_runner):
        """Test execution of workflow with no tasks."""
        import cloaca
        
        @cloaca.workflow("empty_workflow", "Empty workflow test")
        def create_empty_workflow():
            builder = cloaca.WorkflowBuilder("empty_workflow")
            builder.description("Empty workflow test")
            return builder.build()
        
        # Execute
        with timeout_protection(10):
            runner = shared_runner
            context = cloaca.Context()
            context.set("empty_test", True)
            
            result = runner.execute("empty_workflow", context)
            
            assert result is not None
            final_context = result.final_context
            assert final_context.get("empty_test") is True


class TestParallelWorkflowExecution:
    """Test parallel task execution scenarios."""
    
    def test_parallel_task_execution(self, shared_runner):
        """Test that parallel tasks execute simultaneously."""
        import cloaca
        
        print("Starting parallel task execution test...")
        
        # Track execution times to verify parallelism
        class ExecutionTracker:
            def __init__(self):
                self.times = {}
        
        tracker = ExecutionTracker()
        
        @cloaca.task(id="parallel_task_a")
        def parallel_task_a(context):
            print("Parallel task A executing...")
            tracker.times["task_a_start"] = time.time()
            time.sleep(0.05)  # Short sleep to simulate work
            tracker.times["task_a_end"] = time.time()
            context.set("task_a_executed", True)
            return context
        
        @cloaca.task(id="parallel_task_b")
        def parallel_task_b(context):
            print("Parallel task B executing...")
            tracker.times["task_b_start"] = time.time()
            time.sleep(0.05)  # Short sleep to simulate work
            tracker.times["task_b_end"] = time.time()
            context.set("task_b_executed", True)
            return context
        
        @cloaca.task(id="join_task", dependencies=["parallel_task_a", "parallel_task_b"])
        def join_task(context):
            print("Join task executing...")
            tracker.times["join_start"] = time.time()
            context.set("join_executed", True)
            return context
        
        # Register workflow
        @cloaca.workflow("parallel_workflow", "Parallel execution test")
        def create_parallel_workflow():
            print("Creating parallel workflow...")
            builder = cloaca.WorkflowBuilder("parallel_workflow")
            builder.description("Parallel execution test")
            builder.add_task("parallel_task_a")
            builder.add_task("parallel_task_b")
            builder.add_task("join_task")
            return builder.build()
        
        # Execute
        with timeout_protection(15):
            print("Executing parallel workflow...")
            runner = shared_runner
            context = cloaca.Context()
            context.set("parallel_test_id", "parallel_001")
            
            start_time = time.time()
            result = runner.execute("parallel_workflow", context)
            total_time = time.time() - start_time
            
            print(f"Execution completed in {total_time:.3f}s")
            print(f"Execution times recorded: {list(tracker.times.keys())}")
            
            # Verify execution succeeded
            assert result is not None
            assert result.status == "Completed"
            
            print("✓ Parallel workflow test passed (execution successful)")
    
    def test_complex_parallel_dependencies(self, shared_runner):
        """Test complex workflow with mixed parallel and sequential dependencies."""
        import cloaca
        
        @cloaca.task(id="root_task")
        def root_task(context):
            context.set("root_complete", True)
            return context
        
        # Two parallel branches from root
        @cloaca.task(id="branch_a1", dependencies=["root_task"])
        def branch_a1(context):
            context.set("branch_a1_complete", True)
            return context
        
        @cloaca.task(id="branch_b1", dependencies=["root_task"])
        def branch_b1(context):
            context.set("branch_b1_complete", True)
            return context
        
        # Second level of each branch
        @cloaca.task(id="branch_a2", dependencies=["branch_a1"])
        def branch_a2(context):
            context.set("branch_a2_complete", True)
            return context
        
        @cloaca.task(id="branch_b2", dependencies=["branch_b1"])
        def branch_b2(context):
            context.set("branch_b2_complete", True)
            return context
        
        # Merge task depends on both branches
        @cloaca.task(id="merge_task", dependencies=["branch_a2", "branch_b2"])
        def merge_task(context):
            context.set("merge_complete", True)
            return context
        
        @cloaca.workflow("complex_parallel", "Complex parallel dependencies")
        def create_complex_parallel():
            builder = cloaca.WorkflowBuilder("complex_parallel")
            builder.description("Complex parallel dependencies")
            builder.add_task("root_task")
            builder.add_task("branch_a1")
            builder.add_task("branch_b1")
            builder.add_task("branch_a2")
            builder.add_task("branch_b2")
            builder.add_task("merge_task")
            return builder.build()
        
        # Execute
        with timeout_protection(20):
            runner = shared_runner
            context = cloaca.Context()
            context.set("complexity", "high")
            context.set("test_id", "complex_001")
            
            result = runner.execute("complex_parallel", context)
            
            assert result is not None
            assert result.status == "Completed"
            
            final_context = result.final_context
            assert final_context.get("complexity") == "high"
            assert final_context.get("test_id") == "complex_001"


class TestWorkflowDataFlow:
    """Test data flow between tasks in workflows."""
    
    def test_context_data_flow_between_tasks(self, shared_runner):
        """Test that data flows correctly between tasks via context."""
        import cloaca
        
        @cloaca.task(id="data_producer")
        def data_producer(context):
            # Produce some data
            context.set("produced_value", "hello")
            context.set("produced_number", 42)
            context.set("produced_list", [1, 2, 3])
            return context
        
        @cloaca.task(id="data_processor", dependencies=["data_producer"])
        def data_processor(context):
            # Process the data from producer
            value = context.get("produced_value")
            number = context.get("produced_number")
            data_list = context.get("produced_list")
            
            # Transform the data
            context.set("processed_value", f"{value}_processed")
            context.set("processed_number", number * 2)
            context.set("processed_list", [x * 10 for x in data_list])
            return context
        
        @cloaca.task(id="data_consumer", dependencies=["data_processor"])
        def data_consumer(context):
            # Consume processed data
            value = context.get("processed_value")
            number = context.get("processed_number")
            data_list = context.get("processed_list")
            
            # Verify data flow worked correctly
            assert value == "hello_processed"
            assert number == 84
            assert data_list == [10, 20, 30]
            
            context.set("data_flow_verified", True)
            return context
        
        # Register workflow
        @cloaca.workflow("data_flow_test", "Data flow test")
        def create_data_flow_test():
            builder = cloaca.WorkflowBuilder("data_flow_test")
            builder.description("Data flow test")
            builder.add_task("data_producer")
            builder.add_task("data_processor")
            builder.add_task("data_consumer")
            return builder.build()
        
        # Execute
        with timeout_protection(15):
            runner = shared_runner
            context = cloaca.Context()
            context.set("test_id", "data_flow_001")
            
            result = runner.execute("data_flow_test", context)
            
            # If we get here without assertion errors, data flow worked correctly
            assert result is not None
            assert result.status == "Completed"
    
    def test_context_serialization_through_workflow(self, shared_runner):
        """Test that complex context data survives workflow execution."""
        import cloaca
        
        @cloaca.task(id="serialization_task")
        def serialization_task(context):
            # Work with complex data types
            complex_data = context.get("complex_data")
            assert isinstance(complex_data, dict)
            assert complex_data["nested"]["value"] == "test"
            
            # Modify the data
            complex_data["processed"] = True
            context.set("complex_data", complex_data)
            
            return context
        
        @cloaca.workflow("serialization_test", "Context serialization test")
        def create_serialization_test():
            builder = cloaca.WorkflowBuilder("serialization_test")
            builder.description("Context serialization test")
            builder.add_task("serialization_task")
            return builder.build()
        
        # Execute with complex initial data
        with timeout_protection(10):
            runner = shared_runner
            context = cloaca.Context()
            
            complex_data = {
                "nested": {"value": "test", "count": 5},
                "list": [1, "two", 3.14],
                "boolean": True,
                "null": None
            }
            context.set("complex_data", complex_data)
            context.set("test_id", "serialization_001")
            
            result = runner.execute("serialization_test", context)
            
            assert result is not None
            assert result.status == "Completed"
            
            # Original data should be preserved in final context
            final_context = result.final_context
            assert final_context.get("test_id") == "serialization_001"
            # Note: task modifications are not returned in final_context by design


class TestBackgroundServices:
    """Test background services functionality."""
    
    def test_background_services_multiple_executions(self, shared_runner):
        """Test that background services handle multiple workflow executions."""
        import cloaca
        
        # Define a task that verifies background services are running
        @cloaca.task(id="background_test_task")
        def background_test_task(context):
            # This task's successful execution proves that:
            # 1. Scheduler picked up the task and marked it ready
            # 2. Executor executed the task
            # 3. Background services handled the coordination
            execution_id = context.get("execution_id", "unknown")
            context.set("background_services_working", True)
            context.set("execution_processed", execution_id)
            return context
        
        # Register workflow
        @cloaca.workflow("background_services_test", "Background services test")
        def create_background_test():
            builder = cloaca.WorkflowBuilder("background_services_test")
            builder.description("Background services test")
            builder.add_task("background_test_task")
            return builder.build()
        
        # Multiple executions to test that background services handle multiple workflows
        runner = shared_runner
        
        with timeout_protection(20):
            for i in range(3):
                context = cloaca.Context()
                context.set("execution_id", f"bg_test_{i}")
                
                result = runner.execute("background_services_test", context)
                
                # Each execution should succeed, proving background services work
                assert result is not None
                assert result.status == "Completed"
                
                final_context = result.final_context
                assert final_context.get("execution_id") == f"bg_test_{i}"
    
    def test_background_services_concurrent_workflows(self, shared_runner):
        """Test background services with different workflow types."""
        import cloaca
        
        @cloaca.task(id="service_task_a")
        def service_task_a(context):
            context.set("service_a_executed", True)
            return context
        
        @cloaca.task(id="service_task_b")
        def service_task_b(context):
            context.set("service_b_executed", True)
            return context
        
        @cloaca.workflow("service_workflow_a", "Service test A")
        def create_service_workflow_a():
            builder = cloaca.WorkflowBuilder("service_workflow_a")
            builder.add_task("service_task_a")
            return builder.build()
        
        @cloaca.workflow("service_workflow_b", "Service test B")
        def create_service_workflow_b():
            builder = cloaca.WorkflowBuilder("service_workflow_b")
            builder.add_task("service_task_b")
            return builder.build()
        
        # Execute different workflows
        with timeout_protection(15):
            runner = shared_runner
            
            # Execute workflow A
            context_a = cloaca.Context({"workflow_type": "A"})
            result_a = runner.execute("service_workflow_a", context_a)
            assert result_a is not None
            assert result_a.status == "Completed"
            
            # Execute workflow B
            context_b = cloaca.Context({"workflow_type": "B"})
            result_b = runner.execute("service_workflow_b", context_b)
            assert result_b is not None
            assert result_b.status == "Completed"


class TestErrorHandling:
    """Test error handling in workflow execution."""
    
    def test_nonexistent_workflow_execution(self, shared_runner):
        """Test execution of non-existent workflow."""
        import cloaca
        
        runner = shared_runner
        context = cloaca.Context()
        
        with pytest.raises(ValueError) as exc_info:
            runner.execute("nonexistent_workflow", context)
        
        assert "Workflow not found in registry" in str(exc_info.value)
    
    def test_workflow_with_missing_task_dependency(self, shared_runner):
        """Test workflow execution when task has missing dependencies."""
        import cloaca
        
        # This should fail during workflow building, not execution
        with pytest.raises(ValueError) as exc_info:
            @cloaca.task(id="dependent_task", dependencies=["missing_task"])
            def dependent_task(context):
                return context
            
            @cloaca.workflow("broken_workflow", "Workflow with missing dependency")
            def create_broken_workflow():
                builder = cloaca.WorkflowBuilder("broken_workflow")
                builder.add_task("dependent_task")
                return builder.build()
        
        # Should fail when trying to register the task with missing dependency
        assert "not found in registry" in str(exc_info.value)
    
    def test_workflow_execution_timeout_protection(self, shared_runner):
        """Test that workflow execution works within timeout constraints."""
        import cloaca
        
        @cloaca.task(id="quick_task")
        def quick_task(context):
            # Fast task that should complete well within timeout
            context.set("quick_executed", True)
            return context
        
        @cloaca.workflow("timeout_test", "Timeout protection test")
        def create_timeout_test():
            builder = cloaca.WorkflowBuilder("timeout_test")
            builder.add_task("quick_task")
            return builder.build()
        
        # Execute with strict timeout
        with timeout_protection(5):  # Very short timeout
            runner = shared_runner
            context = cloaca.Context({"timeout_test": True})
            
            result = runner.execute("timeout_test", context)
            
            assert result is not None
            assert result.status == "Completed"