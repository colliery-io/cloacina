"""
Basic tests for Cloaca Python bindings.

These tests verify that the basic functionality works with whichever backend
is installed in the current environment (sqlite or postgres).
Run with: pytest python-tests/test_basic.py

The backend is determined by which package was installed:
    pip install cloaca[sqlite]   # installs sqlite backend
    pip install cloaca[postgres] # installs postgres backend
"""

import os
import pytest
import importlib.util


@pytest.fixture
def installed_backend():
    """Fixture that detects and validates which backend package is installed."""
    backends = []
    
    # Check if cloaca_postgres is available
    if importlib.util.find_spec("cloaca_postgres") is not None:
        backends.append("postgres")
    
    # Check if cloaca_sqlite is available  
    if importlib.util.find_spec("cloaca_sqlite") is not None:
        backends.append("sqlite")
    
    # Should have exactly one backend installed
    assert len(backends) == 1, f"Expected exactly 1 backend, found {len(backends)}: {backends}"
    
    return backends[0]


def test_import_cloaca():
    """Test that we can import cloaca without errors."""
    import cloaca
    assert hasattr(cloaca, "hello_world")
    assert hasattr(cloaca, "get_backend")


def test_hello_world():
    """Test the hello_world function."""
    import cloaca
    result = cloaca.hello_world()
    assert isinstance(result, str)
    assert "Hello from Cloaca backend!" in result


def test_get_backend(installed_backend):
    """Test the get_backend function returns the installed backend."""
    import cloaca
    
    # Verify cloaca reports the correct backend
    backend = cloaca.get_backend()
    assert backend == installed_backend, f"Expected {installed_backend}, but got {backend}"
    
    # Should be consistent with module attribute
    assert backend == cloaca.__backend__


def test_backend_detection(installed_backend):
    """Test that backend detection works as expected."""
    import cloaca

    # Check that we can get the backend from both the function and module attribute
    backend_from_func = cloaca.get_backend()
    backend_from_attr = getattr(cloaca, "__backend__", None)

    # Both should match the installed backend
    assert backend_from_func == installed_backend
    assert backend_from_attr == installed_backend


def test_backend_specific_import(installed_backend):
    """Test importing cloaca works with the installed backend."""
    import cloaca
    
    # Verify cloaca reports the correct backend
    backend = cloaca.get_backend()
    assert backend == installed_backend, f"Expected {installed_backend}, but got {backend}"
    assert cloaca.hello_world() == "Hello from Cloaca backend!"


class TestBackendFunctionality:
    """Test backend functionality."""

    def test_current_backend_works(self, installed_backend):
        """Test that the currently installed backend works correctly."""
        import cloaca
        backend = cloaca.get_backend()
        
        # Should match the installed backend
        assert backend == installed_backend
        
        # Should have consistent backend reporting
        assert hasattr(cloaca, "__backend__")
        assert cloaca.__backend__ == installed_backend

    def test_backend_hello_world(self):
        """Test hello_world function works with current backend."""
        import cloaca
        result = cloaca.hello_world()
        assert result == "Hello from Cloaca backend!"

    def test_backend_attributes_exist(self):
        """Test that expected backend attributes exist."""
        import cloaca
        
        # All backends should have these core functions
        assert hasattr(cloaca, "hello_world")
        assert hasattr(cloaca, "get_backend")
        assert hasattr(cloaca, "__backend__")
        
        # Should be callable
        assert callable(cloaca.hello_world)
        assert callable(cloaca.get_backend)

    def test_hello_class_basic(self):
        """Test basic HelloClass functionality."""
        import cloaca
        
        # Test HelloClass creation
        hello_class = cloaca.HelloClass()
        assert hello_class is not None
        
        # Test method call
        message = hello_class.get_message()
        assert message == "Hello from HelloClass!"
        
        # Test string representation
        repr_str = repr(hello_class)
        assert "HelloClass" in repr_str

    def test_context_creation(self):
        """Test Context creation and basic functionality."""
        import cloaca
        
        # Test empty context creation
        ctx = cloaca.Context()
        assert ctx is not None
        assert len(ctx) == 0
        
        # Test context creation with dict
        ctx_with_data = cloaca.Context({"key1": "value1", "key2": 42})
        assert len(ctx_with_data) == 2
        
    def test_context_basic_operations(self):
        """Test Context basic get/set operations."""
        import cloaca
        
        ctx = cloaca.Context()
        
        # Test set and get
        ctx.set("test_key", "test_value")
        assert ctx.get("test_key") == "test_value"
        assert ctx.get("nonexistent") is None
        
        # Test dictionary-style access
        ctx["dict_key"] = "dict_value"
        assert ctx["dict_key"] == "dict_value"
        
        # Test contains
        assert "test_key" in ctx
        assert "nonexistent" not in ctx
        
    def test_context_insert_update(self):
        """Test Context insert and update operations."""
        import cloaca
        
        ctx = cloaca.Context()
        
        # Test insert
        ctx.insert("new_key", "new_value")
        assert ctx.get("new_key") == "new_value"
        
        # Test update
        ctx.update("new_key", "updated_value")
        assert ctx.get("new_key") == "updated_value"
        
        # Test insert on existing key should fail
        try:
            ctx.insert("new_key", "another_value")
            assert False, "Should have raised ValueError"
        except ValueError:
            pass  # Expected
            
        # Test update on nonexistent key should fail
        try:
            ctx.update("nonexistent", "value")
            assert False, "Should have raised KeyError"
        except KeyError:
            pass  # Expected
            
    def test_context_remove_delete(self):
        """Test Context remove and delete operations."""
        import cloaca
        
        ctx = cloaca.Context({"key1": "value1", "key2": "value2"})
        
        # Test remove
        removed = ctx.remove("key1")
        assert removed == "value1"
        assert "key1" not in ctx
        assert ctx.remove("nonexistent") is None
        
        # Test dictionary-style deletion
        del ctx["key2"]
        assert "key2" not in ctx
        assert len(ctx) == 0
        
        # Test delete nonexistent should fail
        try:
            del ctx["nonexistent"]
            assert False, "Should have raised KeyError"
        except KeyError:
            pass  # Expected
            
    def test_context_serialization(self):
        """Test Context JSON serialization."""
        import cloaca
        import json
        
        # Create context with various data types
        ctx = cloaca.Context({
            "string": "test",
            "number": 42,
            "float": 3.14,
            "boolean": True,
            "null": None,
            "list": [1, 2, 3],
            "dict": {"nested": "value"}
        })
        
        # Test to_json
        json_str = ctx.to_json()
        assert isinstance(json_str, str)
        
        # Verify it's valid JSON
        parsed = json.loads(json_str)
        assert parsed["string"] == "test"
        assert parsed["number"] == 42
        assert parsed["boolean"] is True
        
        # Test from_json
        ctx_from_json = cloaca.Context.from_json(json_str)
        assert len(ctx_from_json) == len(ctx)
        assert ctx_from_json.get("string") == "test"
        assert ctx_from_json.get("number") == 42
        
    def test_context_to_dict(self):
        """Test Context to_dict conversion."""
        import cloaca
        
        ctx = cloaca.Context({"key1": "value1", "key2": 42})
        
        # Test to_dict
        data_dict = ctx.to_dict()
        assert isinstance(data_dict, dict)
        assert data_dict["key1"] == "value1"
        assert data_dict["key2"] == 42
        
    def test_context_update_from_dict(self):
        """Test Context update_from_dict functionality."""
        import cloaca
        
        ctx = cloaca.Context({"existing": "value"})
        
        # Test update_from_dict
        ctx.update_from_dict({"new_key": "new_value", "existing": "updated"})
        assert ctx.get("new_key") == "new_value"
        assert ctx.get("existing") == "updated"
        
    def test_context_string_representation(self):
        """Test Context string representation."""
        import cloaca
        
        ctx = cloaca.Context({"test": "value"})
        repr_str = repr(ctx)
        assert "Context" in repr_str
        assert isinstance(repr_str, str)

    def test_default_runner_config_creation(self):
        """Test DefaultRunnerConfig creation with defaults and custom values."""
        import cloaca
        
        # Test default creation
        config = cloaca.DefaultRunnerConfig()
        assert config is not None
        assert isinstance(config.max_concurrent_tasks, int)
        assert config.max_concurrent_tasks > 0
        
        # Test static default method
        config_default = cloaca.DefaultRunnerConfig.default()
        assert config_default.max_concurrent_tasks == config.max_concurrent_tasks
        
        # Test custom configuration
        custom_config = cloaca.DefaultRunnerConfig(
            max_concurrent_tasks=8,
            task_timeout_seconds=600,
            enable_cron_scheduling=False
        )
        assert custom_config.max_concurrent_tasks == 8
        assert custom_config.task_timeout_seconds == 600
        assert custom_config.enable_cron_scheduling is False

    def test_default_runner_config_getters(self):
        """Test DefaultRunnerConfig getter properties."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        
        # Test all getters return reasonable values
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
        
        # Test some expected default values
        assert config.max_concurrent_tasks == 4
        assert config.executor_poll_interval_ms == 100
        assert config.task_timeout_seconds == 300  # 5 minutes
        assert config.enable_recovery is True
        assert config.enable_cron_scheduling is True

    def test_default_runner_config_setters(self):
        """Test DefaultRunnerConfig setter properties."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        
        # Test setters
        config.max_concurrent_tasks = 16
        assert config.max_concurrent_tasks == 16
        
        config.task_timeout_seconds = 1200
        assert config.task_timeout_seconds == 1200
        
        config.enable_cron_scheduling = False
        assert config.enable_cron_scheduling is False
        
        config.db_pool_size = 20
        assert config.db_pool_size == 20
        
        config.cron_poll_interval_seconds = 60
        assert config.cron_poll_interval_seconds == 60

    def test_default_runner_config_backend_specific_defaults(self):
        """Test that backend-specific defaults are correct."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        backend = cloaca.get_backend()
        
        # SQLite should have db_pool_size = 1, PostgreSQL should have db_pool_size = 10
        if backend == "sqlite":
            assert config.db_pool_size == 1
        elif backend == "postgres":
            assert config.db_pool_size == 10

    def test_default_runner_config_duration_conversions(self):
        """Test that duration fields convert correctly between units."""
        import cloaca
        
        # Test millisecond conversions
        config = cloaca.DefaultRunnerConfig(executor_poll_interval_ms=250)
        assert config.executor_poll_interval_ms == 250
        
        # Test second conversions
        config = cloaca.DefaultRunnerConfig(task_timeout_seconds=900)
        assert config.task_timeout_seconds == 900
        
        # Test pipeline timeout uses default when None is passed
        config = cloaca.DefaultRunnerConfig(pipeline_timeout_seconds=None)
        assert config.pipeline_timeout_seconds == 3600  # Default from Rust

    def test_default_runner_config_to_dict(self):
        """Test DefaultRunnerConfig to_dict conversion."""
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

    def test_default_runner_config_repr(self):
        """Test DefaultRunnerConfig string representation."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig()
        repr_str = repr(config)
        assert "DefaultRunnerConfig" in repr_str
        assert isinstance(repr_str, str)
        assert "max_concurrent_tasks" in repr_str

    def test_default_runner_config_all_parameters(self):
        """Test DefaultRunnerConfig with all parameters specified."""
        import cloaca
        
        config = cloaca.DefaultRunnerConfig(
            max_concurrent_tasks=12,
            executor_poll_interval_ms=50,
            scheduler_poll_interval_ms=75,
            task_timeout_seconds=1800,
            pipeline_timeout_seconds=7200,
            db_pool_size=15,
            enable_recovery=False,
            enable_cron_scheduling=True,
            cron_poll_interval_seconds=45,
            cron_max_catchup_executions=100,
            cron_enable_recovery=False,
            cron_recovery_interval_seconds=600,
            cron_lost_threshold_minutes=15,
            cron_max_recovery_age_seconds=172800,
            cron_max_recovery_attempts=5
        )
        
        # Verify all values were set correctly
        assert config.max_concurrent_tasks == 12
        assert config.executor_poll_interval_ms == 50
        assert config.scheduler_poll_interval_ms == 75
        assert config.task_timeout_seconds == 1800
        assert config.pipeline_timeout_seconds == 7200
        assert config.db_pool_size == 15
        assert config.enable_recovery is False
        assert config.enable_cron_scheduling is True
        assert config.cron_poll_interval_seconds == 45
        assert config.cron_max_catchup_executions == 100
        assert config.cron_enable_recovery is False
        assert config.cron_recovery_interval_seconds == 600
        assert config.cron_lost_threshold_minutes == 15
        assert config.cron_max_recovery_age_seconds == 172800
        assert config.cron_max_recovery_attempts == 5
