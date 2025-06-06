#!/usr/bin/env python3
"""Test creating fresh runner each time like Rust example."""
import sys
import os

print("Testing fresh runner creation pattern...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'debug,cloacina=trace'
os.environ['RUST_BACKTRACE'] = '1'

try:
    print("1. Importing cloaca...")
    import cloaca
    
    print("2. Creating simple task...")
    @cloaca.task(id="simple_task")
    def simple_task(context):
        print("Task executed!")
        context.set("task_result", "success")
        return context
    
    print("3. Building workflow with auto-registration...")
    @cloaca.workflow("test_workflow", "Minimal test workflow")
    def create_test_workflow():
        builder = cloaca.WorkflowBuilder("test_workflow")
        builder.add_task("simple_task")
        return builder.build()
    
    # Test 1: Create fresh runner each time
    print("4. Test 1: Creating fresh runner...")
    runner1 = cloaca.DefaultRunner("sqlite://test_fresh1.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
    context1 = cloaca.Context()
    context1.set("test_id", "fresh_test_1")
    
    print("5. Executing with fresh runner...")
    import signal
    def timeout_handler(signum, frame):
        raise TimeoutError("Execution timed out")
    
    signal.signal(signal.SIGALRM, timeout_handler)
    signal.alarm(10)  # 10 second timeout
    
    try:
        result1 = runner1.execute("test_workflow", context1)
        signal.alarm(0)
        print(f"   SUCCESS: Fresh runner execution completed: {result1}")
        
        # Clean up
        if os.path.exists("test_fresh1.db"):
            os.remove("test_fresh1.db")
            
    except TimeoutError:
        print("   FAILED: Fresh runner execution timed out")
        signal.alarm(0)
        
    print("6. Test completed")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)