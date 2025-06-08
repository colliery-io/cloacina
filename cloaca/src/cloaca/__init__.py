"""
Cloaca - Python bindings for Cloacina workflow orchestration engine.

This is the dispatcher package that automatically selects and loads
the appropriate backend (PostgreSQL or SQLite) based on availability
and environment configuration.
"""

import importlib
from typing import Any, Optional


__backend__: Optional[str] = None
__version__ = ...


def _validate_backend_version(module: Any, backend_name: str) -> None:
    """Validate that backend version matches dispatcher version."""
    if hasattr(module, "__version__"):
        backend_version = module.__version__
        if backend_version != __version__:
            raise ImportError(
                f"Version mismatch detected!\n"
                f"  Dispatcher version: {__version__}\n"
                f"  {backend_name.title()} backend version: {backend_version}\n"
                f"This indicates a package installation problem. Please reinstall:\n"
                f"  pip uninstall cloaca cloaca-{backend_name}\n"
                f"  pip install cloaca[{backend_name}]=={__version__}"
            )


def _load_backend() -> tuple[Any, str]:
    """Load the appropriate backend based on what's installed."""
    available_backends = []

    try:
        module = importlib.import_module("cloaca_postgres")
        _validate_backend_version(module, "postgres")
        available_backends.append(("postgres", module))
    except ImportError as e:
        # If it's a version mismatch, re-raise it immediately
        if "Version mismatch detected" in str(e):
            raise
        # Otherwise, backend just isn't installed, which is fine
        pass

    try:
        module = importlib.import_module("cloaca_sqlite")
        _validate_backend_version(module, "sqlite")
        available_backends.append(("sqlite", module))
    except ImportError as e:
        # If it's a version mismatch, re-raise it immediately
        if "Version mismatch detected" in str(e):
            raise
        # Otherwise, backend just isn't installed, which is fine
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
    if hasattr(_backend_module, "HelloClass"):
        HelloClass = _backend_module.HelloClass
    if hasattr(_backend_module, "Context"):
        Context = _backend_module.Context
    if hasattr(_backend_module, "DefaultRunnerConfig"):
        DefaultRunnerConfig = _backend_module.DefaultRunnerConfig
    if hasattr(_backend_module, "task"):
        task = _backend_module.task
    if hasattr(_backend_module, "DefaultRunner"):
        DefaultRunner = _backend_module.DefaultRunner
    if hasattr(_backend_module, "PipelineResult"):
        PipelineResult = _backend_module.PipelineResult
    if hasattr(_backend_module, "WorkflowBuilder"):
        WorkflowBuilder = _backend_module.WorkflowBuilder
    if hasattr(_backend_module, "Workflow"):
        Workflow = _backend_module.Workflow
    if hasattr(_backend_module, "register_workflow_constructor"):
        register_workflow_constructor = _backend_module.register_workflow_constructor

    print(__version__)

except ImportError as import_error:
    # If no backend is available, provide helpful error message
    error_msg = str(import_error)

    def _raise_no_backend(*args, **kwargs):
        raise ImportError(error_msg)

    # Create placeholder symbols that raise helpful errors
    hello_world = _raise_no_backend
    get_backend = _raise_no_backend

    __all__ = ["hello_world", "get_backend"]


def get_version() -> str:
    """Get the version of the cloaca package."""
    return __version__
