#!/usr/bin/env python3
"""Test complex workflow with task dependencies."""
import sys
import os

print("Testing complex workflow with dependencies...")

# Enable Rust logging
os.environ['RUST_LOG'] = 'simple_example=debug,cloacina=debug'

try:
    print("1. Importing cloaca...")
    import cloaca
    
    print("2. Creating dependent tasks...")
    
    @cloaca.task(id="setup_task")
    def setup_task(context):
        print("Setup task executing...")
        context.set("setup_data", "initialized")
        context.set("setup_complete", True)
        return context
    
    @cloaca.task(id="process_task", dependencies=["setup_task"])
    def process_task(context):
        print("Process task executing...")
        setup_data = context.get("setup_data")
        print(f"  Using setup data: {setup_data}")
        context.set("processed_data", f"processed_{setup_data}")
        return context
    
    @cloaca.task(id="finalize_task", dependencies=["process_task"])
    def finalize_task(context):
        print("Finalize task executing...")
        processed = context.get("processed_data")
        print(f"  Using processed data: {processed}")
        context.set("final_result", f"final_{processed}")
        return context
    
    # Parallel tasks that both depend on setup
    @cloaca.task(id="parallel_a", dependencies=["setup_task"])
    def parallel_a(context):
        print("Parallel task A executing...")
        context.set("parallel_a_result", "result_a")
        return context
    
    @cloaca.task(id="parallel_b", dependencies=["setup_task"])
    def parallel_b(context):
        print("Parallel task B executing...")
        context.set("parallel_b_result", "result_b")
        return context
    
    # Task that depends on both parallel tasks
    @cloaca.task(id="combine_task", dependencies=["parallel_a", "parallel_b"])
    def combine_task(context):
        print("Combine task executing...")
        result_a = context.get("parallel_a_result")
        result_b = context.get("parallel_b_result")
        context.set("combined_result", f"{result_a}+{result_b}")
        return context
    
    print("3. Building complex workflow...")
    @cloaca.workflow("complex_workflow", "Multi-task workflow with dependencies")
    def create_complex_workflow():
        builder = cloaca.WorkflowBuilder("complex_workflow")
        builder.description("Complex workflow testing dependencies and parallelism")
        
        # Add all tasks - dependencies will be handled automatically
        builder.add_task("setup_task")
        builder.add_task("process_task")
        builder.add_task("finalize_task")
        builder.add_task("parallel_a")
        builder.add_task("parallel_b")
        builder.add_task("combine_task")
        
        return builder.build()
    
    print("4. Creating runner...")
    runner = cloaca.DefaultRunner("sqlite://complex_test.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
    
    print("5. Creating context...")
    context = cloaca.Context()
    context.set("workflow_id", "complex_test_001")
    
    print("6. Executing complex workflow...")
    print("   Expected order: setup -> (process, parallel_a, parallel_b) -> finalize, combine")
    
    result = runner.execute("complex_workflow", context)
    
    print("7. Execution completed!")
    print(f"   Status: {result.status}")
    
    final_context = result.final_context
    print("8. Verifying results...")
    print(f"   Setup data: {final_context.get('setup_data')}")
    print(f"   Processed data: {final_context.get('processed_data')}")
    print(f"   Final result: {final_context.get('final_result')}")
    print(f"   Parallel A: {final_context.get('parallel_a_result')}")
    print(f"   Parallel B: {final_context.get('parallel_b_result')}")
    print(f"   Combined: {final_context.get('combined_result')}")
    
    # Verify correct execution order and data flow
    assert final_context.get("setup_data") == "initialized"
    assert final_context.get("processed_data") == "processed_initialized"
    assert final_context.get("final_result") == "final_processed_initialized"
    assert final_context.get("parallel_a_result") == "result_a"
    assert final_context.get("parallel_b_result") == "result_b"
    assert final_context.get("combined_result") == "result_a+result_b"
    
    print("9. ✓ Complex workflow test passed!")
    print("   - Dependencies respected")
    print("   - Parallel execution working")
    print("   - Data flow correct")
    
    # Clean up
    if os.path.exists("complex_test.db"):
        os.remove("complex_test.db")
        
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)