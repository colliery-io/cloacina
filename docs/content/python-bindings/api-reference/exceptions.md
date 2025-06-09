---
title: "Exceptions"
description: "Exception classes for error handling in Cloaca workflows"
weight: 90
---

# Exceptions

Cloaca provides a hierarchy of exception classes for handling different types of errors that can occur during workflow definition, execution, and management.

## Exception Hierarchy

```
CloacaException (base)
├── WorkflowError
│   ├── WorkflowValidationError
│   ├── WorkflowExecutionError
│   └── WorkflowTimeoutError
├── TaskError
│   ├── TaskValidationError
│   ├── TaskExecutionError
│   └── TaskTimeoutError
├── ContextError
├── ConfigurationError
└── DatabaseError
    ├── ConnectionError
    └── MigrationError
```

## Base Exception

### CloacaException

Base exception class for all Cloaca-related errors.

```python
import cloaca

try:
    # Cloaca operation
    result = runner.execute("workflow", context)
except cloaca.CloacaException as e:
    print(f"Cloaca error: {e}")
```

## Workflow Exceptions

### WorkflowError

Base class for workflow-related errors.

```python
try:
    workflow = builder.build()
except cloaca.WorkflowError as e:
    print(f"Workflow error: {e}")
```

### WorkflowValidationError

Raised when workflow validation fails during build.

```python
import cloaca

@cloaca.task(id="task_a")
def task_a(context):
    return context

@cloaca.task(id="task_b", dependencies=["non_existent_task"])
def task_b(context):
    return context

try:
    builder = cloaca.WorkflowBuilder("invalid_workflow")
    builder.add_task("task_a")
    builder.add_task("task_b")
    workflow = builder.build()  # Raises WorkflowValidationError
except cloaca.WorkflowValidationError as e:
    print(f"Validation failed: {e}")
    # Handle missing dependency error
```

Common validation errors:
- Missing task dependencies
- Circular dependencies
- Duplicate task IDs
- Invalid task references

### WorkflowExecutionError

Raised when workflow execution fails unexpectedly.

```python
try:
    result = runner.execute("my_workflow", context)
except cloaca.WorkflowExecutionError as e:
    print(f"Execution failed: {e}")
    print(f"Workflow: {e.workflow_name}")
    print(f"Execution ID: {e.execution_id}")
```

### WorkflowTimeoutError

Raised when workflow execution exceeds the timeout limit.

```python
import cloaca

# Configure with timeout
config = cloaca.DefaultRunnerConfig(task_timeout_seconds=30)
runner = cloaca.DefaultRunner("sqlite:///:memory:", config)

try:
    result = runner.execute("long_workflow", context)
except cloaca.WorkflowTimeoutError as e:
    print(f"Workflow timed out after {e.timeout_seconds} seconds")
    print(f"Partial result available: {e.partial_result}")
```

## Task Exceptions

### TaskError

Base class for task-related errors.

```python
@cloaca.task(id="risky_task")
def risky_task(context):
    try:
        # Risky operation
        result = perform_operation()
        context.set("result", result)
    except Exception as e:
        # Convert to TaskError
        raise cloaca.TaskError(f"Task failed: {e}") from e

    return context
```

### TaskValidationError

Raised when task definition is invalid.

```python
try:
    @cloaca.task(id="")  # Empty ID
    def invalid_task(context):
        return context
except cloaca.TaskValidationError as e:
    print(f"Invalid task definition: {e}")
```

### TaskExecutionError

Raised when task execution fails.

```python
@cloaca.task(id="failing_task")
def failing_task(context):
    try:
        # Operation that might fail
        result = risky_operation()
        context.set("result", result)
    except Exception as e:
        # Wrap in TaskExecutionError with context
        raise cloaca.TaskExecutionError(
            f"Task execution failed: {e}",
            task_id="failing_task",
            context=context
        ) from e

    return context
```

### TaskTimeoutError

Raised when individual task execution times out.

```python
@cloaca.task(id="slow_task", timeout_seconds=60)
def slow_task(context):
    # This will raise TaskTimeoutError if it takes > 60 seconds
    time.sleep(120)  # Simulates long operation
    return context

try:
    result = runner.execute("workflow_with_slow_task", context)
except cloaca.TaskTimeoutError as e:
    print(f"Task {e.task_id} timed out after {e.timeout_seconds} seconds")
```

## Context Exceptions

### ContextError

Raised for context-related errors.

```python
try:
    # Try to get non-existent required data
    value = context.get_required("missing_key")
except cloaca.ContextError as e:
    print(f"Context error: {e}")
    # Handle missing required data
```

## Configuration Exceptions

### ConfigurationError

Raised for invalid configuration.

```python
try:
    config = cloaca.DefaultRunnerConfig(
        max_concurrent_workflows=-5  # Invalid negative value
    )
except cloaca.ConfigurationError as e:
    print(f"Configuration error: {e}")
```

## Database Exceptions

### DatabaseError

Base class for database-related errors.

```python
try:
    runner = cloaca.DefaultRunner("invalid://database/url")
except cloaca.DatabaseError as e:
    print(f"Database error: {e}")
```

### ConnectionError

Raised when database connection fails.

```python
try:
    runner = cloaca.DefaultRunner("postgresql://user:pass@nonexistent:5432/db")
    result = runner.execute("workflow", context)
except cloaca.ConnectionError as e:
    print(f"Database connection failed: {e}")
    print(f"Database URL: {e.database_url}")
```

### MigrationError

Raised when database migration fails.

```python
try:
    runner = cloaca.DefaultRunner("sqlite:///readonly.db")
except cloaca.MigrationError as e:
    print(f"Database migration failed: {e}")
    print(f"Migration version: {e.target_version}")
```

## Error Handling Patterns

### Comprehensive Error Handling

```python
import cloaca

def execute_workflow_safely(runner, workflow_name, context):
    """Execute workflow with comprehensive error handling."""
    try:
        result = runner.execute(workflow_name, context)

        if result.status == cloaca.PipelineStatus.COMPLETED:
            return result.final_context
        else:
            raise cloaca.WorkflowExecutionError(
                f"Workflow failed with status: {result.status}"
            )

    except cloaca.WorkflowValidationError as e:
        print(f"Workflow validation failed: {e}")
        return None

    except cloaca.TaskTimeoutError as e:
        print(f"Task {e.task_id} timed out")
        return None

    except cloaca.WorkflowTimeoutError as e:
        print(f"Workflow timed out after {e.timeout_seconds}s")
        return e.partial_result

    except cloaca.DatabaseError as e:
        print(f"Database error: {e}")
        return None

    except cloaca.CloacaException as e:
        print(f"Unexpected Cloaca error: {e}")
        return None

    except Exception as e:
        print(f"Unexpected error: {e}")
        return None
```

### Retry with Exception Handling

```python
import time
import random

def execute_with_retry(runner, workflow_name, context, max_attempts=3):
    """Execute workflow with retry logic."""
    for attempt in range(max_attempts):
        try:
            return runner.execute(workflow_name, context)

        except cloaca.ConnectionError as e:
            if attempt < max_attempts - 1:
                wait_time = (2 ** attempt) + random.uniform(0, 1)
                print(f"Connection failed, retrying in {wait_time:.1f}s...")
                time.sleep(wait_time)
                continue
            else:
                print(f"All {max_attempts} attempts failed")
                raise

        except cloaca.TaskTimeoutError as e:
            print(f"Task timeout on attempt {attempt + 1}")
            if attempt < max_attempts - 1:
                continue
            else:
                raise

        except (cloaca.WorkflowValidationError, cloaca.ConfigurationError):
            # Don't retry validation or configuration errors
            raise

        except cloaca.CloacaException as e:
            print(f"Cloaca error on attempt {attempt + 1}: {e}")
            if attempt < max_attempts - 1:
                time.sleep(1)
                continue
            else:
                raise
```

### Custom Exception Handling

```python
class WorkflowManager:
    """Workflow manager with custom error handling."""

    def __init__(self, runner):
        self.runner = runner
        self.error_handlers = {
            cloaca.TaskTimeoutError: self._handle_task_timeout,
            cloaca.WorkflowTimeoutError: self._handle_workflow_timeout,
            cloaca.DatabaseError: self._handle_database_error,
        }

    def execute_workflow(self, name, context):
        """Execute workflow with custom error handling."""
        try:
            return self.runner.execute(name, context)
        except Exception as e:
            # Find appropriate handler
            for exception_type, handler in self.error_handlers.items():
                if isinstance(e, exception_type):
                    return handler(e, name, context)

            # No specific handler found
            return self._handle_generic_error(e, name, context)

    def _handle_task_timeout(self, error, workflow_name, context):
        print(f"Task {error.task_id} timed out in workflow {workflow_name}")
        # Could implement partial result recovery
        return None

    def _handle_workflow_timeout(self, error, workflow_name, context):
        print(f"Workflow {workflow_name} timed out")
        return error.partial_result  # Return partial results

    def _handle_database_error(self, error, workflow_name, context):
        print(f"Database error during {workflow_name}: {error}")
        # Could implement fallback to different database
        return None

    def _handle_generic_error(self, error, workflow_name, context):
        print(f"Unexpected error in {workflow_name}: {error}")
        return None
```

## Best Practices

### Exception Information

Always preserve exception context:

```python
try:
    result = runner.execute("workflow", context)
except cloaca.TaskExecutionError as e:
    # Access exception details
    print(f"Task ID: {e.task_id}")
    print(f"Error message: {e}")
    print(f"Original exception: {e.__cause__}")

    # Access context if available
    if hasattr(e, 'context'):
        error_context = e.context
        print(f"Context at error: {error_context.data}")
```

### Logging Exceptions

Implement proper logging:

```python
import logging

logger = logging.getLogger(__name__)

try:
    result = runner.execute("workflow", context)
except cloaca.CloacaException as e:
    logger.error(
        "Workflow execution failed",
        extra={
            "workflow_name": getattr(e, 'workflow_name', 'unknown'),
            "error_type": type(e).__name__,
            "error_message": str(e)
        },
        exc_info=True
    )
```

## See Also

- **[Task Decorator]({{< ref "/python-bindings/api-reference/task/" >}})** - Task-level error handling
- **[DefaultRunner]({{< ref "/python-bindings/api-reference/runner/" >}})** - Workflow execution and errors
- **[Configuration]({{< ref "/python-bindings/api-reference/configuration/" >}})** - Configuration validation errors
