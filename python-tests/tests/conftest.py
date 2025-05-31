"""
Pytest configuration and shared fixtures for Cloacina Python bindings tests.
"""

import os
import pytest
import tempfile
import time
from pathlib import Path
from unittest.mock import patch


# Test environment setup
@pytest.fixture(scope="session", autouse=True)
def setup_test_environment():
    """Configure test environment."""
    # Ensure we're in test mode
    os.environ["CLOACINA_TEST_MODE"] = "1"
    
    # Set up test database URLs
    test_dir = Path(tempfile.gettempdir()) / "cloacina_tests"
    test_dir.mkdir(exist_ok=True)
    
    # SQLite test database
    sqlite_db = test_dir / "test_cloacina.db"
    os.environ["CLOACINA_SQLITE_URL"] = f"sqlite:///{sqlite_db}"
    
    # PostgreSQL test database (if available)
    postgres_url = os.environ.get("TEST_DATABASE_URL", 
                                  "postgresql://localhost:5432/cloacina_test")
    os.environ["CLOACINA_POSTGRES_URL"] = postgres_url
    
    yield
    
    # Cleanup
    if sqlite_db.exists():
        sqlite_db.unlink()


@pytest.fixture
def temp_sqlite_db():
    """Create a temporary SQLite database path for testing."""
    # Create a unique temporary file path without creating the file
    temp_dir = Path(tempfile.gettempdir()) / "cloacina_tests"
    temp_dir.mkdir(exist_ok=True)
    
    # Generate unique database path
    db_path = temp_dir / f"test_{os.getpid()}_{int(time.time())}.db"
    
    yield f"sqlite:///{db_path}"
    
    # Cleanup
    db_path.unlink(missing_ok=True)


@pytest.fixture
def mock_backend_import():
    """Mock backend imports for testing the dispatcher."""
    with patch.dict('sys.modules', {
        'cloacina_postgres': None,
        'cloacina_sqlite': None,
    }):
        yield


@pytest.fixture 
def sample_context():
    """Sample context data for testing."""
    return {
        "users": [
            {"id": 1, "name": "Alice", "email": "alice@example.com"},
            {"id": 2, "name": "Bob", "email": "bob@example.com"},
        ],
        "config": {"batch_size": 10, "timeout": 30},
        "metadata": {"timestamp": "2024-01-01T00:00:00Z"}
    }


@pytest.fixture
def sample_tasks():
    """Sample task functions for testing."""
    def extract_task(context):
        context = context or {}
        context["extracted_data"] = [1, 2, 3, 4, 5]
        return context
    
    def transform_task(context):
        data = context.get("extracted_data", [])
        context["transformed_data"] = [x * 2 for x in data]
        return context
    
    def load_task(context):
        data = context.get("transformed_data", [])
        context["loaded_count"] = len(data)
        context["status"] = "completed"
        return context
    
    return {
        "extract": extract_task,
        "transform": transform_task, 
        "load": load_task
    }


def pytest_configure(config):
    """Configure pytest with custom markers."""
    config.addinivalue_line(
        "markers", "backend(name): mark test to run only with specific backend"
    )


def pytest_collection_modifyitems(config, items):
    """Modify test collection based on available backends."""
    # Check which backends are available
    postgres_available = False
    sqlite_available = False
    
    try:
        import cloacina_postgres
        postgres_available = True
    except ImportError:
        pass
    
    try:
        import cloacina_sqlite  
        sqlite_available = True
    except ImportError:
        pass
    
    # Skip tests based on backend availability
    skip_postgres = pytest.mark.skip(reason="cloacina-postgres not available")
    skip_sqlite = pytest.mark.skip(reason="cloacina-sqlite not available")
    
    for item in items:
        if "postgres" in item.keywords and not postgres_available:
            item.add_marker(skip_postgres)
        if "sqlite" in item.keywords and not sqlite_available:
            item.add_marker(skip_sqlite)