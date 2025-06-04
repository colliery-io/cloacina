"""
{{backend|title}} backend for Cloaca - Python bindings for Cloacina workflow orchestration.
"""

# Import from the extension module built by maturin
from .cloaca_{{backend}} import hello_world, get_backend, HelloClass, Context, DefaultRunnerConfig, __backend__

# __version__ is automatically provided by maturin from Cargo.toml

__all__ = [
    "hello_world",
    "get_backend",
    "HelloClass",
    "Context",
    "DefaultRunnerConfig",
    "__backend__",
]