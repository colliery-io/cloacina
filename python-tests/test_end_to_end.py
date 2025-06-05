"""
End-to-end test for workflow execution with the @workflow decorator.
This is a separate file to isolate potential deadlock issues.
"""
import pytest
import tempfile
import os


def test_end_to_end_workflow_execution():
    """Test complete end-to-end workflow execution using the @workflow decorator."""
    import cloaca
    import logging
    import sys
    
    # Enable detailed logging
    logging.basicConfig(
        level=logging.DEBUG,
        format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
        stream=sys.stdout
    )
    
    # Also try to enable Rust tracing if available
    import os
    os.environ['RUST_LOG'] = 'debug,cloacina=trace'
    
    # Create a temporary database for testing
    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
        db_path = tmp.name
    
    print(f"Using database: {db_path}")
    
    try:
        print("1. Defining task...")
        # Define a simple task
        @cloaca.task(id="e2e_task")
        def e2e_task(context):
            """Simple task that sets a value."""
            print("Task e2e_task executing!")
            context.set("task_executed", True)
            context.set("result", "e2e_success")
            return context
        
        print("2. Defining workflow...")
        # Define workflow using decorator
        @cloaca.workflow("e2e_workflow", "End-to-end test workflow")
        def create_e2e_workflow():
            """Create end-to-end test workflow."""
            print("Creating workflow instance...")
            builder = cloaca.WorkflowBuilder("e2e_workflow")
            builder.description("End-to-end test workflow")
            builder.add_task("e2e_task")
            workflow = builder.build()
            print(f"Workflow built: {workflow}")
            return workflow
        
        print("3. Creating runner...")
        # Create runner and execute workflow
        runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
        print("Runner created successfully")
        
        print("4. Creating context...")
        context = cloaca.Context()
        context.set("test_id", "e2e_001")
        print(f"Context created: {context}")
        
        print("5. Executing workflow...")
        # Execute the auto-registered workflow
        result = runner.execute("e2e_workflow", context)
        print("6. Execution completed!")
        
        # Verify execution was successful
        assert result is not None
        assert hasattr(result, 'status')
        assert hasattr(result, 'final_context')
        
        # Check the final context
        final_context = result.final_context
        assert final_context.get("test_id") == "e2e_001"
        assert final_context.get("task_executed") is True
        assert final_context.get("result") == "e2e_success"
        
        print(f"✓ End-to-end workflow execution completed!")
        print(f"  Status: {result.status}")
        print(f"  Result: {final_context.get('result')}")
        
    finally:
        # Clean up
        if os.path.exists(db_path):
            os.unlink(db_path)


def test_multi_task_workflow_execution():
    """Test workflow with multiple tasks and dependencies."""
    import cloaca
    
    # Create a temporary database for testing
    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
        db_path = tmp.name
    
    try:
        # Define tasks with dependencies
        @cloaca.task(id="step1")
        def step1_task(context):
            context.set("step1_data", "Hello")
            return context
        
        @cloaca.task(id="step2", dependencies=["step1"])
        def step2_task(context):
            value = context.get("step1_data")
            context.set("step2_data", f"{value} World")
            return context
        
        # Define workflow
        @cloaca.workflow("multi_step_workflow", "Multi-step workflow")
        def create_multi_step_workflow():
            builder = cloaca.WorkflowBuilder("multi_step_workflow")
            builder.description("Multi-step workflow")
            builder.add_task("step1")
            builder.add_task("step2")
            return builder.build()
        
        # Execute workflow
        runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
        context = cloaca.Context()
        
        result = runner.execute("multi_step_workflow", context)
        
        # Verify execution
        assert result is not None
        final_context = result.final_context
        assert final_context.get("step1_data") == "Hello"
        assert final_context.get("step2_data") == "Hello World"
        
        print("✓ Multi-task workflow execution completed!")
        
    finally:
        # Clean up
        if os.path.exists(db_path):
            os.unlink(db_path)


if __name__ == "__main__":
    test_end_to_end_workflow_execution()
    test_multi_task_workflow_execution()