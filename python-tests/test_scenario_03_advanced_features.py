"""
Scenario 3: Advanced Features Tests

This test file verifies advanced workflow features including function-based DAG topology,
complex dependency chains, trigger rules, workflow versioning, and registry management.

Uses clean_runner fixture to ensure clean state between tests.
"""

import pytest
from conftest import timeout_protection


class TestFunctionBasedDAGTopology:
    """Test function-based DAG topology definition vs string-based."""
    
    def test_task_decorator_without_explicit_id(self, clean_runner):
        """Test that task decorator can auto-generate ID from function name."""
        import cloaca
        
        # Task without explicit ID should use function name
        @cloaca.task()
        def auto_id_task(context):
            context.set("auto_id_executed", True)
            return context
        
        # Build workflow using the auto-generated ID
        @cloaca.workflow("auto_id_test", "Auto ID test")
        def create_auto_id_test():
            builder = cloaca.WorkflowBuilder("auto_id_test")
            builder.add_task("auto_id_task")  # Should work with auto-generated ID
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"auto_id_test": True})
            result = runner.execute("auto_id_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_function_references_in_dependencies(self, clean_runner):
        """Test using function references instead of strings in dependencies."""
        import cloaca
        
        # First task - will be referenced by function
        @cloaca.task()
        def producer_task(context):
            context.set("produced_data", "test_value")
            return context
        
        # Second task - uses function reference in dependencies
        @cloaca.task(dependencies=[producer_task])
        def consumer_task(context):
            data = context.get("produced_data")
            assert data == "test_value"
            context.set("consumed", True)
            return context
        
        @cloaca.workflow("function_deps_test", "Function dependencies test")
        def create_function_deps_test():
            builder = cloaca.WorkflowBuilder("function_deps_test")
            builder.add_task("producer_task")
            builder.add_task("consumer_task")
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"function_deps_test": True})
            result = runner.execute("function_deps_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_workflow_builder_with_function_references(self, clean_runner):
        """Test WorkflowBuilder.add_task() with function references."""
        import cloaca
        
        @cloaca.task()
        def step_one(context):
            context.set("step_one_done", True)
            return context
        
        @cloaca.task()
        def step_two(context):
            context.set("step_two_done", True)
            return context
        
        @cloaca.workflow("function_refs_test", "Function references test")
        def create_function_refs_test():
            builder = cloaca.WorkflowBuilder("function_refs_test")
            # Add tasks using function references instead of strings
            builder.add_task(step_one)   # Function reference
            builder.add_task(step_two)   # Function reference
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"function_refs_test": True})
            result = runner.execute("function_refs_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_mixed_string_and_function_dependencies(self, clean_runner):
        """Test mixing string and function references in dependencies."""
        import cloaca
        
        # Task with explicit string ID
        @cloaca.task(id="string_id_task")
        def task_with_string_id(context):
            context.set("string_task_done", True)
            return context
        
        # Task with auto-generated ID
        @cloaca.task()
        def function_id_task(context):
            context.set("function_task_done", True)
            return context
        
        # Task that depends on both using mixed references
        @cloaca.task(dependencies=["string_id_task", function_id_task])
        def mixed_deps_task(context):
            # Verify both dependencies executed
            assert context.get("string_task_done") is True
            assert context.get("function_task_done") is True
            context.set("mixed_deps_done", True)
            return context
        
        @cloaca.workflow("mixed_deps_test", "Mixed dependencies test")
        def create_mixed_deps_test():
            builder = cloaca.WorkflowBuilder("mixed_deps_test")
            builder.add_task("string_id_task")
            builder.add_task(function_id_task)  # Function reference
            builder.add_task("mixed_deps_task")
            return builder.build()
        
        with timeout_protection(15):
            runner = clean_runner
            context = cloaca.Context({"mixed_deps_test": True})
            result = runner.execute("mixed_deps_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_complex_function_based_dag(self, clean_runner):
        """Test complex DAG with multiple function references and dependencies."""
        import cloaca
        
        @cloaca.task()
        def extract_data(context):
            context.set("raw_data", [1, 2, 3, 4, 5])
            return context
        
        @cloaca.task(dependencies=[extract_data])
        def validate_data(context):
            raw_data = context.get("raw_data")
            assert len(raw_data) == 5
            context.set("validation_passed", True)
            return context
        
        @cloaca.task(dependencies=[extract_data])
        def transform_data(context):
            raw_data = context.get("raw_data")
            transformed = [x * 2 for x in raw_data]
            context.set("transformed_data", transformed)
            return context
        
        @cloaca.task(dependencies=[validate_data, transform_data])
        def load_data(context):
            validation = context.get("validation_passed")
            transformed = context.get("transformed_data")
            assert validation is True
            assert transformed == [2, 4, 6, 8, 10]
            context.set("load_complete", True)
            return context
        
        @cloaca.workflow("complex_function_dag", "Complex function-based DAG")
        def create_complex_dag():
            builder = cloaca.WorkflowBuilder("complex_function_dag")
            # All tasks added using function references
            builder.add_task(extract_data)
            builder.add_task(validate_data)
            builder.add_task(transform_data)
            builder.add_task(load_data)
            return builder.build()
        
        with timeout_protection(15):
            runner = clean_runner
            context = cloaca.Context({"complex_dag_test": True})
            result = runner.execute("complex_function_dag", context)
            
            assert result is not None
            assert result.status == "Completed"


class TestComplexDependencyChains:
    """Test complex dependency chain scenarios."""
    
    def test_diamond_dependency_pattern(self, clean_runner):
        """Test diamond-shaped dependency pattern (common convergence)."""
        import cloaca
        
        @cloaca.task()
        def root_task(context):
            context.set("root_value", "root_completed")
            return context
        
        # Two tasks that depend on root (parallel)
        @cloaca.task(dependencies=[root_task])
        def left_branch(context):
            root_val = context.get("root_value")
            context.set("left_result", f"left_processed_{root_val}")
            return context
        
        @cloaca.task(dependencies=[root_task])
        def right_branch(context):
            root_val = context.get("root_value")
            context.set("right_result", f"right_processed_{root_val}")
            return context
        
        # Task that depends on both branches (convergence)
        @cloaca.task(dependencies=[left_branch, right_branch])
        def convergence_task(context):
            left_result = context.get("left_result")
            right_result = context.get("right_result")
            
            assert "left_processed_root_completed" in left_result
            assert "right_processed_root_completed" in right_result
            
            context.set("convergence_complete", True)
            return context
        
        @cloaca.workflow("diamond_pattern", "Diamond dependency pattern")
        def create_diamond_pattern():
            builder = cloaca.WorkflowBuilder("diamond_pattern")
            builder.add_task(root_task)
            builder.add_task(left_branch)
            builder.add_task(right_branch)
            builder.add_task(convergence_task)
            return builder.build()
        
        with timeout_protection(15):
            runner = clean_runner
            context = cloaca.Context({"diamond_test": True})
            result = runner.execute("diamond_pattern", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_multi_level_dependency_chain(self, clean_runner):
        """Test deep dependency chains with multiple levels."""
        import cloaca
        
        @cloaca.task()
        def level_1_task(context):
            context.set("level_1_value", 1)
            return context
        
        @cloaca.task(dependencies=[level_1_task])
        def level_2_task(context):
            prev_val = context.get("level_1_value")
            context.set("level_2_value", prev_val + 1)
            return context
        
        @cloaca.task(dependencies=[level_2_task])
        def level_3_task(context):
            prev_val = context.get("level_2_value")
            context.set("level_3_value", prev_val + 1)
            return context
        
        @cloaca.task(dependencies=[level_3_task])
        def level_4_task(context):
            prev_val = context.get("level_3_value")
            context.set("level_4_value", prev_val + 1)
            return context
        
        @cloaca.task(dependencies=[level_4_task])
        def final_verification_task(context):
            # Verify the chain executed correctly
            assert context.get("level_1_value") == 1
            assert context.get("level_2_value") == 2
            assert context.get("level_3_value") == 3
            assert context.get("level_4_value") == 4
            context.set("chain_complete", True)
            return context
        
        @cloaca.workflow("multi_level_chain", "Multi-level dependency chain")
        def create_multi_level_chain():
            builder = cloaca.WorkflowBuilder("multi_level_chain")
            builder.add_task(level_1_task)
            builder.add_task(level_2_task)
            builder.add_task(level_3_task)
            builder.add_task(level_4_task)
            builder.add_task(final_verification_task)
            return builder.build()
        
        with timeout_protection(15):
            runner = clean_runner
            context = cloaca.Context({"chain_test": True})
            result = runner.execute("multi_level_chain", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_fan_out_fan_in_pattern(self, clean_runner):
        """Test fan-out followed by fan-in dependency pattern."""
        import cloaca
        
        @cloaca.task()
        def source_task(context):
            context.set("source_data", [1, 2, 3, 4, 5])
            return context
        
        # Fan out: multiple tasks depend on source
        @cloaca.task(dependencies=[source_task])
        def process_task_1(context):
            data = context.get("source_data")
            context.set("result_1", sum(data))  # Sum
            return context
        
        @cloaca.task(dependencies=[source_task])
        def process_task_2(context):
            data = context.get("source_data")
            context.set("result_2", len(data))  # Count
            return context
        
        @cloaca.task(dependencies=[source_task])
        def process_task_3(context):
            data = context.get("source_data")
            context.set("result_3", max(data))  # Max
            return context
        
        @cloaca.task(dependencies=[source_task])
        def process_task_4(context):
            data = context.get("source_data")
            context.set("result_4", min(data))  # Min
            return context
        
        # Fan in: single task depends on all processors
        @cloaca.task(dependencies=[process_task_1, process_task_2, process_task_3, process_task_4])
        def aggregation_task(context):
            sum_result = context.get("result_1")
            count_result = context.get("result_2")
            max_result = context.get("result_3")
            min_result = context.get("result_4")
            
            # Verify all processors executed correctly
            assert sum_result == 15  # 1+2+3+4+5
            assert count_result == 5
            assert max_result == 5
            assert min_result == 1
            
            context.set("aggregation_complete", True)
            return context
        
        @cloaca.workflow("fan_out_fan_in", "Fan-out fan-in pattern")
        def create_fan_pattern():
            builder = cloaca.WorkflowBuilder("fan_out_fan_in")
            builder.add_task(source_task)
            builder.add_task(process_task_1)
            builder.add_task(process_task_2)
            builder.add_task(process_task_3)
            builder.add_task(process_task_4)
            builder.add_task(aggregation_task)
            return builder.build()
        
        with timeout_protection(20):
            runner = clean_runner
            context = cloaca.Context({"fan_pattern_test": True})
            result = runner.execute("fan_out_fan_in", context)
            
            assert result is not None
            assert result.status == "Completed"


class TestTriggerRules:
    """Test trigger rule functionality."""
    
    def test_always_trigger_rule_default(self, clean_runner):
        """Test that default trigger rule is Always and works correctly."""
        import cloaca
        
        @cloaca.task()
        def always_trigger_task(context):
            # This task should execute because it defaults to Always trigger rule
            context.set("always_trigger_executed", True)
            return context
        
        @cloaca.workflow("always_trigger_test", "Always trigger rule test")
        def create_always_trigger_test():
            builder = cloaca.WorkflowBuilder("always_trigger_test")
            builder.add_task(always_trigger_task)
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"trigger_test": "always"})
            result = runner.execute("always_trigger_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_trigger_rule_with_dependencies(self, clean_runner):
        """Test trigger rules work correctly with task dependencies."""
        import cloaca
        
        @cloaca.task()
        def prerequisite_task(context):
            context.set("prerequisite_complete", True)
            return context
        
        @cloaca.task(dependencies=[prerequisite_task])
        def dependent_trigger_task(context):
            # Should only execute after prerequisite completes
            assert context.get("prerequisite_complete") is True
            context.set("dependent_executed", True)
            return context
        
        @cloaca.workflow("trigger_with_deps", "Trigger rules with dependencies")
        def create_trigger_deps_test():
            builder = cloaca.WorkflowBuilder("trigger_with_deps")
            builder.add_task(prerequisite_task)
            builder.add_task(dependent_trigger_task)
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"trigger_deps_test": True})
            result = runner.execute("trigger_with_deps", context)
            
            assert result is not None
            assert result.status == "Completed"


class TestWorkflowVersioning:
    """Test workflow versioning functionality."""
    
    def test_workflow_version_consistency(self, clean_runner):
        """Test that identical workflows produce identical versions."""
        import cloaca
        
        def create_test_workflow(name):
            @cloaca.task()
            def versioning_task(context):
                context.set("versioning_executed", True)
                return context
            
            @cloaca.workflow(name, "Version consistency test")
            def create_workflow():
                builder = cloaca.WorkflowBuilder(name)
                builder.description("Version consistency test")
                builder.tag("test", "versioning")
                builder.add_task(versioning_task)
                return builder.build()
            
            return create_workflow()
        
        # Create two identical workflows
        workflow1 = create_test_workflow("version_test_1")
        workflow2 = create_test_workflow("version_test_1")  # Same name, same structure
        
        # Should have identical versions
        assert workflow1.version == workflow2.version
    
    def test_workflow_version_changes_with_content(self, clean_runner):
        """Test that workflow versions change when content changes."""
        import cloaca
        
        @cloaca.task()
        def version_change_task(context):
            context.set("version_change_executed", True)
            return context
        
        # Create workflow with one description
        @cloaca.workflow("version_change_test_1", "First description")
        def create_workflow_1():
            builder = cloaca.WorkflowBuilder("version_change_test_1")
            builder.description("First description")
            builder.add_task(version_change_task)
            return builder.build()
        
        # Create workflow with different description
        @cloaca.workflow("version_change_test_2", "Second description")
        def create_workflow_2():
            builder = cloaca.WorkflowBuilder("version_change_test_2") 
            builder.description("Second description")  # Different description
            builder.add_task(version_change_task)
            return builder.build()
        
        workflow1 = create_workflow_1()
        workflow2 = create_workflow_2()
        
        # Should have different versions due to different descriptions
        assert workflow1.version != workflow2.version
    
    def test_workflow_version_with_different_tasks(self, clean_runner):
        """Test that workflows with different tasks have different versions."""
        import cloaca
        
        @cloaca.task()
        def task_a(context):
            context.set("task_a_executed", True)
            return context
        
        @cloaca.task()
        def task_b(context):
            context.set("task_b_executed", True)
            return context
        
        # Workflow with task A
        @cloaca.workflow("version_task_test_a", "Workflow with task A")
        def create_workflow_a():
            builder = cloaca.WorkflowBuilder("version_task_test_a")
            builder.description("Workflow with task A")
            builder.add_task(task_a)
            return builder.build()
        
        # Workflow with task B
        @cloaca.workflow("version_task_test_b", "Workflow with task B")
        def create_workflow_b():
            builder = cloaca.WorkflowBuilder("version_task_test_b")
            builder.description("Workflow with task B")
            builder.add_task(task_b)
            return builder.build()
        
        workflow_a = create_workflow_a()
        workflow_b = create_workflow_b()
        
        # Should have different versions due to different tasks
        assert workflow_a.version != workflow_b.version


class TestRegistryManagement:
    """Test task and workflow registry management."""
    
    def test_task_registry_isolation_between_tests(self, clean_runner):
        """Test that task registries are properly isolated between tests."""
        import cloaca
        
        # This test verifies that clean_runner fixture properly clears registries
        
        @cloaca.task(id="registry_isolation_task")
        def registry_task(context):
            context.set("registry_task_executed", True)
            return context
        
        @cloaca.workflow("registry_isolation_test", "Registry isolation test")
        def create_registry_test():
            builder = cloaca.WorkflowBuilder("registry_isolation_test")
            builder.add_task("registry_isolation_task")
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"registry_test": True})
            result = runner.execute("registry_isolation_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_workflow_registry_isolation_between_tests(self, clean_runner):
        """Test that workflow registries are properly isolated between tests."""
        import cloaca
        
        @cloaca.task(id="workflow_registry_task")
        def workflow_registry_task(context):
            context.set("workflow_registry_executed", True)
            return context
        
        @cloaca.workflow("workflow_registry_test", "Workflow registry test")
        def create_workflow_registry_test():
            builder = cloaca.WorkflowBuilder("workflow_registry_test")
            builder.add_task("workflow_registry_task")
            return builder.build()
        
        with timeout_protection(10):
            runner = clean_runner
            context = cloaca.Context({"workflow_registry_test": True})
            result = runner.execute("workflow_registry_test", context)
            
            assert result is not None
            assert result.status == "Completed"
    
    def test_multiple_workflow_registrations(self, clean_runner):
        """Test registering and executing multiple workflows in same test."""
        import cloaca
        
        # First workflow
        @cloaca.task(id="multi_reg_task_1")
        def task_1(context):
            context.set("task_1_executed", True)
            return context
        
        @cloaca.workflow("multi_reg_workflow_1", "Multi registration test 1")
        def create_workflow_1():
            builder = cloaca.WorkflowBuilder("multi_reg_workflow_1")
            builder.add_task("multi_reg_task_1")
            return builder.build()
        
        # Second workflow
        @cloaca.task(id="multi_reg_task_2")
        def task_2(context):
            context.set("task_2_executed", True)
            return context
        
        @cloaca.workflow("multi_reg_workflow_2", "Multi registration test 2")
        def create_workflow_2():
            builder = cloaca.WorkflowBuilder("multi_reg_workflow_2")
            builder.add_task("multi_reg_task_2")
            return builder.build()
        
        with timeout_protection(15):
            runner = clean_runner
            
            # Execute first workflow
            context_1 = cloaca.Context({"workflow": "first"})
            result_1 = runner.execute("multi_reg_workflow_1", context_1)
            assert result_1 is not None
            assert result_1.status == "Completed"
            
            # Execute second workflow
            context_2 = cloaca.Context({"workflow": "second"})
            result_2 = runner.execute("multi_reg_workflow_2", context_2)
            assert result_2 is not None
            assert result_2.status == "Completed"


class TestErrorHandlingAdvanced:
    """Test advanced error handling scenarios."""
    
    def test_invalid_function_reference_error_handling(self, clean_runner):
        """Test error handling when invalid function reference is used."""
        import cloaca
        
        with pytest.raises(Exception) as exc_info:
            @cloaca.workflow("invalid_ref_test", "Invalid reference test")
            def create_invalid_ref_test():
                builder = cloaca.WorkflowBuilder("invalid_ref_test")
                # Try to add something that's not a string or function
                builder.add_task(123)  # Invalid: not a string or function
                return builder.build()
        
        # Should get a meaningful error message
        assert "string task ID or a function object" in str(exc_info.value)
    
    def test_circular_dependency_detection(self, clean_runner):
        """Test that circular dependencies are detected and handled."""
        import cloaca
        
        # This should be caught during task registration or workflow building
        with pytest.raises(Exception):
            @cloaca.task(id="circular_task_a", dependencies=["circular_task_b"])
            def task_a(context):
                return context
            
            @cloaca.task(id="circular_task_b", dependencies=["circular_task_a"])
            def task_b(context):
                return context
            
            @cloaca.workflow("circular_test", "Circular dependency test")
            def create_circular_test():
                builder = cloaca.WorkflowBuilder("circular_test")
                builder.add_task("circular_task_a")
                builder.add_task("circular_task_b")
                return builder.build()
    
    def test_function_without_name_attribute_error(self, clean_runner):
        """Test error handling for objects without __name__ attribute."""
        import cloaca
        
        class FakeFunction:
            """Object that looks like a function but has no __name__."""
            pass
        
        with pytest.raises(Exception) as exc_info:
            @cloaca.workflow("missing_name_test", "Missing name test")
            def create_missing_name_test():
                builder = cloaca.WorkflowBuilder("missing_name_test")
                builder.add_task(FakeFunction())  # Should fail
                return builder.build()
        
        assert "string task ID or a function object" in str(exc_info.value)