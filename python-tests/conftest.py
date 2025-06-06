"""
Shared pytest configuration and fixtures for all Cloaca tests.
This module provides connection pooling to avoid PostgreSQL 'too many clients' errors.
"""

import os
import pytest
import tempfile
import threading
import sys

# Global connection pool for tests to avoid "too many clients" error
_test_connection_pool = {}
_pool_lock = threading.Lock()


def get_test_db_url():
    """Get appropriate database URL based on compiled backend."""
    # We need to import cloaca here since we can't assume it's available at module level
    # Import after any potential module reloading in the fixture
    backend = None
    try:
        import cloaca
        backend = cloaca.get_backend()
    except ImportError:
        # Fallback - try to determine from environment or assume sqlite
        backend = "sqlite"
    
    if backend == "postgres":
        return "postgresql://cloacina:cloacina@localhost:5432/cloacina"
    elif backend == "sqlite":
        # Create a temporary database file for SQLite
        with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
            db_path = tmp.name
        return f"sqlite://{db_path}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000"
    else:
        raise ValueError(f"Unsupported backend: {backend}")


def get_pooled_connection(db_url):
    """Get a pooled connection to avoid 'too many clients' error."""
    with _pool_lock:
        if db_url not in _test_connection_pool:
            import psycopg2
            from urllib.parse import urlparse
            parsed = urlparse(db_url)
            
            conn = psycopg2.connect(
                host=parsed.hostname,
                port=parsed.port,
                user=parsed.username,
                password=parsed.password,
                database=parsed.path[1:],
                connect_timeout=10
            )
            conn.autocommit = True
            _test_connection_pool[db_url] = conn
            print(f"DEBUG: Created new pooled connection for {db_url}")
        else:
            print(f"DEBUG: Reusing pooled connection for {db_url}")
        return _test_connection_pool[db_url]


def cleanup_test_db(db_url):
    """Clean up test database after test completion."""
    print(f"DEBUG: Cleaning up database: {db_url}")
    
    if db_url.startswith("sqlite://"):
        # For SQLite, remove the file
        db_path = db_url.split("://")[1].split("?")[0]
        if os.path.exists(db_path):
            print(f"DEBUG: Removing SQLite file: {db_path}")
            os.unlink(db_path)
        else:
            print(f"DEBUG: SQLite file not found: {db_path}")
    elif db_url.startswith("postgresql://"):
        # For PostgreSQL, drop all tables using pooled connection
        print("DEBUG: Attempting PostgreSQL cleanup")
        try:
            conn = get_pooled_connection(db_url)
            cursor = conn.cursor()
            
            # Get all table names
            cursor.execute("""
                SELECT tablename FROM pg_tables 
                WHERE schemaname = 'public'
            """)
            tables = cursor.fetchall()
            print(f"DEBUG: Found {len(tables)} tables to drop: {[t[0] for t in tables]}")
            
            # Drop all tables
            for table in tables:
                print(f"DEBUG: Dropping table: {table[0]}")
                cursor.execute(f"DROP TABLE IF EXISTS {table[0]} CASCADE")
            
            cursor.close()
            print("DEBUG: PostgreSQL cleanup completed successfully")
                    
        except ImportError:
            # psycopg2 not available, skip cleanup
            print("Warning: psycopg2 not available, skipping PostgreSQL cleanup")
        except Exception as e:
            # Log but don't fail the test
            print(f"Warning: Failed to clean up PostgreSQL tables: {e}")
    else:
        print(f"DEBUG: Unknown database type for cleanup: {db_url}")


@pytest.fixture
def isolated_runner():
    """Fixture that provides a completely isolated runner with fresh database and fresh module state."""
    # Force reload cloaca module to get fresh global registry state
    # This prevents mutex poisoning and stale workflow data between tests
    import importlib
    if 'cloaca' in sys.modules:
        importlib.reload(sys.modules['cloaca'])
    
    # Import cloaca after potential reload
    import cloaca
    
    # Get appropriate database URL for the backend
    db_url = get_test_db_url()
    print(f"DEBUG: Generated database URL: {db_url}")
    
    # Test database connectivity first
    if db_url.startswith("postgresql://"):
        try:
            conn = get_pooled_connection(db_url)
            print("DEBUG: PostgreSQL connection test successful")
        except Exception as e:
            print(f"DEBUG: PostgreSQL connection test failed: {e}")
            raise
    
    try:
        # Create runner instance - each test gets its own
        print(f"DEBUG: About to create DefaultRunner with URL: {db_url}")
        runner = cloaca.DefaultRunner(db_url)
        print(f"DEBUG: DefaultRunner created successfully")
        
        yield runner
        
        # Explicit cleanup - shutdown runner first with timeout protection
        try:
            print("DEBUG: Shutting down runner services")
            import signal
            
            def timeout_handler(signum, frame):
                print("WARNING: Runner shutdown timed out after 5 seconds")
                raise TimeoutError("Runner shutdown timeout")
            
            # Set a 5 second timeout for shutdown
            signal.signal(signal.SIGALRM, timeout_handler)
            signal.alarm(5)
            
            try:
                runner.shutdown()
                signal.alarm(0)  # Cancel timeout
                print("DEBUG: Runner shutdown completed")
            except TimeoutError:
                print("WARNING: Forced shutdown due to timeout")
                signal.alarm(0)  # Cancel timeout
                
        except Exception as e:
            print(f"Warning: Error during runner shutdown: {e}")
        
        # Clean up database
        cleanup_test_db(db_url)
        
    except Exception as e:
        print(f"ERROR: Failed to create runner: {e}")
        cleanup_test_db(db_url)
        raise


def pytest_sessionfinish(session, exitstatus):
    """Close all pooled connections at the end of the test session."""
    with _pool_lock:
        for db_url, conn in _test_connection_pool.items():
            try:
                conn.close()
                print(f"DEBUG: Closed pooled connection for {db_url}")
            except Exception as e:
                print(f"Warning: Error closing connection for {db_url}: {e}")
        _test_connection_pool.clear()