#!/usr/bin/env python3
"""Test Python bindings with async DAL."""
import sys
import os

print("Testing Python bindings with async DAL...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'debug,cloacina=trace'

try:
    print("1. Importing cloaca...")
    import cloaca
    print(f"   Backend: {cloaca.get_backend()}")
    
    print("2. Defining task...")
    @cloaca.task(id="async_dal_test_task")
    def async_dal_test_task(context):
        print("   TASK EXECUTING WITH ASYNC DAL!")
        context.set("task_executed", True)
        context.set("test", "async_dal_success")
        return context
    
    print("3. Creating workflow...")
    @cloaca.workflow("async_dal_test", "Test workflow with async DAL")
    def create_test_workflow():
        builder = cloaca.WorkflowBuilder("async_dal_test")
        builder.add_task("async_dal_test_task")
        return builder.build()
    
    print("4. Creating runner with SQLite WAL mode...")
    # Use WAL mode for better concurrency
    runner = cloaca.DefaultRunner("sqlite://test_async_dal.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
    print("   Runner created successfully")
    
    print("5. Creating context...")
    context = cloaca.Context()
    context.set("input", "test_value")
    
    print("6. Executing workflow (with async DAL)...")
    sys.stdout.flush()
    
    result = runner.execute("async_dal_test", context)
    
    print("7. SUCCESS! Workflow executed with async DAL")
    print(f"   Status: {result.status}")
    print(f"   Task executed: {result.final_context.get('task_executed')}")
    print(f"   Test result: {result.final_context.get('test')}")
    
    # Clean up
    if os.path.exists("test_async_dal.db"):
        os.remove("test_async_dal.db")
    
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)