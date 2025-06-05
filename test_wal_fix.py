#!/usr/bin/env python3
"""Test with proper SQLite WAL connection string."""
import sys
import os

print("Testing with WAL mode SQLite connection...")

try:
    print("Importing cloaca...")
    import cloaca
    print(f"Backend: {cloaca.get_backend()}")
    
    # Define a simple task
    @cloaca.task(id="wal_test_task")
    def wal_test_task(context):
        print("TASK EXECUTING WITH WAL!")
        context.set("task_executed", True)
        return context
    print("Task defined")
    
    # Create workflow manually
    def create_wal_test_workflow():
        builder = cloaca.WorkflowBuilder("wal_test_workflow")
        builder.description("Test workflow with WAL SQLite")
        builder.add_task("wal_test_task")
        return builder.build()
    
    # Register manually
    print("Registering workflow...")
    cloaca.register_workflow_constructor("wal_test_workflow", create_wal_test_workflow)
    print("Workflow registered")
    
    # Use the SAME connection string as the working Rust example
    db_path = "test_wal.db"
    if os.path.exists(db_path):
        os.remove(db_path)
    
    connection_string = f"sqlite://{db_path}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000"
    print(f"Creating runner with WAL: {connection_string}")
    
    runner = cloaca.DefaultRunner(connection_string)
    print("Runner created")
    
    print("Creating context...")
    context = cloaca.Context()
    context.set("wal_test", True)
    print("Context created")
    
    print("EXECUTING (should work with WAL!)...")
    sys.stdout.flush()
    
    result = runner.execute("wal_test_workflow", context)
    
    print("SUCCESS: Execution completed!")
    print(f"Result: {result}")
    print(f"Status: {result.status}")
    print(f"Task executed: {result.final_context.get('task_executed')}")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()