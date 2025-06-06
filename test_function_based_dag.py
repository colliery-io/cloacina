#!/usr/bin/env python3
"""Test function-based DAG definition instead of strings."""
import sys
import os

print("Testing function-based DAG definition...")

# Enable Rust logging  
os.environ['RUST_LOG'] = 'simple_example=debug,cloacina=debug'

try:
    print("1. Importing cloaca...")
    import cloaca
    
    print("2. Creating tasks...")
    
    @cloaca.task(id="task_a")
    def task_a(context):
        print("Task A executing...")
        context.set("a_result", "data_from_a")
        return context
    
    @cloaca.task(id="task_b", dependencies=["task_a"])  # String dependency (current way)
    def task_b(context):
        print("Task B executing...")
        a_data = context.get("a_result")
        context.set("b_result", f"b_processed_{a_data}")
        return context
    
    # TODO: This should work in the future (function-based dependencies)
    # @cloaca.task(id="task_c", dependencies=[task_a, task_b])  # Function dependencies (desired)
    # def task_c(context):
    #     print("Task C executing...")
    #     return context
    
    print("3. Building workflow with function references...")
    @cloaca.workflow("function_dag", "Test function-based DAG")
    def create_function_dag():
        builder = cloaca.WorkflowBuilder("function_dag")
        
        # Current approach: add tasks by string ID
        builder.add_task("task_a")
        builder.add_task("task_b")
        
        # TODO: Desired approach - add tasks by function reference
        # builder.add_task(task_a)
        # builder.add_task(task_b)
        # builder.add_task(task_c)
        
        return builder.build()
    
    print("4. Testing current string-based approach...")
    runner = cloaca.DefaultRunner("sqlite://function_dag_test.db?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000")
    context = cloaca.Context()
    context.set("test_id", "function_dag_001")
    
    result = runner.execute("function_dag", context)
    
    print("5. Execution completed!")
    print(f"   Status: {result.status}")
    
    final_context = result.final_context
    print("6. Results:")
    print(f"   A result: {final_context.get('a_result')}")
    print(f"   B result: {final_context.get('b_result')}")
    
    print("7. ✓ String-based DAG works!")
    print("\n8. TODO: Function-based DAG definition")
    print("   - Allow @task(dependencies=[func1, func2]) syntax")
    print("   - Allow builder.add_task(function) instead of builder.add_task('string')")
    print("   - Extract task IDs automatically from function names or decorators")
    
    # Clean up
    if os.path.exists("function_dag_test.db"):
        os.remove("function_dag_test.db")
        
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)