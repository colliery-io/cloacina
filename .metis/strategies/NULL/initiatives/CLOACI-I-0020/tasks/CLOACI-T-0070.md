---
id: python-task-discovery-and-executor
level: task
title: "Python task discovery and executor integration"
short_code: "CLOACI-T-0070"
created_at: 2026-01-28T14:29:03.813112+00:00
updated_at: 2026-01-28T14:29:03.813112+00:00
parent: CLOACI-I-0020
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
strategy_id: NULL
initiative_id: CLOACI-I-0020
---

# Python task discovery and executor integration

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0020]]

## Objective

Implement the Python task execution bridge using PyO3. This includes discovering `@task` decorated functions from the entry module, setting up the Python environment with vendor and source paths, executing tasks with context serialization, and handling async Python tasks from synchronous Rust code.

## Acceptance Criteria

- [ ] PyO3 integration initializes Python with correct sys.path
- [ ] Task discovery finds all `@task` decorated functions in entry module
- [ ] Context serialized to Python dict, modifications returned to Rust
- [ ] Async Python tasks executed via `asyncio.run()` or equivalent
- [ ] Python exceptions converted to `TaskError` with traceback
- [ ] GIL management doesn't deadlock with tokio runtime

## Implementation Notes

### Python Environment Setup

```rust
// crates/cloacina/src/python/environment.rs

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::path::Path;

/// Set up Python environment for package execution
pub fn setup_python_environment(
    package: &ExtractedPythonPackage,
) -> PyResult<()> {
    Python::with_gil(|py| {
        let sys = py.import("sys")?;
        let sys_path: &PyList = sys.getattr("path")?.downcast()?;

        // Add vendor directory first (dependencies take precedence)
        if package.vendor_dir.exists() {
            sys_path.insert(0, package.vendor_dir.to_string_lossy().as_ref())?;
        }

        // Add source directory
        sys_path.insert(0, package.source_dir.to_string_lossy().as_ref())?;

        Ok(())
    })
}

/// Clean up sys.path after execution
pub fn cleanup_python_environment(
    package: &ExtractedPythonPackage,
) -> PyResult<()> {
    Python::with_gil(|py| {
        let sys = py.import("sys")?;
        let sys_path: &PyList = sys.getattr("path")?.downcast()?;

        // Remove our paths
        let vendor_path = package.vendor_dir.to_string_lossy().to_string();
        let source_path = package.source_dir.to_string_lossy().to_string();

        // Filter out our paths
        let new_path: Vec<String> = sys_path
            .iter()
            .filter_map(|p| p.extract::<String>().ok())
            .filter(|p| p != &vendor_path && p != &source_path)
            .collect();

        sys_path.clear();
        for p in new_path {
            sys_path.append(p)?;
        }

        Ok(())
    })
}
```

### Task Discovery

```rust
// crates/cloacina/src/python/discovery.rs

use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Discovered Python task with metadata
#[derive(Debug, Clone)]
pub struct DiscoveredPythonTask {
    pub id: String,
    pub function_name: String,
    pub dependencies: Vec<String>,
    pub is_async: bool,
}

/// Discover all @task decorated functions in the entry module
pub fn discover_tasks(
    entry_module: &str,
) -> PyResult<Vec<DiscoveredPythonTask>> {
    Python::with_gil(|py| {
        // Import the entry module
        let module = py.import(entry_module)?;

        // Get the task registry from cloaca module
        let cloaca = py.import("cloaca")?;
        let registry = cloaca.getattr("_task_registry")?;

        // Get all registered tasks
        let tasks: &PyDict = registry.downcast()?;

        let mut discovered = Vec::new();
        for (task_id, task_info) in tasks.iter() {
            let task_id: String = task_id.extract()?;
            let info: &PyDict = task_info.downcast()?;

            let function_name: String = info
                .get_item("function_name")?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("function_name"))?
                .extract()?;

            let dependencies: Vec<String> = info
                .get_item("dependencies")?
                .map(|d| d.extract().unwrap_or_default())
                .unwrap_or_default();

            let is_async: bool = info
                .get_item("is_async")?
                .map(|a| a.extract().unwrap_or(false))
                .unwrap_or(false);

            discovered.push(DiscoveredPythonTask {
                id: task_id,
                function_name,
                dependencies,
                is_async,
            });
        }

        Ok(discovered)
    })
}
```

### Python Task Decorator (cloaca side)

```python
# python/cloaca/src/cloaca/task.py

import asyncio
import functools
import inspect
from typing import Any, Callable, TypeVar

# Global task registry - populated by @task decorator
_task_registry: dict[str, dict[str, Any]] = {}

F = TypeVar('F', bound=Callable[..., Any])

def task(
    id: str,
    dependencies: list[str] | None = None,
) -> Callable[[F], F]:
    """
    Decorator to mark a function as a Cloacina task.

    Usage:
        @task(id="my-task", dependencies=["other-task"])
        async def my_task(context: dict) -> dict:
            # Task implementation
            return context
    """
    def decorator(func: F) -> F:
        is_async = asyncio.iscoroutinefunction(func)

        _task_registry[id] = {
            "function": func,
            "function_name": func.__name__,
            "dependencies": dependencies or [],
            "is_async": is_async,
        }

        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            return func(*args, **kwargs)

        return wrapper  # type: ignore

    return decorator


def get_task(task_id: str) -> Callable | None:
    """Get a task function by ID."""
    info = _task_registry.get(task_id)
    return info["function"] if info else None


def list_tasks() -> list[str]:
    """List all registered task IDs."""
    return list(_task_registry.keys())
```

### Task Execution Bridge

```rust
// crates/cloacina/src/python/executor.rs

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};
use crate::context::Context;
use crate::error::TaskError;
use chrono::Utc;

/// Execute a Python task and return the modified context
pub async fn execute_python_task(
    package: &ExtractedPythonPackage,
    task_id: &str,
    context: Context<serde_json::Value>,
) -> Result<Context<serde_json::Value>, TaskError> {
    // Clone values for move into blocking task
    let task_id = task_id.to_string();
    let context_json = serde_json::to_string(context.data())
        .map_err(|e| TaskError::ExecutionFailed {
            task_id: task_id.clone(),
            message: format!("Failed to serialize context: {}", e),
            timestamp: Utc::now(),
        })?;

    let source_dir = package.source_dir.clone();
    let vendor_dir = package.vendor_dir.clone();
    let entry_module = package.entry_module.clone();

    // Execute Python in a blocking task to avoid GIL + async issues
    let result = tokio::task::spawn_blocking(move || {
        execute_python_task_sync(
            &source_dir,
            &vendor_dir,
            &entry_module,
            &task_id,
            &context_json,
        )
    })
    .await
    .map_err(|e| TaskError::ExecutionFailed {
        task_id: task_id.clone(),
        message: format!("Python execution panicked: {}", e),
        timestamp: Utc::now(),
    })??;

    // Parse result back into context
    let result_value: serde_json::Value = serde_json::from_str(&result)
        .map_err(|e| TaskError::ExecutionFailed {
            task_id,
            message: format!("Failed to parse Python result: {}", e),
            timestamp: Utc::now(),
        })?;

    // Merge result into original context
    let mut new_context = context;
    if let serde_json::Value::Object(obj) = result_value {
        for (key, value) in obj {
            if new_context.get(&key).is_some() {
                new_context.update(key, value).ok();
            } else {
                new_context.insert(key, value).ok();
            }
        }
    }

    Ok(new_context)
}

fn execute_python_task_sync(
    source_dir: &Path,
    vendor_dir: &Path,
    entry_module: &str,
    task_id: &str,
    context_json: &str,
) -> Result<String, TaskError> {
    Python::with_gil(|py| {
        // Setup sys.path
        let sys = py.import("sys")?;
        let sys_path: &PyList = sys.getattr("path")?.downcast()?;

        if vendor_dir.exists() {
            sys_path.insert(0, vendor_dir.to_string_lossy().as_ref())?;
        }
        sys_path.insert(0, source_dir.to_string_lossy().as_ref())?;

        // Import entry module to trigger task registration
        py.import(entry_module)?;

        // Get the task function
        let cloaca = py.import("cloaca")?;
        let get_task = cloaca.getattr("get_task")?;
        let task_func = get_task.call1((task_id,))?;

        if task_func.is_none() {
            return Err(TaskError::ExecutionFailed {
                task_id: task_id.to_string(),
                message: format!("Task '{}' not found in module '{}'", task_id, entry_module),
                timestamp: Utc::now(),
            });
        }

        // Parse context JSON to Python dict
        let json_module = py.import("json")?;
        let context_dict = json_module
            .getattr("loads")?
            .call1((context_json,))?;

        // Check if task is async
        let inspect = py.import("inspect")?;
        let is_coroutine = inspect
            .getattr("iscoroutinefunction")?
            .call1((task_func,))?
            .is_true()?;

        // Execute the task
        let result = if is_coroutine {
            // Run async task with asyncio
            let asyncio = py.import("asyncio")?;
            let coro = task_func.call1((context_dict,))?;
            asyncio.getattr("run")?.call1((coro,))?
        } else {
            // Run sync task directly
            task_func.call1((context_dict,))?
        };

        // Convert result back to JSON
        let result_json: String = json_module
            .getattr("dumps")?
            .call1((result,))?
            .extract()?;

        Ok(result_json)
    })
    .map_err(|e: PyErr| {
        Python::with_gil(|py| {
            let traceback = e
                .traceback(py)
                .map(|tb| tb.format().unwrap_or_default())
                .unwrap_or_default();

            TaskError::ExecutionFailed {
                task_id: task_id.to_string(),
                message: format!("Python error: {}\n{}", e, traceback),
                timestamp: Utc::now(),
            }
        })
    })
}
```

### GIL Management for Tokio

```rust
// crates/cloacina/src/python/gil.rs

use pyo3::prelude::*;
use std::future::Future;
use tokio::task::JoinHandle;

/// Execute a Python operation without blocking the tokio runtime.
///
/// PyO3's GIL acquisition can block, which is problematic in async context.
/// We use spawn_blocking to move GIL operations to the blocking thread pool.
pub fn run_python<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce(Python<'_>) -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| f(py))
    })
}

/// Wrapper that ensures GIL is released during await points
pub struct GilReleaser;

impl GilReleaser {
    /// Release GIL while awaiting a future, then reacquire
    pub async fn release_during<F, T>(future: F) -> T
    where
        F: Future<Output = T>,
    {
        // In the current architecture, we don't hold the GIL across await
        // because all Python execution is in spawn_blocking
        future.await
    }
}
```

### Error Conversion

```rust
// crates/cloacina/src/python/error.rs

use pyo3::prelude::*;
use pyo3::exceptions::*;
use crate::error::TaskError;
use chrono::Utc;

/// Convert PyErr to TaskError with full traceback
pub fn pyerr_to_task_error(err: PyErr, task_id: &str) -> TaskError {
    Python::with_gil(|py| {
        let error_type = err.get_type(py).name().unwrap_or("UnknownError");
        let error_msg = err.value(py).to_string();

        let traceback = err
            .traceback(py)
            .and_then(|tb| tb.format().ok())
            .unwrap_or_default();

        TaskError::ExecutionFailed {
            task_id: task_id.to_string(),
            message: format!(
                "{}: {}\n\nTraceback:\n{}",
                error_type, error_msg, traceback
            ),
            timestamp: Utc::now(),
        }
    })
}

/// Map common Python exceptions to appropriate error types
pub fn classify_python_error(err: &PyErr, task_id: &str) -> TaskError {
    Python::with_gil(|py| {
        if err.is_instance_of::<PyKeyError>(py) {
            TaskError::ExecutionFailed {
                task_id: task_id.to_string(),
                message: format!("Context key error: {}", err),
                timestamp: Utc::now(),
            }
        } else if err.is_instance_of::<PyTypeError>(py) {
            TaskError::ValidationFailed {
                message: format!("Type error in task {}: {}", task_id, err),
            }
        } else if err.is_instance_of::<PyImportError>(py) {
            TaskError::ExecutionFailed {
                task_id: task_id.to_string(),
                message: format!("Import error (missing dependency?): {}", err),
                timestamp: Utc::now(),
            }
        } else {
            pyerr_to_task_error(err.clone(), task_id)
        }
    })
}
```

### Module Structure

```rust
// crates/cloacina/src/python/mod.rs

mod discovery;
mod environment;
mod error;
mod executor;
mod gil;

pub use discovery::{discover_tasks, DiscoveredPythonTask};
pub use environment::{setup_python_environment, cleanup_python_environment};
pub use error::{pyerr_to_task_error, classify_python_error};
pub use executor::execute_python_task;
pub use gil::{run_python, GilReleaser};
```

### Technical Dependencies

- **T-0069**: Package loader provides `ExtractedPythonPackage`
- **Existing cloaca**: Python `@task` decorator must match discovery expectations

### Risk Considerations

1. **GIL contention**: Heavy Python workloads can starve Rust tasks. Use spawn_blocking pool sizing.
2. **Memory leaks**: Python objects held across GIL boundaries can leak. Ensure cleanup.
3. **Signal handling**: Python's signal handling may conflict with tokio. Test SIGINT behavior.
4. **Thread safety**: Python code may not be thread-safe. Document that tasks should be stateless.
5. **Import caching**: Python caches imports. Module reload between packages may need `importlib.reload()`.

## Status Updates

*To be added during implementation*
