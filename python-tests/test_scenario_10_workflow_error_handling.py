"""
Test Workflow Error Handling

This test file verifies error handling and recovery mechanisms in workflows.
Tests include successful completion, task failures, timeouts, and recovery strategies.

Uses shared_runner fixture for actual workflow execution.
"""

import pytest


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
        
        cloaca.register_workflow_constructor("success_workflow", create_workflow)
        
        # Execute workflow
        context = cloaca.Context({"test_type": "success"})
        result = shared_runner.execute("success_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.error_message is None