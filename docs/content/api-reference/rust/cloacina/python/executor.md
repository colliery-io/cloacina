# cloacina::python::executor <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Python task executor trait and error types.

The [`PythonTaskExecutor`] trait abstracts over the PyO3 bridge so
that cloacina core does not depend on `pyo3`. The `cloaca-backend`
crate provides the concrete implementation.

## Structs

### `cloacina::python::executor::PythonTaskResult`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Result of executing a Python task.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `String` | Task ID that was executed. |
| `output_json` | `String` | JSON-serialized output from the Python function. |



## Enums

### `cloacina::python::executor::PythonExecutionError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Errors that can occur during Python task execution.

#### Variants

- **`EnvironmentSetup`** - Failed to set up the Python environment (sys.path, imports).
- **`TaskNotFound`** - The requested task was not found in the entry module.
- **`TaskException`** - The task function raised an exception.
- **`SerializationError`** - Context serialization/deserialization failed.
- **`ImportError`** - Import error — likely a missing vendored dependency.
- **`RuntimeUnavailable`** - The GIL could not be acquired (e.g., Python not initialized).
