#!/usr/bin/env python3
"""Minimal debug script."""
import sys
import os

print("Starting minimal debug...")
print(f"Python: {sys.version}")
print(f"Path: {sys.path[:3]}...")

# Add test env to path
test_env = "/Users/dstorey/Desktop/colliery/cloacina/test-env-sqlite/lib/python3.12/site-packages"
if os.path.exists(test_env):
    sys.path.insert(0, test_env)
    print(f"Added test env to path")

try:
    print("About to import cloaca...")
    import cloaca
    print("SUCCESS: Import worked!")
    print(f"Backend: {cloaca.get_backend()}")
    
    # Test WITHOUT decorator - manual registration
    print("\nTesting manual workflow registration...")
    
    # Define a task
    @cloaca.task(id="manual_task")
    def manual_task(context):
        print("Manual task executing!")
        context.set("executed", True)
        return context
    print("Task defined")
    
    # Create workflow manually
    def create_manual_workflow():
        builder = cloaca.WorkflowBuilder("manual_workflow")
        builder.description("Manually registered workflow")
        builder.add_task("manual_task")
        return builder.build()
    
    # Register manually
    print("Registering workflow manually...")
    cloaca.register_workflow_constructor("manual_workflow", create_manual_workflow)
    print("Workflow registered!")
    
    # Now try to execute
    import tempfile
    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
        db_path = tmp.name
    
    print(f"\nCreating runner with database: {db_path}")
    runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
    print("Runner created")
    
    print("\nCreating context...")
    context = cloaca.Context()
    context.set("test", "value")
    print("Context created")
    
    print("\nExecuting workflow (this might hang)...")
    print("HANGING POINT: About to call runner.execute()")
    sys.stdout.flush()  # Force output
    
    result = runner.execute("manual_workflow", context)
    
    print("SUCCESS: Execution completed!")
    print(f"Result: {result}")
    print(f"Status: {result.status}")
    
except Exception as e:
    print(f"FAILED: {e}")
    import traceback
    traceback.print_exc()