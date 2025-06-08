---
title: "Testing Workflows"
description: "How to test Cloaca workflows effectively"
weight: 10
---

# Testing Workflows

Learn how to write comprehensive tests for your Cloaca workflows, tasks, and error handling scenarios.

## Prerequisites

- Basic knowledge of Python testing frameworks
- Familiarity with Cloaca workflow concepts
- Understanding of task definitions and context

## Testing Framework Setup

### Using pytest (Recommended)

```bash
pip install pytest pytest-asyncio
```

### Basic Test Structure

```python
import pytest
import cloaca
from unittest.mock import Mock, patch

@pytest.fixture
def in_memory_runner():
    """Create an in-memory runner for testing."""
    runner = cloaca.DefaultRunner("sqlite:///:memory:")
    yield runner
    runner.shutdown()

@pytest.fixture
def sample_context():
    """Create a sample context for testing."""
    return cloaca.Context({
        "test_data": [1, 2, 3, 4, 5],
        "expected_result": "processed"
    })
```

## Testing Individual Tasks

### Basic Task Testing

```python
import cloaca

@cloaca.task(id="double_numbers")
def double_numbers(context):
    """Double all numbers in the input data."""
    numbers = context.get("numbers", [])
    doubled = [x * 2 for x in numbers]
    context.set("doubled_numbers", doubled)
    return context

def test_double_numbers():
    """Test the double_numbers task."""
    # Arrange
    context = cloaca.Context({"numbers": [1, 2, 3]})

    # Act
    result_context = double_numbers(context)

    # Assert
    expected = [2, 4, 6]
    assert result_context.get("doubled_numbers") == expected
```

### Testing Task Error Handling

```python
@cloaca.task(id="divide_numbers")
def divide_numbers(context):
    """Divide numbers by a divisor."""
    numbers = context.get("numbers", [])
    divisor = context.get("divisor", 1)

    if divisor == 0:
        context.set("error", "Division by zero")
        context.set("success", False)
        return context

    result = [x / divisor for x in numbers]
    context.set("result", result)
    context.set("success", True)
    return context

def test_divide_numbers_success():
    """Test successful division."""
    context = cloaca.Context({"numbers": [10, 20, 30], "divisor": 2})
    result = divide_numbers(context)

    assert result.get("success") is True
    assert result.get("result") == [5.0, 10.0, 15.0]

def test_divide_numbers_zero_division():
    """Test division by zero handling."""
    context = cloaca.Context({"numbers": [10, 20, 30], "divisor": 0})
    result = divide_numbers(context)

    assert result.get("success") is False
    assert result.get("error") == "Division by zero"
```

## Testing Complete Workflows

### Simple Workflow Testing

```python
def create_test_workflow():
    """Create a simple workflow for testing."""
    builder = cloaca.WorkflowBuilder("test_workflow")
    builder.description("Test workflow")
    builder.add_task("double_numbers")
    return builder.build()

def test_workflow_execution(in_memory_runner, sample_context):
    """Test complete workflow execution."""
    # Register workflow
    cloaca.register_workflow_constructor("test_workflow", create_test_workflow)

    # Execute workflow
    context = cloaca.Context({"numbers": [1, 2, 3]})
    result = in_memory_runner.execute("test_workflow", context)

    # Verify result
    assert result.status == cloaca.PipelineStatus.COMPLETED
    assert result.final_context.get("doubled_numbers") == [2, 4, 6]
```

### Testing Complex Dependencies

```python
@cloaca.task(id="fetch_data")
def fetch_data(context):
    """Simulate fetching data."""
    data = {"users": 100, "orders": 250}
    context.set("raw_data", data)
    return context

@cloaca.task(id="process_data", dependencies=["fetch_data"])
def process_data(context):
    """Process the fetched data."""
    raw_data = context.get("raw_data")
    processed = {
        "total_users": raw_data["users"],
        "avg_orders": raw_data["orders"] / raw_data["users"]
    }
    context.set("processed_data", processed)
    return context

def create_dependency_workflow():
    """Create workflow with dependencies."""
    builder = cloaca.WorkflowBuilder("dependency_workflow")
    builder.add_task("fetch_data")
    builder.add_task("process_data")
    return builder.build()

def test_dependency_workflow(in_memory_runner):
    """Test workflow with task dependencies."""
    cloaca.register_workflow_constructor("dependency_workflow", create_dependency_workflow)

    context = cloaca.Context({})
    result = in_memory_runner.execute("dependency_workflow", context)

    assert result.status == cloaca.PipelineStatus.COMPLETED

    processed = result.final_context.get("processed_data")
    assert processed["total_users"] == 100
    assert processed["avg_orders"] == 2.5
```

## Mocking External Dependencies

### Testing with Mock APIs

```python
from unittest.mock import patch, Mock
import requests

@cloaca.task(id="fetch_api_data")
def fetch_api_data(context):
    """Fetch data from external API."""
    api_url = context.get("api_url")
    try:
        response = requests.get(api_url)
        response.raise_for_status()
        data = response.json()
        context.set("api_data", data)
        context.set("success", True)
    except Exception as e:
        context.set("error", str(e))
        context.set("success", False)

    return context

@patch('requests.get')
def test_fetch_api_data_success(mock_get):
    """Test successful API data fetch."""
    # Mock successful response
    mock_response = Mock()
    mock_response.json.return_value = {"result": "success", "data": [1, 2, 3]}
    mock_response.raise_for_status.return_value = None
    mock_get.return_value = mock_response

    # Execute task
    context = cloaca.Context({"api_url": "https://api.example.com/data"})
    result = fetch_api_data(context)

    # Verify
    assert result.get("success") is True
    assert result.get("api_data") == {"result": "success", "data": [1, 2, 3]}
    mock_get.assert_called_once_with("https://api.example.com/data")

@patch('requests.get')
def test_fetch_api_data_failure(mock_get):
    """Test API data fetch failure."""
    # Mock failed response
    mock_get.side_effect = requests.exceptions.RequestException("Connection failed")

    # Execute task
    context = cloaca.Context({"api_url": "https://api.example.com/data"})
    result = fetch_api_data(context)

    # Verify error handling
    assert result.get("success") is False
    assert "Connection failed" in result.get("error")
```

## Testing Async Tasks

### Async Task Testing

```python
import asyncio

@cloaca.task(id="async_fetch")
async def async_fetch(context):
    """Async task that simulates network operation."""
    await asyncio.sleep(0.1)  # Simulate async operation

    url = context.get("url")
    result = f"Data from {url}"
    context.set("fetched_data", result)
    return context

@pytest.mark.asyncio
async def test_async_fetch():
    """Test async task execution."""
    context = cloaca.Context({"url": "https://example.com"})
    result = await async_fetch(context)

    assert result.get("fetched_data") == "Data from https://example.com"
```

## Testing Error Scenarios

### Workflow Failure Testing

```python
@cloaca.task(id="failing_task")
def failing_task(context):
    """Task that always fails for testing."""
    raise ValueError("Intentional failure for testing")

def create_failing_workflow():
    """Create workflow that will fail."""
    builder = cloaca.WorkflowBuilder("failing_workflow")
    builder.add_task("failing_task")
    return builder.build()

def test_workflow_failure(in_memory_runner):
    """Test workflow failure handling."""
    cloaca.register_workflow_constructor("failing_workflow", create_failing_workflow)

    context = cloaca.Context({})
    result = in_memory_runner.execute("failing_workflow", context)

    # Should handle failure gracefully
    assert result.status == cloaca.PipelineStatus.FAILED
```

### Testing Validation Errors

```python
def test_invalid_workflow_validation():
    """Test workflow validation errors."""
    with pytest.raises(cloaca.WorkflowValidationError):
        builder = cloaca.WorkflowBuilder("invalid_workflow")
        builder.add_task("non_existent_task")  # Should fail validation
        builder.build()
```

## Performance Testing

### Execution Time Testing

```python
import time

def test_workflow_performance(in_memory_runner):
    """Test workflow execution performance."""
    cloaca.register_workflow_constructor("test_workflow", create_test_workflow)

    context = cloaca.Context({"numbers": list(range(1000))})

    start_time = time.time()
    result = in_memory_runner.execute("test_workflow", context)
    execution_time = time.time() - start_time

    assert result.status == cloaca.PipelineStatus.COMPLETED
    assert execution_time < 1.0  # Should complete in less than 1 second
```

### Memory Usage Testing

```python
import tracemalloc

def test_workflow_memory_usage(in_memory_runner):
    """Test workflow memory usage."""
    tracemalloc.start()

    cloaca.register_workflow_constructor("test_workflow", create_test_workflow)
    context = cloaca.Context({"numbers": list(range(10000))})

    result = in_memory_runner.execute("test_workflow", context)

    current, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    assert result.status == cloaca.PipelineStatus.COMPLETED
    assert peak < 10 * 1024 * 1024  # Less than 10MB peak usage
```

## Integration Testing

### Database Integration Testing

```python
import tempfile
import os

@pytest.fixture
def temp_db_runner():
    """Create runner with temporary database file."""
    temp_db = tempfile.NamedTemporaryFile(delete=False, suffix='.db')
    temp_db.close()

    runner = cloaca.DefaultRunner(f"sqlite:///{temp_db.name}")
    yield runner

    runner.shutdown()
    os.unlink(temp_db.name)

def test_database_persistence(temp_db_runner):
    """Test that workflow state persists to database."""
    cloaca.register_workflow_constructor("test_workflow", create_test_workflow)

    context = cloaca.Context({"numbers": [1, 2, 3]})
    result = temp_db_runner.execute("test_workflow", context)

    assert result.status == cloaca.PipelineStatus.COMPLETED
    # Additional database state verification could be added here
```

## Test Organization

### Test Structure

```
tests/
├── conftest.py              # Shared fixtures
├── test_tasks.py            # Individual task tests
├── test_workflows.py        # Workflow integration tests
├── test_error_handling.py   # Error scenario tests
├── test_performance.py      # Performance tests
└── test_integration.py      # Full integration tests
```

### Shared Fixtures (conftest.py)

```python
import pytest
import cloaca

@pytest.fixture(scope="session")
def test_runner():
    """Session-scoped test runner."""
    runner = cloaca.DefaultRunner("sqlite:///:memory:")
    yield runner
    runner.shutdown()

@pytest.fixture
def clean_registry():
    """Clean workflow registry between tests."""
    # Store original registry
    original_registry = cloaca._workflow_registry.copy()

    yield

    # Restore original registry
    cloaca._workflow_registry.clear()
    cloaca._workflow_registry.update(original_registry)
```

## Best Practices

### Test Design Principles

1. **Isolation**: Each test should be independent
2. **Determinism**: Tests should produce consistent results
3. **Speed**: Tests should run quickly for frequent execution
4. **Coverage**: Test both success and failure paths
5. **Clarity**: Test names and structure should be self-documenting

### Common Patterns

```python
class TestDataProcessingWorkflow:
    """Organize related tests in classes."""

    @pytest.fixture(autouse=True)
    def setup_workflow(self):
        """Setup executed before each test."""
        cloaca.register_workflow_constructor("data_processing", create_data_workflow)

    def test_valid_input(self, in_memory_runner):
        """Test with valid input data."""
        # Test implementation
        pass

    def test_empty_input(self, in_memory_runner):
        """Test with empty input data."""
        # Test implementation
        pass

    def test_invalid_input(self, in_memory_runner):
        """Test with invalid input data."""
        # Test implementation
        pass
```

## See Also

- **[Error Handling Tutorial](/python-bindings/tutorials/04-error-handling/)** - Learn about error handling patterns
- **[API Reference](/python-bindings/api-reference/)** - Complete API documentation
- **[Performance Optimization](/python-bindings/how-to-guides/performance-optimization/)** - Optimize workflow performance
