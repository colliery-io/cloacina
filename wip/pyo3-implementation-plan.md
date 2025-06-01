# PyO3 Implementation Plan for Cloacina

This document provides a comprehensive implementation plan for the PyO3 feature that enables Python bindings for the Cloacina workflow engine. This plan captures the design decisions, architecture, and lessons learned from the initial implementation attempts.

## Overview

The PyO3 feature provides Python bindings for Cloacina, allowing Python developers to define workflows and tasks using Python decorators while leveraging the high-performance Rust execution engine underneath.

### Key Goals

1. **Python-First API**: Provide a native Python experience with decorators and familiar Python patterns
2. **Backend Agnostic**: Support both PostgreSQL and SQLite backends through separate packages
3. **Auto-Discovery**: Automatically discover and import available backend packages
4. **Performance**: Maintain Rust performance benefits while providing Python convenience
5. **Type Safety**: Bridge Python dynamic typing with Rust static typing safely

## Architecture Overview

### Package Structure

```
cloacina-postgres/          # PostgreSQL backend Python package
├── src/                    # Rust PyO3 implementation
├── python/                 # Python wrapper code
└── pyproject.toml         # Package configuration

cloacina-sqlite/           # SQLite backend Python package
├── src/                   # Rust PyO3 implementation
├── python/                # Python wrapper code
└── pyproject.toml        # Package configuration

cloacina-dispatcher/       # Python dispatcher package
├── src/                   # Pure Python auto-discovery
└── pyproject.toml        # Package configuration
```

### Design Principles

1. **Modular Backend Design**: Each database backend is a separate Python package built with PyO3
2. **Dynamic Import**: The dispatcher automatically discovers and imports available backends
3. **Global Task Registry**: Rust maintains a global registry of Python tasks across all packages
4. **Context Marshalling**: Bidirectional conversion between Python dicts and Rust Context types
5. **Async Bridge**: Use pyo3-asyncio to bridge Python asyncio and Rust tokio

## Implementation Components

### 1. Task Registration System

#### Python Decorator Interface

```python
from cloacina import task

@task(task_id="process_data")
def process_data(context):
    # Python task implementation
    return {"processed": True}
```

#### Rust Global Registry

**CORRECTED DESIGN**: The task registry should be implemented as a single global static in the main `cloacina` crate:

```rust
// cloacina/src/task.rs (new python module)
#[cfg(feature = "python")]
mod python {
    use once_cell::sync::Lazy;
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;

    static GLOBAL_PYTHON_TASK_REGISTRY: Lazy<Arc<Mutex<HashMap<String, Arc<PythonTask>>>>> =
        Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

    // Single implementation of PythonTask and registry functions
}
```

**Design Decision Correction**: Upon review, a single registry in the main `cloacina` crate is the superior approach because:

1. **Single Source of Truth**: One registry shared across all backends
2. **No Code Duplication**: Eliminates identical implementations in each backend
3. **Cross-Backend Compatibility**: Tasks work consistently regardless of backend choice
4. **Simplified Maintenance**: Changes only need to be made in one place
5. **Future-Proof**: New backends automatically inherit task functionality

**Current Implementation Issue**: The existing code duplicates the entire `PythonTask` implementation and registry in both `cloacina-postgres/src/task.rs` and `cloacina-sqlite/src/task.rs`, leading to maintenance burden and potential inconsistencies.

**Recommended Architecture Change**:

1. Add `python` feature to main `cloacina` crate with PyO3 dependencies:
```toml
# cloacina/Cargo.toml
[features]
python = ["pyo3", "pyo3-asyncio", "once_cell"]

[dependencies]
pyo3 = { version = "0.21", features = ["extension-module"], optional = true }
pyo3-asyncio = { version = "0.21", features = ["tokio-runtime"], optional = true }
```

2. Move `PythonTask` and registry to `cloacina/src/task.rs`
3. Backend packages become thin wrappers:
```rust
// cloacina-postgres/src/task.rs and cloacina-sqlite/src/task.rs
pub use cloacina::task::python::*;
```

### 2. Context Marshalling

#### Python to Rust Conversion

```rust
// cloacina-postgres/src/context.rs, cloacina-sqlite/src/context.rs
pub fn context_from_python(py_dict: &Bound<'_, pyo3::types::PyDict>) -> PyResult<cloacina::Context<serde_json::Value>> {
    let mut context = cloacina::Context::<serde_json::Value>::new();

    for (key, value) in py_dict.iter() {
        let key_str: String = key.extract()?;
        let json_value: serde_json::Value = pythonize::depythonize_bound(value)?;
        context.set(&key_str, json_value);
    }

    Ok(context)
}
```

#### Rust to Python Conversion

```rust
pub fn context_to_python(context: &cloacina::Context<serde_json::Value>, py: Python<'_>) -> PyResult<PyObject> {
    let dict = pyo3::types::PyDict::new_bound(py);

    for (key, value) in context.data().iter() {
        let py_value = pythonize::pythonize(py, value)?;
        dict.set_item(key, py_value)?;
    }

    Ok(dict.to_object(py))
}
```

**Design Decision**: Use `pythonize` crate for JSON-Python conversion to handle complex nested structures safely.

### 3. Executor Implementation

#### PyUnifiedExecutor Structure

```rust
#[pyclass(name = "UnifiedExecutor")]
pub struct PyUnifiedExecutor {
    inner: Arc<RwLock<Option<UnifiedExecutor>>>,
    config: UnifiedExecutorConfig,
}
```

**Design Decision**: Use `Arc<RwLock<Option<T>>>` pattern to allow for lazy initialization and thread-safe access.

#### Database URL Handling

**Design Approach**: Different backends naturally require different URL formats:

- PostgreSQL: `postgresql://localhost:5432/cloacina`
- SQLite: `sqlite://:memory:` or `sqlite:///path/to/db.sqlite`

**Recommended Implementation**:
```rust
// Make database_url required - no defaults
#[pyo3(signature = (database_url))]
pub fn initialize<'py>(
    &self,
    database_url: &str,
    py: Python<'py>,
) -> PyResult<Bound<'py, PyAny>> {
    // Validate URL format matches backend expectations
    self.validate_database_url(database_url)?;
    // ... rest of implementation
}
```

**Design Decision**: Making `database_url` required rather than providing defaults because:
1. **Explicit Configuration**: Forces users to consciously choose their database configuration
2. **Prevents Accidents**: Avoids unintended use of in-memory databases in production
3. **Clear Intent**: Makes database choice explicit in user code
4. **Backend Agnostic**: Each backend can validate its own URL format requirements

**URL Validation**: Each backend should validate that the provided URL matches expected format:
```rust
impl PyUnifiedExecutor {
    fn validate_database_url(&self, url: &str) -> PyResult<()> {
        // PostgreSQL backend: ensure URL starts with postgresql://
        // SQLite backend: ensure URL starts with sqlite:// or is a file path
        if !self.is_valid_url_format(url) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid database URL format for this backend: {}", url)
            ));
        }
        Ok(())
    }
}

### 4. Workflow Execution

#### Current Implementation Status

**SQLite Package**: Implements real task execution that iterates through registered Python tasks:

```rust
// Get all registered Python tasks
let python_tasks = crate::task::get_all_python_tasks();

// Execute tasks sequentially
for python_task in python_tasks {
    match python_task.execute(final_context.clone()).await {
        Ok(updated_context) => {
            final_context = updated_context;
            // Create successful TaskResult
        }
        Err(e) => {
            // Create failed TaskResult and return error
        }
    }
}
```

**PostgreSQL Package**: Still uses mock execution returning placeholder results.

**Design Decision**: Start with sequential execution for simplicity, add dependency resolution later.

### 5. Auto-Discovery Dispatcher

#### Package Discovery Logic

```python
# cloacina-dispatcher/src/cloacina/__init__.py
def _discover_backends():
    """Discover available cloacina backend packages."""
    backends = {}

    for backend_name in ['postgres', 'sqlite']:
        package_name = f'cloacina_{backend_name}'
        try:
            backend_module = importlib.import_module(package_name)
            backends[backend_name] = backend_module
        except ImportError:
            continue

    return backends
```

**Design Decision**: Use importlib for dynamic discovery rather than setuptools entry points for simplicity.

## Build System

### Maturin Configuration

Each backend package uses maturin for building Python wheels:

```toml
# pyproject.toml for each backend
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "cloacina-postgres"  # or cloacina-sqlite
requires-python = ">=3.9"
dependencies = [
    "cloacina-dispatcher",
]

[tool.maturin]
features = ["pyo3/extension-module"]
python-source = "python"
```

**Design Decision**: Use `python-source = "python"` to include Python wrapper code alongside Rust bindings.

## Critical Dependencies

### Runtime Dependencies

1. **PyO3**: Rust-Python bindings framework
2. **pyo3-asyncio**: Bridge between Python asyncio and Rust tokio
3. **pythonize**: JSON-Python conversion
4. **serde_json**: JSON handling in Rust
5. **uuid**: UUID generation for execution tracking
6. **chrono**: Timestamp handling

### Version Constraints

**Critical**: Must use `pyo3-asyncio-0-21` (version 0.21) to match PyO3 version compatibility.

```rust
use pyo3_asyncio_0_21 as pyo3_asyncio;
```

## Known Issues and Solutions

### 1. PyO3-Angreal Testing Integration

**Issue**: PyO3 runtime creates TypeId conflicts when run through angreal testing harness.

**Root Cause**: PyO3 expects isolated Python environments, but angreal may interfere with runtime initialization.

**Solution Strategy**: Design testing approach that uses angreal harness with proper virtual environment isolation:
```bash
# Goal: Make this work properly with virtual env isolation
angreal task python_tests

# Implementation should handle PyO3 runtime isolation within harness
```

**Requirements**:
- Tests must run from angreal harness (project requirement)
- Use virtual environments to isolate PyO3 runtime
- Ensure reproducible test environment setup

### 2. Mock vs Real Execution Inconsistency

**Issue**: Backend packages have inconsistent execution implementations:
- SQLite package: Real task execution with Python function calls
- PostgreSQL package: Mock execution returning fake `PipelineResult`

**Solution**: Eliminate all mocking - implement real execution in both backends:
```rust
// Both backends should use real execution like SQLite currently does
let python_tasks = crate::task::get_all_python_tasks();
for python_task in python_tasks {
    match python_task.execute(final_context.clone()).await {
        Ok(updated_context) => {
            final_context = updated_context;
            // Record real TaskResult
        }
        Err(e) => {
            // Handle real execution failure
        }
    }
}
```

### 3. Testing Strategy: Functional Tests Only

**Approach**: Use entirely functional testing for Python packages rather than unit testing individual components.

**Rationale**:
- PyO3 integration is best tested end-to-end
- Functional tests catch integration issues that unit tests miss
- Simpler to maintain and understand
- Tests real user workflows

**Implementation**:
```python
# Test complete workflows end-to-end
def test_complete_workflow_execution():
    # 1. Register tasks using @task decorator
    # 2. Create executor and initialize with real database
    # 3. Execute workflow and verify results
    # 4. Check database state and task execution results
```

## Testing Strategy

### Functional Testing (Primary Approach)

**Philosophy**: Test complete user workflows end-to-end rather than individual components.

```python
# python-tests/tests/functional/test_workflow_execution.py
import pytest
import tempfile
import cloacina_sqlite
import cloacina_postgres

def test_sqlite_complete_workflow():
    """Test complete workflow execution with SQLite backend"""
    # Create temporary database
    with tempfile.NamedTemporaryFile(suffix='.db') as db_file:
        # Register tasks using decorator
        @cloacina_sqlite.task(task_id="process_data")
        def process_data(context):
            return {"processed": True, "count": context.get("count", 0) + 1}

        # Create and initialize executor
        executor = cloacina_sqlite.UnifiedExecutor()
        await executor.initialize(f"sqlite:///{db_file.name}")

        # Execute workflow
        workflow = cloacina_sqlite.Workflow("test_workflow")
        result = await executor.execute(workflow, {"input": "test"})

        # Verify results
        assert result["status"] == "Completed"
        assert result["final_context"]["processed"] is True
        assert result["final_context"]["count"] == 1

def test_postgres_complete_workflow():
    """Test complete workflow execution with PostgreSQL backend"""
    # Similar functional test for PostgreSQL
    pass

def test_cross_backend_task_isolation():
    """Verify tasks registered in one backend don't interfere with another"""
    # Test that task registries are properly isolated/shared as designed
    pass
```

### Build and Installation Testing

```bash
# Angreal-based testing harness (must work with PyO3)
angreal task python_tests

# Manual verification steps
maturin build --release
pip install target/wheels/cloacina_sqlite-*.whl
python -c "import cloacina_sqlite; print('Success')"
```

### Database Testing

- **SQLite**: Use temporary files and in-memory databases
- **PostgreSQL**: Use test containers or local test database
- **Real Databases**: No mocking - test against actual database backends

## Development Workflow

### Initial Setup

1. **Clone Repository**: Get the cloacina repository
2. **Install Rust**: Ensure Rust toolchain is installed
3. **Install Python Dependencies**: `pip install maturin pytest`
4. **Build Packages**: Use maturin to build each backend package

### Development Cycle

1. **Make Changes**: Edit Rust or Python source code
2. **Build Package**: `maturin develop` for development builds
3. **Run Tests**: Execute unit and integration tests
4. **Debug Issues**: Use direct Python testing, not angreal

### Release Process

1. **Update Versions**: Bump version numbers in pyproject.toml files
2. **Build Wheels**: `maturin build --release` for each package
3. **Test Installation**: Install and test wheels locally
4. **Publish**: Upload to PyPI (when ready)

## Future Enhancements

### 1. Architecture Improvements (High Priority)

- **~~Single Registry Implementation~~**: ✅ Addressed - consolidate task registry to main `cloacina` crate
- **Angreal-PyO3 Integration**: Fix testing harness to work with PyO3 using virtual environment isolation
- **Real Execution Implementation**: Replace mock execution in PostgreSQL package with real task execution
- **Task-Workflow Association**: Proper workflow-task relationships and dependency resolution

### 2. Advanced Task Features

- **Async Python Tasks**: Support for `async def` Python functions
- **Task Dependencies**: Implement proper dependency resolution
- **Task Retry Logic**: Add retry mechanisms for failed tasks
- **Task Timeouts**: Implement per-task timeout handling

### 3. Workflow Management

- **Workflow Validation**: Pre-execution workflow validation
- **Workflow Versioning**: Support for multiple workflow versions
- **Conditional Execution**: Support for conditional task execution
- **Parallel Execution**: Implement parallel task execution

### 4. Error Handling

- **Better Error Messages**: Improve Python-Rust error translation
- **Error Recovery**: Implement error recovery mechanisms
- **Debugging Support**: Add debugging and introspection tools

### 5. Performance Optimization

- **Connection Pooling**: Optimize database connection usage
- **Memory Management**: Optimize Python-Rust data transfer
- **Batch Operations**: Support for batch task execution

## Security Considerations

### 1. Input Validation

- **Context Validation**: Validate Python input before conversion to Rust
- **SQL Injection Prevention**: Use parameterized queries
- **Type Safety**: Maintain type safety across Python-Rust boundary

### 2. Resource Management

- **Memory Limits**: Implement memory usage limits
- **Connection Limits**: Limit database connections
- **Execution Timeouts**: Prevent runaway task execution

## Conclusion

This implementation plan provides a comprehensive roadmap for the PyO3 feature implementation. The architecture balances Python convenience with Rust performance while maintaining modularity and extensibility.

Key success factors:
1. **Incremental Development**: Build and test components incrementally
2. **Backend Separation**: Keep database backends as separate packages
3. **Testing Focus**: Maintain comprehensive test coverage
4. **Documentation**: Keep implementation choices well-documented

The foundation is solid, and the remaining work involves completing the real execution implementation, improving thread safety, and adding advanced workflow features.
