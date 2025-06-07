"""
Test Parameterized Workflows

This test file verifies workflows with configurable parameters,
demonstrating dynamic workflow construction based on input parameters.

Uses shared_runner fixture for workflow execution validation.
"""

import pytest


class TestParameterizedWorkflows:
    """Test workflows with configurable parameters."""
    
    def test_parameterized_workflow_construction(self, shared_runner):
        """Test workflows with configurable parameters."""
        import cloaca
        
        # Define parameterized task factory
        def create_parameterized_task(task_id, multiplier):
            @cloaca.task(id=task_id)
            def parameterized_task(context):
                input_value = context.get("input_value", 1)
                result = input_value * multiplier
                context.set(f"{task_id}_result", result)
                context.set(f"{task_id}_multiplier", multiplier)
                return context
            return parameterized_task
        
        # Create tasks with different parameters
        task_double = create_parameterized_task("param_task_double", 2)
        task_triple = create_parameterized_task("param_task_triple", 3)
        task_quadruple = create_parameterized_task("param_task_quadruple", 4)
        
        # Build parameterized workflow
        def create_parameterized_workflow(workflow_name, task_configs):
            with cloaca.WorkflowBuilder(workflow_name) as builder:
                builder.description(f"Parameterized workflow: {workflow_name}")
                builder.tag("type", "parameterized")
                
                # Add tasks based on configuration
                for task_id, multiplier in task_configs:
                    builder.add_task(task_id)
            
            return builder.build()
        
        # Configure and register workflow
        task_configs = [
            ("param_task_double", 2),
            ("param_task_triple", 3),
            ("param_task_quadruple", 4)
        ]
        
        workflow = create_parameterized_workflow("parameterized_test_workflow", task_configs)
        
        # Execute the workflow
        context = cloaca.Context({"input_value": 10})
        result = shared_runner.execute("parameterized_test_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None