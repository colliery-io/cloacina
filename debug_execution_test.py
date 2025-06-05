#!/usr/bin/env python3
"""Debug script to isolate the execution hang."""
import sys
import os
import time

print("Starting execution debug test...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'debug,cloacina=trace'
os.environ['RUST_BACKTRACE'] = '1'

try:
    print("Importing cloaca...")
    import cloaca
    print(f"Backend: {cloaca.get_backend()}")
    
    # Define a simple task
    @cloaca.task(id="test_task")
    def test_task(context):
        print("TASK EXECUTING!")
        context.set("task_executed", True)
        return context
    print("Task defined")
    
    # Create workflow manually (no decorator)
    def create_test_workflow():
        builder = cloaca.WorkflowBuilder("test_workflow")
        builder.description("Test workflow for debugging")
        builder.add_task("test_task")
        return builder.build()
    
    # Register manually
    print("Registering workflow...")
    cloaca.register_workflow_constructor("test_workflow", create_test_workflow)
    print("Workflow registered")
    
    # Use a persistent database file
    db_path = "debug_execution.db"
    if os.path.exists(db_path):
        os.remove(db_path)
    
    print(f"Creating runner with database: {db_path}")
    runner = cloaca.DefaultRunner(f"sqlite://{db_path}")
    print("Runner created")
    
    print("Creating context...")
    context = cloaca.Context()
    context.set("debug", True)
    print("Context created")
    
    # Check database before execution
    print("Checking database before execution...")
    os.system(f'sqlite3 {db_path} "SELECT COUNT(*) as pipeline_count FROM pipeline_executions;"')
    
    print("ABOUT TO EXECUTE - will hang here...")
    sys.stdout.flush()
    
    # Start execution in the background and monitor database
    import threading
    import signal
    
    def monitor_db():
        """Monitor database changes during execution"""
        time.sleep(2)  # Give execution time to start
        for i in range(10):  # Monitor for 20 seconds
            print(f"DB Monitor {i+1}: Checking database...")
            os.system(f'sqlite3 {db_path} "SELECT COUNT(*) as pipelines FROM pipeline_executions; SELECT COUNT(*) as tasks FROM task_executions;"')
            time.sleep(2)
    
    # Start monitoring thread
    monitor_thread = threading.Thread(target=monitor_db, daemon=True)
    monitor_thread.start()
    
    # Set a timeout for execution
    def timeout_handler(signum, frame):
        print("TIMEOUT: Execution took too long, checking final database state...")
        os.system(f'sqlite3 {db_path} "SELECT * FROM pipeline_executions;"')
        os.system(f'sqlite3 {db_path} "SELECT * FROM task_executions;"')
        sys.exit(1)
    
    signal.signal(signal.SIGALRM, timeout_handler)
    signal.alarm(20)  # 20 second timeout
    
    result = runner.execute("test_workflow", context)
    
    print("SUCCESS: Execution completed!")
    print(f"Result: {result}")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
finally:
    if os.path.exists(db_path):
        print("Final database state:")
        os.system(f'sqlite3 {db_path} "SELECT * FROM pipeline_executions;"')
        os.system(f'sqlite3 {db_path} "SELECT * FROM task_executions;"')