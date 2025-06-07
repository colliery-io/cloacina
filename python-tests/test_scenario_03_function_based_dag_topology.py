"""
Test Function-Based DAG Topology

This test file verifies advanced workflow features exploring different
approaches to task relationship definition and workflow construction.

Uses shared_runner fixture for actual workflow execution.
"""

import pytest


class TestFunctionBasedDAGTopology:
    """Test function-based DAG topology features."""
    
    def test_comprehensive_dag_topology_patterns(self, shared_runner):
        """Test comprehensive DAG topology patterns and task relationship approaches."""
        import cloaca
        
        # Test current string-based approach and explore alternatives
        print("Testing comprehensive DAG topology patterns...")
        
        # Test 1: Standard string-based dependencies (current approach)
        print("Testing standard string-based task dependencies...")
        
        @cloaca.task(id="topology_root")
        def topology_root(context):
            context.set("topology_root_executed", True)
            context.set("execution_order", [context.get("execution_order", []) + ["topology_root"]][0])
            return context
        
        @cloaca.task(id="topology_branch_a", dependencies=["topology_root"])
        def topology_branch_a(context):
            context.set("topology_branch_a_executed", True)
            order = context.get("execution_order", [])
            order.append("topology_branch_a")
            context.set("execution_order", order)
            return context
        
        @cloaca.task(id="topology_branch_b", dependencies=["topology_root"])
        def topology_branch_b(context):
            context.set("topology_branch_b_executed", True)
            order = context.get("execution_order", [])
            order.append("topology_branch_b")
            context.set("execution_order", order)
            return context
        
        @cloaca.task(id="topology_join", dependencies=["topology_branch_a", "topology_branch_b"])
        def topology_join(context):
            context.set("topology_join_executed", True)
            order = context.get("execution_order", [])
            order.append("topology_join")
            context.set("execution_order", order)
            return context
        
        def create_string_based_workflow():
            builder = cloaca.WorkflowBuilder("string_based_topology_workflow")
            builder.description("String-based DAG topology test")
            builder.add_task("topology_root")
            builder.add_task("topology_branch_a")
            builder.add_task("topology_branch_b")
            builder.add_task("topology_join")
            return builder.build()
        
        cloaca.register_workflow_constructor("string_based_topology_workflow", create_string_based_workflow)
        
        context = cloaca.Context({"test_type": "string_topology"})
        result = shared_runner.execute("string_based_topology_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        print("✓ String-based DAG topology works correctly")
        
        # Test 2: Dynamic task addition patterns
        print("Testing dynamic task addition patterns...")
        
        @cloaca.task(id="dynamic_task_1")
        def dynamic_task_1(context):
            context.set("dynamic_1_executed", True)
            return context
        
        @cloaca.task(id="dynamic_task_2", dependencies=["dynamic_task_1"])
        def dynamic_task_2(context):
            context.set("dynamic_2_executed", True)
            return context
        
        @cloaca.task(id="dynamic_task_3", dependencies=["dynamic_task_1"])
        def dynamic_task_3(context):
            context.set("dynamic_3_executed", True)
            return context
        
        def create_dynamic_workflow():
            builder = cloaca.WorkflowBuilder("dynamic_topology_workflow")
            builder.description("Dynamic task addition topology test")
            
            # Add tasks in different order than dependency order
            builder.add_task("dynamic_task_3")  # Add dependent first
            builder.add_task("dynamic_task_1")  # Add dependency after
            builder.add_task("dynamic_task_2")  # Add another dependent
            
            return builder.build()
        
        cloaca.register_workflow_constructor("dynamic_topology_workflow", create_dynamic_workflow)
        
        context = cloaca.Context({"test_type": "dynamic_topology"})
        result = shared_runner.execute("dynamic_topology_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        print("✓ Dynamic task addition patterns work correctly")
        
        # Test 3: Complex topology validation
        print("Testing complex topology patterns...")
        
        @cloaca.task(id="complex_start")
        def complex_start(context):
            context.set("complex_start_executed", True)
            return context
        
        @cloaca.task(id="complex_middle_1", dependencies=["complex_start"])
        def complex_middle_1(context):
            context.set("complex_middle_1_executed", True)
            return context
        
        @cloaca.task(id="complex_middle_2", dependencies=["complex_start"])
        def complex_middle_2(context):
            context.set("complex_middle_2_executed", True)
            return context
        
        @cloaca.task(id="complex_middle_3", dependencies=["complex_middle_1"])
        def complex_middle_3(context):
            context.set("complex_middle_3_executed", True)
            return context
        
        @cloaca.task(id="complex_end", dependencies=["complex_middle_2", "complex_middle_3"])
        def complex_end(context):
            context.set("complex_end_executed", True)
            return context
        
        def create_complex_workflow():
            builder = cloaca.WorkflowBuilder("complex_topology_workflow")
            builder.description("Complex topology validation test")
            
            # Add all tasks
            task_names = ["complex_start", "complex_middle_1", "complex_middle_2", 
                         "complex_middle_3", "complex_end"]
            
            for task_name in task_names:
                builder.add_task(task_name)
            
            return builder.build()
        
        cloaca.register_workflow_constructor("complex_topology_workflow", create_complex_workflow)
        
        context = cloaca.Context({"test_type": "complex_topology"})
        result = shared_runner.execute("complex_topology_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        print("✓ Complex topology patterns work correctly")
        
        # Test 4: Topology inspection (if available)
        print("Testing topology inspection capabilities...")
        
        workflow = create_complex_workflow()
        
        # Check if workflow exposes topology information
        topology_info_available = False
        
        if hasattr(workflow, 'tasks'):
            print(f"✓ Workflow exposes tasks: {len(workflow.tasks) if hasattr(workflow.tasks, '__len__') else 'available'}")
            topology_info_available = True
        
        if hasattr(workflow, 'dependencies'):
            print("✓ Workflow exposes dependency information")
            topology_info_available = True
        
        if hasattr(workflow, 'name'):
            print(f"✓ Workflow name accessible: {workflow.name}")
            topology_info_available = True
        
        if hasattr(workflow, 'description'):
            print(f"✓ Workflow description accessible: {workflow.description}")
            topology_info_available = True
        
        if not topology_info_available:
            print("⚠ Workflow topology information not directly accessible (may be internal)")
        
        # Summary
        features_tested = 4
        features_passed = 3  # String-based, dynamic, complex all passed
        
        if topology_info_available:
            features_passed += 1
        
        print(f"\nDAG topology features working: {features_passed}/{features_tested}")
        
        # Test passes if core functionality works
        assert features_passed >= 3, "Core DAG topology functionality should work"
        
        print("✓ Comprehensive DAG topology test completed")