# Cloacina Python Bindings Test Suite

Comprehensive test suite for Cloacina's Python bindings, covering the dispatcher package and both PostgreSQL and SQLite backends.

## Test Architecture

The test suite is organized into three main categories:

### 🔧 Unit Tests (`tests/unit/`)
- Test individual components in isolation
- Mock external dependencies  
- Fast execution, no database required
- Focus on @task decorator, task registry, basic functionality

### 🔗 Integration Tests (`tests/integration/`)
- Test backend-specific functionality
- Require actual database connections
- Test full pipeline execution
- Separate test files for PostgreSQL and SQLite backends

### 📦 Dispatcher Tests (`tests/dispatcher/`) 
- Test the main `cloacina` package logic
- Test backend selection and import resolution
- Test error handling when backends are missing
- Mock backend packages for isolated testing

## Running Tests

### Using Angreal (Recommended)

```bash
# Run all Python binding tests
angreal tests python

# Run tests for specific backend
angreal tests python --backend postgres
angreal tests python --backend sqlite

# Run specific test types
angreal tests python --type unit
angreal tests python --type integration  
angreal tests python --type dispatcher

# Run all tests (includes Python tests)
angreal tests all
```

### Using Pytest Directly

```bash
# Install test dependencies
pip install -r python-tests/requirements-test.txt

# Run all tests
pytest python-tests/ -v

# Run with backend-specific markers
pytest python-tests/ -m postgres -v
pytest python-tests/ -m sqlite -v

# Run specific test types
pytest python-tests/ -m unit -v
pytest python-tests/ -m integration -v
pytest python-tests/ -m dispatcher -v

# Run tests in parallel
pytest python-tests/ -n auto
```

## Test Markers

The test suite uses pytest markers to categorize and filter tests:

- `@pytest.mark.unit` - Unit tests
- `@pytest.mark.integration` - Integration tests
- `@pytest.mark.postgres` - PostgreSQL backend tests
- `@pytest.mark.sqlite` - SQLite backend tests
- `@pytest.mark.dispatcher` - Dispatcher package tests
- `@pytest.mark.slow` - Tests that take longer than usual
- `@pytest.mark.network` - Tests requiring network access

## Test Environment Setup

### Automatic Setup

The test suite automatically:
- Creates temporary SQLite databases for testing
- Sets up test environment variables
- Skips tests when required backends aren't available
- Installs test dependencies when run via angreal

### Manual Setup

For direct pytest usage:

```bash
# Install test dependencies
pip install -r python-tests/requirements-test.txt

# Set up PostgreSQL test database (optional)
export TEST_DATABASE_URL="postgresql://user:pass@localhost:5432/cloacina_test"

# SQLite tests use temporary files automatically
```

## Test Coverage

### What's Tested ✅

**Unit Tests:**
- @task decorator functionality
- Task registration and dependency handling
- Context flow between tasks
- Error handling in individual components

**Integration Tests - PostgreSQL:**
- Backend package import and initialization
- Full pipeline execution with PostgreSQL
- Database schema creation and migrations
- Connection error handling

**Integration Tests - SQLite:**
- Backend package import and initialization  
- Full pipeline execution with SQLite
- Database file creation and permissions
- SQLite-specific features (WAL mode, etc.)

**Dispatcher Tests:**
- Backend detection and import resolution
- Error messages when no backend available
- API compatibility across backends
- Installation flow testing

### What Needs Implementation ⚠️

Many test functions are currently placeholder stubs that need implementation:

**High Priority:**
- Actual dispatcher package import testing
- Full pipeline execution verification
- Context persistence testing across backends
- Database schema validation

**Medium Priority:**
- Performance benchmarking tests
- Concurrent execution testing
- Memory usage profiling
- Large context data handling

**Low Priority:**
- Network integration tests
- Backup/restore functionality
- Advanced SQLite features testing

## Test Structure

```
python-tests/
├── pytest.ini              # Pytest configuration
├── requirements-test.txt    # Test dependencies
├── tests/
│   ├── conftest.py         # Shared fixtures and setup
│   ├── unit/
│   │   └── test_task_decorator.py
│   ├── integration/
│   │   ├── test_postgres_backend.py
│   │   └── test_sqlite_backend.py
│   └── dispatcher/
│       └── test_dispatcher.py
└── README.md               # This file
```

## Adding New Tests

### Unit Tests

Add to `tests/unit/` for testing individual components:

```python
import pytest

@pytest.mark.unit
def test_my_feature():
    # Test individual component
    pass
```

### Integration Tests

Add to `tests/integration/` for backend-specific testing:

```python
import pytest

@pytest.mark.postgres
@pytest.mark.integration
def test_postgres_feature():
    # Test PostgreSQL-specific functionality
    pass

@pytest.mark.sqlite  
@pytest.mark.integration
def test_sqlite_feature():
    # Test SQLite-specific functionality
    pass
```

### Dispatcher Tests

Add to `tests/dispatcher/` for testing the main package:

```python
import pytest

@pytest.mark.dispatcher
def test_dispatcher_feature():
    # Test main cloacina package behavior
    pass
```

## Continuous Integration

The test suite is designed to work in CI environments:

### GitHub Actions Matrix

```yaml
strategy:
  matrix:
    python-version: [3.9, 3.10, 3.11, 3.12]
    backend: [postgres, sqlite]
    test-type: [unit, integration, dispatcher]
```

### Test Execution Strategy

1. **Unit Tests**: Run on all Python versions, no external dependencies
2. **SQLite Integration**: Run on all Python versions, no setup required
3. **PostgreSQL Integration**: Run with PostgreSQL service container
4. **Dispatcher Tests**: Run on all combinations to ensure compatibility

## Debugging Tests

### Running Individual Tests

```bash
# Run specific test file
pytest python-tests/tests/unit/test_task_decorator.py -v

# Run specific test function
pytest python-tests/tests/unit/test_task_decorator.py::TestTaskDecorator::test_basic_task_registration -v

# Run with debugging output
pytest python-tests/ -v -s --tb=long
```

### Common Issues

**Backend Not Available:**
- Tests are automatically skipped when backends aren't installed
- Install with `pip install cloacina-postgres` or `pip install cloacina-sqlite`

**Database Connection Errors:**
- PostgreSQL tests require a running PostgreSQL instance
- Set `TEST_DATABASE_URL` environment variable for custom connection
- SQLite tests use temporary files and should always work

**Import Errors:**
- Ensure the backend packages are built and available
- Run `maturin develop` in the backend package directories

## Contributing

When adding new Python binding features:

1. **Add unit tests** for individual components
2. **Add integration tests** for both backends (if applicable)
3. **Update dispatcher tests** if changing the main package API
4. **Update test markers** for new test categories
5. **Document test coverage** for new functionality

## Performance Testing

Some tests are marked as `@pytest.mark.slow` for performance testing:

```bash
# Skip slow tests (default)
pytest python-tests/

# Include slow tests
pytest python-tests/ --runslow
```

These tests help ensure the Python bindings maintain good performance characteristics as the codebase grows.