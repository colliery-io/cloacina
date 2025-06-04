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
