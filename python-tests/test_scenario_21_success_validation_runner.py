"""
Test Success Validation with Shared Runner

This test file verifies expected outcomes and status reporting
when using the shared runner fixture.

Uses shared_runner fixture to validate success scenarios.
"""

import pytest


class TestSuccessValidationRunner:
    """Test success validation and status reporting."""
    
    def test_workflow_success_status_reporting(self, shared_runner):
        """Verify expected outcomes and status reporting for successful workflows."""
        import cloaca
        import time
        
        @cloaca.task(id="success_validation_task")
        def success_validation_task(context):
            # Record execution details
            context.set("task_start_time", time.time())
            context.set("execution_successful", True)
            
            # Simulate some work
            data = context.get("input_data", [])
            processed_data = [item * 2 for item in data]
            context.set("processed_data", processed_data)
            
            # Set completion markers
            context.set("task_completed", True)
            context.set("task_end_time", time.time())
            
            return context
        
        def create_workflow():
            builder = cloaca.WorkflowBuilder("success_validation_workflow")
            builder.description("Success validation test")
            builder.add_task("success_validation_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("success_validation_workflow", create_workflow)
        
        # Execute with test data
        context = cloaca.Context({
            "input_data": [1, 2, 3, 4, 5],
            "expected_result": [2, 4, 6, 8, 10]
        })
        
        start_time = time.time()
        result = shared_runner.execute("success_validation_workflow", context)
        end_time = time.time()
        
        # Validate success status
        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None
        
        # Validate timing
        execution_time = end_time - start_time
        assert execution_time < 5.0  # Should complete quickly
        
        # Validate context preservation
        assert result.final_context is not None
        assert result.final_context.get("input_data") == [1, 2, 3, 4, 5]