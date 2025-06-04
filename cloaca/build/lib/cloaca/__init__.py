"""
Cloaca - Python bindings for Cloacina workflow orchestration engine.

This is the dispatcher package that automatically selects and loads
the appropriate backend (PostgreSQL or SQLite) based on availability
and environment configuration.
"""

import importlib
import os
from typing import Any, Optional

__version__ = "0.1.0"
__backend__: Optional[str] = None


def _load_backend() -> tuple[Any, str]:
    """Load the appropriate backend based on what's installed."""
    available_backends = []

    try:
        module = importlib.import_module("cloaca_postgres")
        available_backends.append(("postgres", module))
    except ImportError:
        pass

    try:
        module = importlib.import_module("cloaca_sqlite")
        available_backends.append(("sqlite", module))
    except ImportError:
        pass

    if len(available_backends) == 0:
        raise ImportError(
            "No Cloaca backend available. Install one:\n"
            "  pip install cloaca[postgres]  # for PostgreSQL support\n"
            "  pip install cloaca[sqlite]    # for SQLite support"
        )
    elif len(available_backends) == 1:
        backend_name, module = available_backends[0]
        return module, backend_name
    else:
        # Multiple backends available - this shouldn't happen in practice
        # with proper virtual environment isolation, but handle gracefully
        backend_names = [name for name, _ in available_backends]
        raise ImportError(
            f"Multiple backends installed: {', '.join(backend_names)}. "
            f"This indicates a configuration issue - only one backend should be "
            f"installed per environment. Use separate virtual environments."
        )


# Load backend and expose its API
try:
    _backend_module, __backend__ = _load_backend()

    # Re-export all backend symbols
    __all__ = getattr(_backend_module, "__all__", [])
    for attr in __all__:
        globals()[attr] = getattr(_backend_module, attr)

    # Also expose commonly used symbols directly
    if hasattr(_backend_module, "hello_world"):
        hello_world = _backend_module.hello_world
    if hasattr(_backend_module, "get_backend"):
        get_backend = _backend_module.get_backend

except ImportError:
    # If no backend is available, provide helpful error message
    def _raise_no_backend(*args, **kwargs):
        raise ImportError(str(e))

    # Create placeholder symbols that raise helpful errors
    hello_world = _raise_no_backend
    get_backend = _raise_no_backend

    __all__ = ["hello_world", "get_backend"]


def get_backend() -> Optional[str]:
    """Get the currently loaded backend name."""
    return __backend__


def get_version() -> str:
    """Get the version of the cloaca package."""
    return __version__
