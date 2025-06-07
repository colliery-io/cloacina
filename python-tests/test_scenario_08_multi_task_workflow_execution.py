"""
Test Multi-Task Workflow Execution

This test file verifies multi-task workflows with dependencies.
Tests include sequential execution, parallel execution, and complex dependency patterns.

Uses shared_runner fixture for actual workflow execution.
"""

import pytest


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
        
        cloaca.register_workflow_constructor("sequential_workflow", create_workflow)
        
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
        
        cloaca.register_workflow_constructor("parallel_workflow", create_workflow)
        
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
        
        cloaca.register_workflow_constructor("diamond_workflow", create_workflow)
        
        # Execute workflow
        context = cloaca.Context({"test_type": "diamond"})
        result = shared_runner.execute("diamond_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"