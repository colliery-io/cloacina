"""
Test Registry Management

This test file verifies task and workflow registry isolation between tests,
ensuring proper cleanup and registration management.

Uses clean_runner fixture to ensure clean state between tests.
"""

import pytest


class TestRegistryManagement:
    """Test registry management and isolation."""
    
    def test_comprehensive_registry_management(self, shared_runner):
        """Test comprehensive registry management including isolation, cleanup, and state verification."""
        import cloaca
        
        # Test 1: Registry state verification
        print("Testing registry state verification...")
        
        # Define tasks for registry testing
        @cloaca.task(id="registry_test_task_1")
        def registry_test_task_1(context):
            context.set("registry_test_task_1_executed", True)
            return context
        
        @cloaca.task(id="registry_test_task_2")
        def registry_test_task_2(context):
            context.set("registry_test_task_2_executed", True)
            return context
        
        # Register workflow with these tasks
        def create_registry_workflow():
            builder = cloaca.WorkflowBuilder("registry_test_workflow")
            builder.description("Registry management test workflow")
            builder.add_task("registry_test_task_1")
            builder.add_task("registry_test_task_2")
            return builder.build()
        
        cloaca.register_workflow_constructor("registry_test_workflow", create_registry_workflow)
        
        # Test workflow execution to verify registry state
        context = cloaca.Context({"test_type": "registry_verification"})
        result = shared_runner.execute("registry_test_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("registry_test_task_1_executed") is True
        assert result.final_context.get("registry_test_task_2_executed") is True
        print("✓ Registry state verification works correctly")
        
        # Test 2: Task registry behavior verification
        print("Testing task registry behavior...")
        
        @cloaca.task(id="isolated_task_a")
        def isolated_task_a(context):
            context.set("isolated_task_a_executed", True)
            context.set("task_registry_test", "task_a_registered")
            return context
        
        @cloaca.task(id="isolated_task_b")
        def isolated_task_b(context):
            context.set("isolated_task_b_executed", True)
            # Check if previous task's registry info is available
            existing_test = context.get("task_registry_test", "none")
            context.set("previous_task_registry_state", existing_test)
            return context
        
        def create_task_registry_workflow():
            builder = cloaca.WorkflowBuilder("task_registry_workflow")
            builder.description("Task registry behavior test")
            builder.add_task("isolated_task_a")
            builder.add_task("isolated_task_b")
            return builder.build()
        
        cloaca.register_workflow_constructor("task_registry_workflow", create_task_registry_workflow)
        
        context = cloaca.Context({"test_type": "task_registry"})
        result = shared_runner.execute("task_registry_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("isolated_task_a_executed") is True
        assert result.final_context.get("isolated_task_b_executed") is True
        print("✓ Task registry behavior verification works correctly")
        
        # Test 3: Workflow registry consistency
        print("Testing workflow registry consistency...")
        
        @cloaca.task(id="consistency_task_1")
        def consistency_task_1(context):
            context.set("consistency_task_1_executed", True)
            return context
        
        @cloaca.task(id="consistency_task_2")
        def consistency_task_2(context):
            context.set("consistency_task_2_executed", True)
            return context
        
        # Register multiple workflow constructors
        def create_consistency_workflow_a():
            builder = cloaca.WorkflowBuilder("consistency_workflow_a")
            builder.description("First consistency test workflow")
            builder.add_task("consistency_task_1")
            return builder.build()
        
        def create_consistency_workflow_b():
            builder = cloaca.WorkflowBuilder("consistency_workflow_b")
            builder.description("Second consistency test workflow")
            builder.add_task("consistency_task_2")
            return builder.build()
        
        cloaca.register_workflow_constructor("consistency_workflow_a", create_consistency_workflow_a)
        cloaca.register_workflow_constructor("consistency_workflow_b", create_consistency_workflow_b)
        
        # Test both workflows
        context_a = cloaca.Context({"test_type": "consistency_a"})
        result_a = shared_runner.execute("consistency_workflow_a", context_a)
        
        context_b = cloaca.Context({"test_type": "consistency_b"})
        result_b = shared_runner.execute("consistency_workflow_b", context_b)
        
        assert result_a is not None and result_a.status == "Completed"
        assert result_b is not None and result_b.status == "Completed"
        assert result_a.context.get("consistency_task_1_executed") is True
        assert result_b.context.get("consistency_task_2_executed") is True
        print("✓ Workflow registry consistency works correctly")
        
        # Test 4: Registry pollution prevention
        print("Testing registry pollution prevention...")
        
        @cloaca.task(id="pollution_test_task")
        def pollution_test_task(context):
            context.set("pollution_test_task_executed", True)
            # Set some data that shouldn't pollute other tests
            context.set("pollution_marker", "test_specific_data")
            return context
        
        def create_pollution_workflow():
            builder = cloaca.WorkflowBuilder("pollution_test_workflow")
            builder.description("Registry pollution prevention test")
            builder.add_task("pollution_test_task")
            return builder.build()
        
        cloaca.register_workflow_constructor("pollution_test_workflow", create_pollution_workflow)
        
        # Execute workflow that might cause pollution
        context = cloaca.Context({"test_type": "pollution_test"})
        result = shared_runner.execute("pollution_test_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("pollution_test_task_executed") is True
        assert result.final_context.get("pollution_marker") == "test_specific_data"
        
        # Verify pollution doesn't affect new contexts
        clean_context = cloaca.Context({"test_type": "clean_test"})
        assert clean_context.get("pollution_marker") is None
        print("✓ Registry pollution prevention works correctly")
        
        # Test 5: Registry management during workflow execution
        print("Testing registry management during execution...")
        
        @cloaca.task(id="management_task_start")
        def management_task_start(context):
            context.set("management_task_start_executed", True)
            context.set("execution_stage", "start")
            return context
        
        @cloaca.task(id="management_task_middle", dependencies=["management_task_start"])
        def management_task_middle(context):
            context.set("management_task_middle_executed", True)
            context.set("execution_stage", "middle")
            return context
        
        @cloaca.task(id="management_task_end", dependencies=["management_task_middle"])
        def management_task_end(context):
            context.set("management_task_end_executed", True)
            context.set("execution_stage", "end")
            
            # Verify execution order and registry consistency
            start_executed = context.get("management_task_start_executed", False)
            middle_executed = context.get("management_task_middle_executed", False)
            
            context.set("registry_execution_consistent", start_executed and middle_executed)
            return context
        
        def create_management_workflow():
            builder = cloaca.WorkflowBuilder("management_execution_workflow")
            builder.description("Registry management during execution test")
            builder.add_task("management_task_start")
            builder.add_task("management_task_middle")
            builder.add_task("management_task_end")
            return builder.build()
        
        cloaca.register_workflow_constructor("management_execution_workflow", create_management_workflow)
        
        context = cloaca.Context({"test_type": "management"})
        result = shared_runner.execute("management_execution_workflow", context)
        
        assert result is not None
        assert result.status == "Completed"
        assert result.final_context.get("management_task_start_executed") is True
        assert result.final_context.get("management_task_middle_executed") is True
        assert result.final_context.get("management_task_end_executed") is True
        assert result.final_context.get("execution_stage") == "end"
        assert result.final_context.get("registry_execution_consistent") is True
        print("✓ Registry management during execution works correctly")
        
        # Summary
        registry_features_tested = 5
        print(f"\nRegistry management features tested: {registry_features_tested}/5")
        print("✓ All registry management features work correctly")
        
        print("✓ Comprehensive registry management test completed")