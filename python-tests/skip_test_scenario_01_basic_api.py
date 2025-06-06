"""
Scenario 1: Basic API Functionality Tests

This test file verifies that core API functionality works without requiring database operations.
Tests include imports, context operations, task decorators, workflow builders, and configuration.

No database connection needed - focuses on object creation and manipulation.
"""

import pytest


class TestBasicImports:
    """Test that we can import and use basic Cloaca functionality."""
    
    def test_import_cloaca_successfully(self):
        """Test that cloaca module imports without errors."""
        import cloaca
        
        # Verify core functions are available
        assert hasattr(cloaca, 'hello_world')
        assert hasattr(cloaca, 'get_backend')
        assert callable(cloaca.hello_world)
        assert callable(cloaca.get_backend)
    
    def test_hello_world_function(self):
        """Test the hello_world function returns expected output."""
        import cloaca
        
        result = cloaca.hello_world()
        assert isinstance(result, str)
        assert result == "Hello from Cloaca backend!"
    
    def test_backend_detection(self):
        """Test backend detection returns valid backend type."""
        import cloaca
        
        backend = cloaca.get_backend()
        assert backend in ["sqlite", "postgres"]
        assert backend == cloaca.__backend__
    
    def test_core_classes_available(self):
        """Test that core classes are importable."""
        import cloaca
        
        # Test class availability
        assert hasattr(cloaca, 'Context')
        assert hasattr(cloaca, 'DefaultRunnerConfig') 
        assert hasattr(cloaca, 'WorkflowBuilder')
        assert hasattr(cloaca, 'HelloClass')
        
        # Test decorator availability
        assert hasattr(cloaca, 'task')
        assert hasattr(cloaca, 'workflow')
        assert hasattr(cloaca, 'register_workflow_constructor')


class TestContextOperations:
    """Test Context class functionality without database operations."""
    
    def test_empty_context_creation(self):
        """Test creating empty context."""
        import cloaca
        
        ctx = cloaca.Context()
        assert ctx is not None
        assert len(ctx) == 0
        assert isinstance(ctx, cloaca.Context)
    
    def test_context_creation_with_data(self):
        """Test creating context with initial data."""
        import cloaca
        
        initial_data = {
            "string_val": "test",
            "int_val": 42,
            "float_val": 3.14,
            "bool_val": True,
            "none_val": None,
            "list_val": [1, 2, 3],
            "dict_val": {"nested": "value"}
        }
        
        ctx = cloaca.Context(initial_data)
        assert len(ctx) == len(initial_data)
        
        # Verify all values are accessible
        assert ctx.get("string_val") == "test"
        assert ctx.get("int_val") == 42
        assert ctx.get("float_val") == 3.14
        assert ctx.get("bool_val") is True
        assert ctx.get("none_val") is None
        assert ctx.get("list_val") == [1, 2, 3]
        assert ctx.get("dict_val") == {"nested": "value"}
    
    def test_context_basic_operations(self):
        """Test basic get/set/contains operations."""
        import cloaca
        
        ctx = cloaca.Context()
        
        # Test set and get
        ctx.set("test_key", "test_value")
        assert ctx.get("test_key") == "test_value"
        assert ctx.get("nonexistent") is None
        
        # Test contains operator
        assert "test_key" in ctx
        assert "nonexistent" not in ctx
        
        # Test dictionary-style access
        ctx["dict_key"] = "dict_value"
        assert ctx["dict_key"] == "dict_value"
        
        # Test length
        assert len(ctx) == 2
    
    def test_context_insert_and_update(self):
        """Test insert and update operations with error handling."""
        import cloaca
        
        ctx = cloaca.Context()
        
        # Test insert on new key
        ctx.insert("new_key", "new_value")
        assert ctx.get("new_key") == "new_value"
        
        # Test insert on existing key should fail
        with pytest.raises(ValueError):
            ctx.insert("new_key", "another_value")
        
        # Test update on existing key
        ctx.update("new_key", "updated_value")
        assert ctx.get("new_key") == "updated_value"
        
        # Test update on nonexistent key should fail
        with pytest.raises(KeyError):
            ctx.update("nonexistent", "value")
    
    def test_context_remove_and_delete(self):
        """Test remove and delete operations."""
        import cloaca
        
        ctx = cloaca.Context({
            "key1": "value1",
            "key2": "value2",
            "key3": "value3"
        })
        
        # Test remove operation
        removed = ctx.remove("key1")
        assert removed == "value1"
        assert "key1" not in ctx
        assert len(ctx) == 2
        
        # Test remove nonexistent returns None
        assert ctx.remove("nonexistent") is None
        
        # Test dictionary-style deletion
        del ctx["key2"]
        assert "key2" not in ctx
        assert len(ctx) == 1
        
        # Test delete nonexistent should fail
        with pytest.raises(KeyError):
            del ctx["nonexistent"]
    
    def test_context_serialization(self):
        """Test JSON serialization and deserialization."""
        import cloaca
        import json
        
        original_data = {
            "string": "test_string",
            "number": 42,
            "float": 3.14159,
            "boolean": True,
            "null": None,
            "list": [1, "two", 3.0],
            "object": {"nested": "value", "count": 5}
        }
        
        ctx = cloaca.Context(original_data)
        
        # Test to_json
        json_str = ctx.to_json()
        assert isinstance(json_str, str)
        
        # Verify it's valid JSON
        parsed = json.loads(json_str)
        assert parsed["string"] == "test_string"
        assert parsed["number"] == 42
        assert parsed["boolean"] is True
        assert parsed["null"] is None
        
        # Test from_json
        ctx_from_json = cloaca.Context.from_json(json_str)
        assert len(ctx_from_json) == len(ctx)
        assert ctx_from_json.get("string") == "test_string"
        assert ctx_from_json.get("number") == 42
        assert ctx_from_json.get("list") == [1, "two", 3.0]
        assert ctx_from_json.get("object") == {"nested": "value", "count": 5}
    
    def test_context_dict_conversion(self):
        """Test to_dict and update_from_dict operations."""
        import cloaca
        
        original_data = {"key1": "value1", "key2": 42}
        ctx = cloaca.Context(original_data)
        
        # Test to_dict
        data_dict = ctx.to_dict()
        assert isinstance(data_dict, dict)
        assert data_dict == original_data
        
        # Test update_from_dict
        update_data = {"key2": 100, "key3": "new_value"}
        ctx.update_from_dict(update_data)
        
        assert ctx.get("key1") == "value1"  # Unchanged
        assert ctx.get("key2") == 100       # Updated
        assert ctx.get("key3") == "new_value"  # Added
        assert len(ctx) == 3
    
    def test_context_string_representation(self):
        """Test context string representation."""
        import cloaca
        
        ctx = cloaca.Context({"test": "value", "count": 5})
        repr_str = repr(ctx)
        
        assert isinstance(repr_str, str)
        assert "Context" in repr_str


class TestTaskDecorator:
    """Test @task decorator functionality without execution."""
    
    def test_basic_task_decorator(self):
        """Test basic task decorator usage."""
        import cloaca
        
        @cloaca.task(id="basic_test_task")
        def basic_task(context):
            context.set("executed", True)
            return context
        
        # Function should remain callable
        assert callable(basic_task)
        
        # Test direct function call
        ctx = cloaca.Context()
        result = basic_task(ctx)
        assert result.get("executed") is True
    
    def test_task_decorator_with_dependencies(self):
        """Test task decorator with dependency specification."""
        import cloaca
        
        @cloaca.task(id="task_with_deps", dependencies=["dep1", "dep2"])
        def task_with_deps(context):
            context.set("deps_task_executed", True)
            return context
        
        assert callable(task_with_deps)
        
        # Test function still works
        ctx = cloaca.Context()
        result = task_with_deps(ctx)
        assert result.get("deps_task_executed") is True
    
    def test_task_decorator_with_retry_policy(self):
        """Test task decorator with comprehensive retry configuration."""
        import cloaca
        
        @cloaca.task(
            id="retry_task",
            retry_attempts=5,
            retry_backoff="exponential",
            retry_delay_ms=2000,
            retry_max_delay_ms=60000,
            retry_condition="transient",
            retry_jitter=True
        )
        def retry_task(context):
            context.set("retry_task_executed", True)
            return context
        
        assert callable(retry_task)
        
        # Test function execution
        ctx = cloaca.Context()
        result = retry_task(ctx)
        assert result.get("retry_task_executed") is True
    
    def test_task_decorator_auto_id(self):
        """Test task decorator with automatic ID generation."""
        import cloaca
        
        @cloaca.task()
        def auto_id_task(context):
            context.set("auto_id_executed", True)
            return context
        
        assert callable(auto_id_task)
        
        # Function name should be used as ID in registry
        ctx = cloaca.Context()
        result = auto_id_task(ctx)
        assert result.get("auto_id_executed") is True
    
    def test_task_decorator_function_references(self):
        """Test using function references in dependencies."""
        import cloaca
        
        @cloaca.task()
        def prerequisite_task(context):
            context.set("prerequisite_done", True)
            return context
        
        @cloaca.task(dependencies=[prerequisite_task])
        def dependent_task(context):
            context.set("dependent_done", True)
            return context
        
        # Both should be callable
        assert callable(prerequisite_task)
        assert callable(dependent_task)
        
        # Test individual execution
        ctx = cloaca.Context()
        
        result1 = prerequisite_task(ctx)
        assert result1.get("prerequisite_done") is True
        
        result2 = dependent_task(ctx)
        assert result2.get("dependent_done") is True
    
    def test_task_decorator_return_none(self):
        """Test task that returns None (success case)."""
        import cloaca
        
        @cloaca.task(id="none_return_task")
        def none_return_task(context):
            context.set("none_task_executed", True)
            # Return None indicates success
            return None
        
        ctx = cloaca.Context()
        result = none_return_task(ctx)
        
        assert result is None
        assert ctx.get("none_task_executed") is True


class TestWorkflowBuilder:
    """Test WorkflowBuilder functionality without execution."""
    
    def test_basic_workflow_builder_creation(self):
        """Test creating WorkflowBuilder with basic configuration."""
        import cloaca
        
        builder = cloaca.WorkflowBuilder("test_workflow")
        assert builder is not None
        
        # Test method chaining
        builder.description("Test workflow description")
        builder.tag("environment", "test")
        builder.tag("team", "backend")
        
        # Should be able to build empty workflow
        workflow = builder.build()
        assert workflow is not None
        assert workflow.name == "test_workflow"
        assert workflow.description == "Test workflow description"
        assert isinstance(workflow.version, str)
        assert len(workflow.version) > 0
    
    def test_workflow_builder_with_tasks(self):
        """Test building workflow with registered tasks."""
        import cloaca
        
        # Register some tasks first
        @cloaca.task(id="workflow_task_1")
        def task1(context):
            context.set("task1_executed", True)
            return context
        
        @cloaca.task(id="workflow_task_2", dependencies=["workflow_task_1"])
        def task2(context):
            context.set("task2_executed", True)  
            return context
        
        # Build workflow
        builder = cloaca.WorkflowBuilder("task_workflow")
        builder.description("Workflow with tasks")
        builder.add_task("workflow_task_1")
        builder.add_task("workflow_task_2")
        
        workflow = builder.build()
        assert workflow.name == "task_workflow"
        assert workflow.description == "Workflow with tasks"
        
        # Test workflow structure
        topo = workflow.topological_sort()
        assert len(topo) == 2
        assert topo == ["workflow_task_1", "workflow_task_2"]
        
        # Test execution levels
        levels = workflow.get_execution_levels()
        assert len(levels) == 2
        assert levels[0] == ["workflow_task_1"]
        assert levels[1] == ["workflow_task_2"]
        
        # Test roots and leaves
        roots = workflow.get_roots()
        leaves = workflow.get_leaves()
        assert roots == ["workflow_task_1"]
        assert leaves == ["workflow_task_2"]
    
    def test_workflow_builder_function_references(self):
        """Test adding tasks using function references."""
        import cloaca
        
        @cloaca.task()
        def step_one(context):
            return context
        
        @cloaca.task()
        def step_two(context):
            return context
        
        builder = cloaca.WorkflowBuilder("function_ref_workflow")
        builder.add_task(step_one)    # Function reference
        builder.add_task(step_two)    # Function reference
        
        workflow = builder.build()
        assert workflow.name == "function_ref_workflow"
        
        topo = workflow.topological_sort()
        assert len(topo) == 2
        assert "step_one" in topo
        assert "step_two" in topo
    
    def test_workflow_builder_error_handling(self):
        """Test error handling in WorkflowBuilder."""
        import cloaca
        
        builder = cloaca.WorkflowBuilder("error_test_workflow")
        
        # Test adding non-existent task
        with pytest.raises(ValueError) as exc_info:
            builder.add_task("nonexistent_task")
        assert "not found in registry" in str(exc_info.value)
        
        # Test adding invalid task reference
        with pytest.raises(Exception) as exc_info:
            builder.add_task(123)  # Not a string or function
        assert "string task ID or a function object" in str(exc_info.value)
    
    def test_workflow_validation(self):
        """Test workflow validation functionality."""
        import cloaca
        
        # Empty workflow should fail validation
        builder = cloaca.WorkflowBuilder("empty_workflow")
        workflow = builder.build()
        
        with pytest.raises(ValueError) as exc_info:
            workflow.validate()
        assert "cannot be empty" in str(exc_info.value)
        
        # Workflow with tasks should validate successfully
        @cloaca.task(id="validation_task")
        def validation_task(context):
            return context
        
        builder = cloaca.WorkflowBuilder("valid_workflow")
        builder.add_task("validation_task")
        workflow = builder.build()
        
        # Should not raise exception
        workflow.validate()
    
    def test_workflow_properties(self):
        """Test workflow property access and methods."""
        import cloaca
        
        @cloaca.task(id="prop_task_1")
        def task1(context):
            return context
        
        @cloaca.task(id="prop_task_2")  
        def task2(context):
            return context
        
        builder = cloaca.WorkflowBuilder("property_workflow")
        builder.description("Test properties")
        builder.tag("type", "test")
        builder.add_task("prop_task_1")
        builder.add_task("prop_task_2")
        
        workflow = builder.build()
        
        # Test basic properties
        assert workflow.name == "property_workflow"
        assert workflow.description == "Test properties"
        assert isinstance(workflow.version, str)
        
        # Test parallel execution check
        result = workflow.can_run_parallel("prop_task_1", "prop_task_2")
        assert isinstance(result, bool)
        assert result is True  # No dependencies, so they can run in parallel
        
        # Test string representation
        repr_str = repr(workflow)
        assert isinstance(repr_str, str)
        assert "Workflow" in repr_str
        assert "property_workflow" in repr_str
    
    def test_workflow_version_consistency(self):
        """Test that identical workflows have identical versions."""
        import cloaca
        
        # Create identical workflows
        def build_identical_workflow(name):
            builder = cloaca.WorkflowBuilder(name)
            builder.description("Identical workflow")
            builder.tag("env", "test")
            return builder.build()
        
        workflow1 = build_identical_workflow("version_test")
        workflow2 = build_identical_workflow("version_test")
        
        # Should have identical versions (content-based hashing)
        assert workflow1.version == workflow2.version
        
        # Different description should result in different version
        builder3 = cloaca.WorkflowBuilder("version_test")
        builder3.description("Different description")
        builder3.tag("env", "test")
        workflow3 = builder3.build()
        
        assert workflow1.version != workflow3.version


class TestDefaultRunnerConfig:
    """Test DefaultRunnerConfig functionality."""
    
    def test_config_creation_with_defaults(self):
        """Test creating config with default values."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        assert config is not None
        
        # Test default values
        assert config.max_concurrent_tasks == 4
        assert config.executor_poll_interval_ms == 100
        assert config.task_timeout_seconds == 300
        assert config.enable_recovery is True
        assert config.enable_cron_scheduling is True
        
        # Backend-specific defaults
        backend = cloaca.get_backend()
        if backend == "sqlite":
            assert config.db_pool_size == 1
        elif backend == "postgres":
            assert config.db_pool_size == 10
    
    def test_config_creation_with_custom_values(self):
        """Test creating config with custom values."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig(
            max_concurrent_tasks=8,
            task_timeout_seconds=600,
            enable_cron_scheduling=False,
            db_pool_size=20,
            cron_poll_interval_seconds=120
        )
        
        # Verify custom values
        assert config.max_concurrent_tasks == 8
        assert config.task_timeout_seconds == 600
        assert config.enable_cron_scheduling is False
        assert config.db_pool_size == 20
        assert config.cron_poll_interval_seconds == 120
    
    def test_config_property_access(self):
        """Test all config property getters and setters."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        
        # Test all getter properties return expected types
        assert isinstance(config.max_concurrent_tasks, int)
        assert isinstance(config.executor_poll_interval_ms, int)
        assert isinstance(config.scheduler_poll_interval_ms, int)
        assert isinstance(config.task_timeout_seconds, int)
        assert isinstance(config.pipeline_timeout_seconds, (int, type(None)))
        assert isinstance(config.db_pool_size, int)
        assert isinstance(config.enable_recovery, bool)
        assert isinstance(config.enable_cron_scheduling, bool)
        assert isinstance(config.cron_poll_interval_seconds, int)
        assert isinstance(config.cron_max_catchup_executions, int)
        assert isinstance(config.cron_enable_recovery, bool)
        assert isinstance(config.cron_recovery_interval_seconds, int)
        assert isinstance(config.cron_lost_threshold_minutes, int)
        assert isinstance(config.cron_max_recovery_age_seconds, int)
        assert isinstance(config.cron_max_recovery_attempts, int)
        
        # Test setters
        config.max_concurrent_tasks = 16
        assert config.max_concurrent_tasks == 16
        
        config.enable_cron_scheduling = False
        assert config.enable_cron_scheduling is False
    
    def test_config_to_dict(self):
        """Test config dictionary conversion."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig(
            max_concurrent_tasks=6,
            enable_cron_scheduling=False
        )
        
        config_dict = config.to_dict()
        assert isinstance(config_dict, dict)
        assert config_dict["max_concurrent_tasks"] == 6
        assert config_dict["enable_cron_scheduling"] is False
        assert "task_timeout_seconds" in config_dict
        assert "db_pool_size" in config_dict
    
    def test_config_static_default_method(self):
        """Test static default method."""
        import cloaca
        
        config1 = cloaca.DefaultRunnerConfig()
        config2 = cloaca.DefaultRunnerConfig.default()
        
        # Should have same default values
        assert config1.max_concurrent_tasks == config2.max_concurrent_tasks
        assert config1.task_timeout_seconds == config2.task_timeout_seconds
        assert config1.enable_recovery == config2.enable_recovery
    
    def test_config_string_representation(self):
        """Test config string representation."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        repr_str = repr(config)
        
        assert isinstance(repr_str, str)
        assert "DefaultRunnerConfig" in repr_str
        assert "max_concurrent_tasks" in repr_str


class TestWorkflowDecorator:
    """Test @workflow decorator functionality."""
    
    def test_basic_workflow_decorator(self):
        """Test basic workflow decorator usage."""
        import cloaca
        
        # Define task for the workflow
        @cloaca.task(id="decorator_test_task")
        def decorator_task(context):
            context.set("decorator_executed", True)
            return context
        
        # Test workflow decorator
        @cloaca.workflow("decorator_workflow", "Workflow using decorator")
        def create_decorator_workflow():
            builder = cloaca.WorkflowBuilder("decorator_workflow")
            builder.description("Workflow using decorator")
            builder.add_task("decorator_test_task")
            return builder.build()
        
        # Function should still be callable
        assert callable(create_decorator_workflow)
        
        # Test direct function call
        workflow = create_decorator_workflow()
        assert workflow.name == "decorator_workflow"
        assert workflow.description == "Workflow using decorator"
        
        # Verify task is included
        topo = workflow.topological_sort()
        assert "decorator_test_task" in topo
    
    def test_register_workflow_constructor(self):
        """Test manual workflow constructor registration."""
        import cloaca
        
        @cloaca.task(id="manual_reg_task")
        def manual_task(context):
            context.set("manual_executed", True)
            return context
        
        def create_manual_workflow():
            builder = cloaca.WorkflowBuilder("manual_workflow")
            builder.description("Manually registered workflow")
            builder.add_task("manual_reg_task")
            return builder.build()
        
        # Test manual registration
        cloaca.register_workflow_constructor("manual_workflow", create_manual_workflow)
        
        # Function should still work
        workflow = create_manual_workflow()
        assert workflow.name == "manual_workflow"
        assert workflow.description == "Manually registered workflow"


class TestHelloClass:
    """Test HelloClass functionality."""
    
    def test_hello_class_creation(self):
        """Test HelloClass creation and basic functionality."""
        import cloaca
        
        hello = cloaca.HelloClass()
        assert hello is not None
        
        # Test message method
        message = hello.get_message()
        assert message == "Hello from HelloClass!"
        
        # Test string representation
        repr_str = repr(hello)
        assert isinstance(repr_str, str)
        assert "HelloClass" in repr_str