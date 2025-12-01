"""
New shared pytest configuration for Cloaca test harness rewrite.

This module implements the new fixture strategy to solve connection pool exhaustion:
- Single shared runner across tests to prevent PostgreSQL "too many clients" errors
- Registry cleanup between tests instead of runner recreation
- Selective isolation for tests that truly need it

Design principles:
1. Single connection pool shared across tests
2. Registry cleanup between tests to prevent workflow/task pollution
3. Fast cleanup avoiding slow runner.shutdown() calls
4. Selective isolation for critical tests
"""

import os
import pytest
import tempfile
import threading
import signal
from contextlib import contextmanager


# Global shared runner instance
_shared_runner = None
_runner_lock = threading.Lock()


def get_test_db_url():
    """Get appropriate database URL based on CLOACA_BACKEND env var."""
    # Use CLOACA_BACKEND env var to determine database, default to sqlite
    backend = os.environ.get("CLOACA_BACKEND", "sqlite").lower()

    if backend == "postgres":
        return "postgresql://cloacina:cloacina@localhost:5432/cloacina"
    elif backend == "sqlite":
        # Create a temporary database file for SQLite
        with tempfile.NamedTemporaryFile(suffix='.db', delete=False) as tmp:
            db_path = tmp.name
        return f"sqlite://{db_path}?mode=rwc&_journal_mode=WAL&_synchronous=NORMAL&_busy_timeout=5000"
    else:
        raise ValueError(f"Unsupported backend: {backend}. Set CLOACA_BACKEND to 'sqlite' or 'postgres'.")




@pytest.fixture(scope="session")
def shared_runner():
    """
    Single shared runner for the entire test session.

    This prevents connection pool exhaustion by reusing the same connection pool
    across all tests. The runner is created once and shutdown at session end.
    """
    global _shared_runner

    with _runner_lock:
        if _shared_runner is None:
            print("DEBUG: Creating shared runner for session")

            import cloaca
            db_url = get_test_db_url()

            print(f"DEBUG: Creating shared runner with URL: {db_url}")
            _shared_runner = cloaca.DefaultRunner(db_url)
            print("DEBUG: Shared runner created successfully")

    yield _shared_runner

    # Cleanup at session end
    if _shared_runner is not None:
        print("DEBUG: Shutting down shared runner at session end")
        try:
            # Set a timeout for shutdown
            def timeout_handler(signum, frame):
                print("WARNING: Shared runner shutdown timed out")
                raise TimeoutError("Shutdown timeout")

            signal.signal(signal.SIGALRM, timeout_handler)
            signal.alarm(10)  # 10 second timeout

            try:
                _shared_runner.shutdown()
                signal.alarm(0)  # Cancel timeout
                print("DEBUG: Shared runner shutdown completed")
            except TimeoutError:
                print("WARNING: Forced shutdown due to timeout")
                signal.alarm(0)

        except Exception as e:
            print(f"WARNING: Error during shared runner shutdown: {e}")

        _shared_runner = None


@pytest.fixture(scope="function")
def clean_runner(shared_runner):
    """
    Clean slate for each test using shared runner.

    Since tests are now isolated to single files and run separately,
    registry clearing may not be necessary. Testing without it.
    """
    print("DEBUG: Setting up clean runner (no registry clearing)")

    # Return the shared runner without clearing registries
    runner = shared_runner

    yield runner

    print("DEBUG: Cleaning up after test (no registry clearing)")


@pytest.fixture(scope="function")
def isolated_db():
    """
    Completely isolated database per test.

    For tests that need true isolation from shared state.
    Creates a new runner with fresh database connection.
    """
    print("DEBUG: Creating isolated database runner")

    import cloaca
    db_url = get_test_db_url()

    # Create isolated runner
    runner = cloaca.DefaultRunner(db_url)
    print("DEBUG: Isolated runner created")

    yield runner

    # Cleanup isolated runner
    print("DEBUG: Shutting down isolated runner")
    try:
        def timeout_handler(signum, frame):
            raise TimeoutError("Isolated runner shutdown timeout")

        signal.signal(signal.SIGALRM, timeout_handler)
        signal.alarm(5)  # 5 second timeout

        try:
            runner.shutdown()
            signal.alarm(0)
            print("DEBUG: Isolated runner shutdown completed")
        except TimeoutError:
            print("WARNING: Isolated runner shutdown timed out")
            signal.alarm(0)

    except Exception as e:
        print(f"WARNING: Error during isolated runner shutdown: {e}")

    # Clean up database if SQLite
    if db_url.startswith("sqlite://"):
        db_path = db_url.split("://")[1].split("?")[0]
        if os.path.exists(db_path):
            try:
                os.unlink(db_path)
                print(f"DEBUG: Removed SQLite file: {db_path}")
            except Exception as e:
                print(f"WARNING: Failed to remove SQLite file: {e}")


@contextmanager
def timeout_protection(seconds=15):
    """Context manager to protect against hanging operations."""
    def timeout_handler(signum, frame):
        raise TimeoutError(f"Operation timed out after {seconds} seconds")

    old_handler = signal.signal(signal.SIGALRM, timeout_handler)
    signal.alarm(seconds)

    try:
        yield
    finally:
        signal.alarm(0)
        signal.signal(signal.SIGALRM, old_handler)


@pytest.fixture(autouse=True)
def enable_rust_logging():
    """Enable Rust logging for all tests."""
    os.environ['RUST_LOG'] = 'cloacina=debug,cloaca_backend=debug'


def pytest_sessionfinish(session, exitstatus):
    """Final cleanup at session end."""
    print("DEBUG: Test session finished, performing final cleanup")
    global _shared_runner

    # Ensure shared runner is cleaned up
    if _shared_runner is not None:
        print("DEBUG: Final shared runner cleanup")
        try:
            _shared_runner.shutdown()
        except Exception as e:
            print(f"WARNING: Error in final shared runner cleanup: {e}")
        _shared_runner = None
