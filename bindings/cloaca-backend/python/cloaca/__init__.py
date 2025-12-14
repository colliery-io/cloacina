"""
Cloaca - Python bindings for Cloacina workflow orchestration engine.

This unified package supports both PostgreSQL and SQLite backends.
Backend selection happens at runtime based on the connection URL.
"""

# Import from the native extension module built by maturin
from cloaca.cloaca import (
    # Test functions
    hello_world,
    HelloClass,
    # Core classes
    Context,
    DefaultRunnerConfig,
    DefaultRunner,
    PipelineResult,
    # Task decorator
    task,
    # Trigger decorator and result class
    trigger,
    TriggerResult,
    # Workflow classes
    WorkflowBuilder,
    Workflow,
    register_workflow_constructor,
    # Value objects
    TaskNamespace,
    WorkflowContext,
    RetryPolicy,
    RetryPolicyBuilder,
    BackoffStrategy,
    RetryCondition,
    # Admin classes (PostgreSQL-specific, runtime error if used with SQLite)
    DatabaseAdmin,
    TenantConfig,
    TenantCredentials,
)

__all__ = [
    # Test functions
    "hello_world",
    "HelloClass",
    # Core classes
    "Context",
    "DefaultRunnerConfig",
    "DefaultRunner",
    "PipelineResult",
    # Task decorator
    "task",
    # Trigger decorator and result class
    "trigger",
    "TriggerResult",
    # Workflow classes
    "WorkflowBuilder",
    "Workflow",
    "register_workflow_constructor",
    # Value objects
    "TaskNamespace",
    "WorkflowContext",
    "RetryPolicy",
    "RetryPolicyBuilder",
    "BackoffStrategy",
    "RetryCondition",
    # Admin classes
    "DatabaseAdmin",
    "TenantConfig",
    "TenantCredentials",
]
