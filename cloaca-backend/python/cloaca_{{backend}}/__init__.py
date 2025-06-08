"""
{{backend|title}} backend for Cloaca - Python bindings for Cloacina workflow orchestration.
"""

# Import from the extension module built by maturin
{% if backend == "postgres" -%}
from .cloaca_{{backend}} import hello_world, get_backend, HelloClass, Context, DefaultRunnerConfig, task, DefaultRunner, PipelineResult, WorkflowBuilder, Workflow, register_workflow_constructor, DatabaseAdmin, TenantConfig, TenantCredentials, __backend__
{% else -%}
from .cloaca_{{backend}} import hello_world, get_backend, HelloClass, Context, DefaultRunnerConfig, task, DefaultRunner, PipelineResult, WorkflowBuilder, Workflow, register_workflow_constructor, __backend__
{% endif -%}

# __version__ is automatically provided by maturin from Cargo.toml

{% if backend == "postgres" -%}
__all__ = [
    "hello_world",
    "get_backend",
    "HelloClass",
    "Context",
    "DefaultRunnerConfig",
    "task",
    "DefaultRunner",
    "PipelineResult",
    "WorkflowBuilder",
    "Workflow",
    "register_workflow_constructor",
    "DatabaseAdmin",
    "TenantConfig",
    "TenantCredentials",
    "__backend__",
]
{% else -%}
__all__ = [
    "hello_world",
    "get_backend",
    "HelloClass",
    "Context",
    "DefaultRunnerConfig",
    "task",
    "DefaultRunner",
    "PipelineResult",
    "WorkflowBuilder",
    "Workflow",
    "register_workflow_constructor",
    "__backend__",
]
{% endif -%}
