#!/usr/bin/env python3
"""Minimal test of workflow execution with async DAL."""
import sys
import os

print("Testing minimal workflow execution...")

# Enable Rust logging at same level as working Rust example
os.environ['RUST_LOG'] = 'simple_example=debug,cloacina=debug'
os.environ['RUST_BACKTRACE'] = '1'

try:
    print("1. Importing cloaca...")
    import cloaca
    
    print("2. Creating context...")
    context = cloaca.Context()
    context.set("test_key", "test_value")
    
    print("3. Creating simple task...")
    @cloaca.task(id="simple_task")
    def simple_task(context):
        print("Task executed!")
        context.set("task_result", "success")
        return context
    
    print("4. Building workflow with auto-registration...")
    @cloaca.workflow("test_workflow", "Minimal test workflow")
    def create_test_workflow():
        builder = cloaca.WorkflowBuilder("test_workflow")
        builder.add_task("simple_task")  # Use task ID, not the function
        return builder.build()
    
    print("5. Creating runner...")
    runner = cloaca.DefaultRunner("sqlite://test_execution.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
    
    print("6. Executing workflow...")
    print("   This should complete quickly with async DAL...")
    
    # Set a timeout for this test
    import signal
    def timeout_handler(signum, frame):
        raise TimeoutError("Execution timed out after 10 seconds")
    
    signal.signal(signal.SIGALRM, timeout_handler)
    signal.alarm(10)  # 10 second timeout
    
    try:
        result = runner.execute("test_workflow", context)
        signal.alarm(0)  # Cancel timeout
        print("   Execution completed!")
        print(f"   Result: {result}")
        print(f"   Final context: {result.final_context}")
        
    except TimeoutError:
        print("   ERROR: Execution timed out - deadlock still present")
        sys.exit(1)
    
    # Clean up
    if os.path.exists("test_execution.db"):
        os.remove("test_execution.db")
        
    print("7. Success! Async DAL execution works")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)