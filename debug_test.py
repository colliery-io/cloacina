#!/usr/bin/env python3
"""Debug script to isolate the deadlock issue."""
import sys
import os
import tempfile

# Add the test environment's site-packages to Python path if needed
test_env_path = "/Users/dstorey/Desktop/colliery/cloacina/test-env-sqlite/lib/python3.12/site-packages"
if os.path.exists(test_env_path):
    sys.path.insert(0, test_env_path)

print("1. Starting debug test...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'debug,cloacina=trace'
os.environ['RUST_BACKTRACE'] = '1'

try:
    print("2. Importing cloaca...")
    import cloaca
    print(f"   Backend: {cloaca.get_backend()}")
    
    print("3. Defining task...")
    @cloaca.task(id="debug_task")
    def debug_task(context):
        print("   DEBUG: Task executing!")
        return context
    
    print("4. Defining workflow...")
    @cloaca.workflow("debug_workflow", "Debug workflow")
    def create_debug_workflow():
        print("   DEBUG: Creating workflow instance...")
        builder = cloaca.WorkflowBuilder("debug_workflow")
        builder.add_task("debug_task")
        return builder.build()
    
    print("5. Testing workflow creation directly...")
    workflow = create_debug_workflow()
    print(f"   Workflow created: {workflow}")
    
    print("6. Creating database...")
    with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
        db_path = tmp.name
    print(f"   Database path: {db_path}")
    
    print("7. Creating runner...")
    runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
    print("   Runner created successfully")
    
    print("8. Creating context...")
    context = cloaca.Context()
    print("   Context created")
    
    print("9. About to execute workflow...")
    print("   This is where we expect the hang to occur")
    
    # This should hang
    result = runner.execute("debug_workflow", context)
    
    print("10. UNEXPECTED: Execution completed!")
    print(f"    Result: {result}")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
finally:
    print("Debug test finished (or interrupted)")