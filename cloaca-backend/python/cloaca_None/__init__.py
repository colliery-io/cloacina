"""
None backend for Cloaca - Python bindings for Cloacina workflow orchestration.
"""

# Import from the extension module built by maturin
from .cloaca_None import hello_world, get_backend, HelloClass, Context, DefaultRunnerConfig, task, WorkflowBuilder, Workflow, __backend__

# __version__ is automatically provided by maturin from Cargo.toml

__all__ = [
    "hello_world",
    "get_backend",
    "HelloClass",
    "Context",
    "DefaultRunnerConfig",
    "task",
    "WorkflowBuilder",
    "Workflow",
    "__backend__",
]