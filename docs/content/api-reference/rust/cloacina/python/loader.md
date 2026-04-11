# cloacina::python::loader <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Python workflow package loader.

Imports a Python workflow module via PyO3, triggering `@task` decorator
registration, then collects the registered tasks and builds the workflow.
This is the bridge between extracted `.cloacina` packages and the
cloacina task execution engine.

## Enums

### `cloacina::python::loader::PythonLoaderError` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Error type for Python package loading operations.

#### Variants

- **`ImportError`**
- **`ValidationError`**
- **`RegistrationError`**
- **`RuntimeError`**



## Functions

### `cloacina::python::loader::ensure_cloaca_module`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn ensure_cloaca_module (py : Python) -> PyResult < () >
```

Ensure the `cloaca` Python module is available in the embedded interpreter.

User workflow code does `from cloaca import task, WorkflowBuilder`.
When running inside the server (no pip-installed cloaca wheel), we inject
a synthetic `cloaca` module that exports the PyO3 types from `cloacina::python`.

<details>
<summary>Source</summary>

```rust
pub fn ensure_cloaca_module(py: Python) -> PyResult<()> {
    let sys_modules = py.import("sys")?.getattr("modules")?;

    // Already registered — nothing to do
    if sys_modules.contains("cloaca")? {
        return Ok(());
    }

    let module = PyModule::new(py, "cloaca")?;

    // Task decorator and handle
    module.add_function(wrap_pyfunction!(super::task::task, &module)?)?;
    module.add_class::<super::task::PyTaskHandle>()?;
    module.add_class::<super::task::TaskDecorator>()?;

    // Context
    module.add_class::<super::context::PyContext>()?;

    // Workflow
    module.add_class::<super::workflow::PyWorkflowBuilder>()?;
    module.add_class::<super::workflow::PyWorkflow>()?;
    module.add_function(wrap_pyfunction!(
        super::workflow::register_workflow_constructor,
        &module
    )?)?;

    // Trigger decorator and result
    module.add_function(wrap_pyfunction!(super::trigger::trigger, &module)?)?;
    module.add_class::<super::trigger::PyTriggerResult>()?;
    module.add_class::<super::trigger::TriggerDecorator>()?;

    // Value objects
    module.add_class::<super::workflow_context::PyWorkflowContext>()?;
    module.add_class::<super::namespace::PyTaskNamespace>()?;

    // Computation graph decorators and builder
    module.add_function(wrap_pyfunction!(
        super::computation_graph::passthrough_accumulator_decorator,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(
        super::computation_graph::stream_accumulator_decorator,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(
        super::computation_graph::polling_accumulator_decorator,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(
        super::computation_graph::batch_accumulator_decorator,
        &module
    )?)?;
    module.add_function(wrap_pyfunction!(super::computation_graph::node, &module)?)?;
    module.add_class::<super::computation_graph::PyComputationGraphBuilder>()?;

    // Variable registry
    module.add_function(wrap_pyfunction!(py_var, &module)?)?;
    module.add_function(wrap_pyfunction!(py_var_or, &module)?)?;

    // Register in sys.modules so `import cloaca` works
    sys_modules.set_item("cloaca", &module)?;

    Ok(())
}
```

</details>



### `cloacina::python::loader::validate_no_stdlib_shadowing`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn validate_no_stdlib_shadowing (workflow_dir : & Path , vendor_dir : & Path ,) -> Result < () , PythonLoaderError >
```

Import a Python workflow module and register its tasks.

This is the core function that bridges extracted Python packages to
the cloacina execution engine:
1. Ensures the `cloaca` module is available in the interpreter
2. Adds workflow and vendor directories to `sys.path`
3. Pushes a workflow context (so `@task` decorators know the namespace)
4. Imports the entry module (triggering decorator registration)
5. Collects registered tasks, builds and validates the workflow
6. Returns the list of registered task namespaces

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_dir` | `-` | — Path to the extracted `workflow/` directory |
| `vendor_dir` | `-` | — Path to the extracted `vendor/` directory |
| `entry_module` | `-` | — Dotted module path (e.g., `"workflow.tasks"`) |
| `package_name` | `-` | — Package name from manifest |
| `tenant_id` | `-` | — Tenant for namespace isolation (default: `"public"`) |


<details>
<summary>Source</summary>

```rust
pub fn validate_no_stdlib_shadowing(
    workflow_dir: &Path,
    vendor_dir: &Path,
) -> Result<(), PythonLoaderError> {
    for dir in [workflow_dir, vendor_dir] {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                // Check for module.py or module/ (package directory)
                let module_name = name_str.strip_suffix(".py").unwrap_or(&name_str);
                if STDLIB_DENY_LIST.contains(&module_name) {
                    return Err(PythonLoaderError::ImportError(format!(
                        "Package contains '{}' which shadows Python stdlib module '{}' — rejected for security",
                        name_str, module_name
                    )));
                }
            }
        }
    }
    Ok(())
}
```

</details>



### `cloacina::python::loader::import_and_register_python_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn import_and_register_python_workflow (workflow_dir : & Path , vendor_dir : & Path , entry_module : & str , package_name : & str , tenant_id : & str ,) -> Result < Vec < TaskNamespace > , PythonLoaderError >
```

<details>
<summary>Source</summary>

```rust
pub fn import_and_register_python_workflow(
    workflow_dir: &Path,
    vendor_dir: &Path,
    entry_module: &str,
    package_name: &str,
    tenant_id: &str,
) -> Result<Vec<TaskNamespace>, PythonLoaderError> {
    // Default: use package_name as workflow_name
    import_and_register_python_workflow_named(
        workflow_dir,
        vendor_dir,
        entry_module,
        package_name,
        package_name,
        tenant_id,
    )
}
```

</details>



### `cloacina::python::loader::import_and_register_python_workflow_named`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn import_and_register_python_workflow_named (workflow_dir : & Path , vendor_dir : & Path , entry_module : & str , package_name : & str , workflow_name : & str , tenant_id : & str ,) -> Result < Vec < TaskNamespace > , PythonLoaderError >
```

<details>
<summary>Source</summary>

```rust
pub fn import_and_register_python_workflow_named(
    workflow_dir: &Path,
    vendor_dir: &Path,
    entry_module: &str,
    package_name: &str,
    workflow_name: &str,
    tenant_id: &str,
) -> Result<Vec<TaskNamespace>, PythonLoaderError> {
    // SECURITY: Check for stdlib shadowing before importing
    validate_no_stdlib_shadowing(workflow_dir, vendor_dir)?;

    let workflow_dir = workflow_dir.to_path_buf();
    let vendor_dir = vendor_dir.to_path_buf();
    let entry_module = entry_module.to_string();
    let package_name = package_name.to_string();
    let workflow_name = workflow_name.to_string();
    let tenant_id = tenant_id.to_string();
    let timeout = Duration::from_secs(IMPORT_TIMEOUT_SECS);

    // PyO3 operations must happen on a thread that can acquire the GIL.
    // Wrap in a timeout to catch infinite loops during import.
    let handle = std::thread::spawn(move || -> Result<Vec<TaskNamespace>, PythonLoaderError> {
        Python::with_gil(|py| {
            // 1. Ensure cloaca module is available
            ensure_cloaca_module(py)?;

            // 2. Add paths to sys.path (append, not insert — avoid shadowing stdlib)
            let sys = py.import("sys")?;
            let path = sys.getattr("path")?;
            path.call_method1(
                "append",
                (workflow_dir
                    .to_str()
                    .ok_or(PythonLoaderError::RuntimeError(
                        "Invalid workflow_dir path".to_string(),
                    ))?,),
            )?;
            if vendor_dir.exists() {
                path.call_method1(
                    "append",
                    (vendor_dir.to_str().ok_or(PythonLoaderError::RuntimeError(
                        "Invalid vendor_dir path".to_string(),
                    ))?,),
                )?;
            }

            // 3. Push workflow context for @task decorators
            let context = PyWorkflowContext::new(&tenant_id, &package_name, &workflow_name);
            push_workflow_context(context.clone());

            // 4. Import entry module — @task decorators fire, tasks registered
            let import_result = py.import(entry_module.as_str());
            if let Err(e) = import_result {
                pop_workflow_context();
                return Err(PythonLoaderError::ImportError(format!(
                    "Failed to import '{}': {}",
                    entry_module, e
                )));
            }

            // 5. Pop context
            pop_workflow_context();

            // 5b. Drain any Python triggers registered via @cloaca.trigger decorators
            //     during the module import, wrap them, and register in the global trigger registry.
            let python_triggers = crate::python::trigger::drain_python_triggers();
            for trigger_def in python_triggers {
                let trigger_name = trigger_def.name.clone();
                let wrapper = std::sync::Arc::new(
                    crate::python::trigger::PythonTriggerWrapper::new(&trigger_def),
                );
                // The constructor returns the same Arc'd wrapper each time — the
                // PythonTriggerWrapper is Send+Sync and holds a PyObject that's
                // accessed only under the GIL.
                let wrapper_for_closure = wrapper.clone();
                crate::trigger::register_trigger_constructor(trigger_name.clone(), move || {
                    wrapper_for_closure.clone()
                });
                tracing::info!("Registered Python trigger: {}", trigger_name);
            }

            // 6. Collect registered tasks and build workflow
            let (t, p, w) = context.as_components();

            let registry = crate::task::global_task_registry();
            let guard = registry.read();

            let mut namespaces = Vec::new();
            let mut workflow = crate::Workflow::new(w);
            workflow.set_tenant(t);
            workflow.set_package(p);

            for (namespace, constructor) in guard.iter() {
                if namespace.tenant_id == t
                    && namespace.package_name == p
                    && namespace.workflow_id == w
                {
                    namespaces.push(namespace.clone());
                    let task_instance = constructor();
                    workflow.add_task(task_instance).map_err(|e| {
                        PythonLoaderError::RegistrationError(format!("Failed to add task: {}", e))
                    })?;
                }
            }
            drop(guard);

            if namespaces.is_empty() {
                return Err(PythonLoaderError::RegistrationError(format!(
                    "No tasks registered after importing '{}'. Ensure the module uses @cloaca.task decorators.",
                    entry_module
                )));
            }

            // 7. Validate and register workflow
            workflow.validate().map_err(|e| {
                PythonLoaderError::ValidationError(format!("Workflow validation failed: {}", e))
            })?;
            let final_workflow = workflow.finalize();

            let workflow_name = final_workflow.name().to_string();
            crate::workflow::register_workflow_constructor(workflow_name, move || {
                final_workflow.clone()
            });

            tracing::info!(
                "Python workflow imported: {} tasks registered for {}::{}::{}",
                namespaces.len(),
                t,
                p,
                w
            );

            Ok(namespaces)
        })
    });

    // Wait with timeout — catches infinite loops during import
    let start = std::time::Instant::now();
    loop {
        if handle.is_finished() {
            let result = handle.join().map_err(|_| {
                PythonLoaderError::RuntimeError("Python import thread panicked".to_string())
            })??;
            return Ok(result);
        }
        if start.elapsed() > timeout {
            return Err(PythonLoaderError::RuntimeError(format!(
                "Python workflow import timed out after {}s",
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
```

</details>



### `cloacina::python::loader::import_python_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn import_python_computation_graph (workflow_dir : & Path , vendor_dir : & Path , entry_module : & str , graph_name : & str ,) -> Result < String , PythonLoaderError >
```

Import a Python computation graph module and return the graph name.

The module is expected to use `ComputationGraphBuilder` + `@node` decorators.
On `__exit__`, the builder registers the graph executor in `GRAPH_EXECUTORS`.
This function imports the module (triggering registration) and returns the
registered graph name so the reconciler can retrieve and wrap the executor.

<details>
<summary>Source</summary>

```rust
pub fn import_python_computation_graph(
    workflow_dir: &Path,
    vendor_dir: &Path,
    entry_module: &str,
    graph_name: &str,
) -> Result<String, PythonLoaderError> {
    validate_no_stdlib_shadowing(workflow_dir, vendor_dir)?;

    let workflow_dir = workflow_dir.to_path_buf();
    let vendor_dir = vendor_dir.to_path_buf();
    let entry_module = entry_module.to_string();
    let graph_name = graph_name.to_string();
    let timeout = Duration::from_secs(IMPORT_TIMEOUT_SECS);

    let handle = std::thread::spawn(move || -> Result<String, PythonLoaderError> {
        Python::with_gil(|py| {
            ensure_cloaca_module(py)?;

            let sys = py.import("sys")?;
            let path = sys.getattr("path")?;
            path.call_method1(
                "append",
                (workflow_dir
                    .to_str()
                    .ok_or(PythonLoaderError::RuntimeError(
                        "Invalid workflow_dir path".to_string(),
                    ))?,),
            )?;
            if vendor_dir.exists() {
                path.call_method1(
                    "append",
                    (vendor_dir.to_str().ok_or(PythonLoaderError::RuntimeError(
                        "Invalid vendor_dir path".to_string(),
                    ))?,),
                )?;
            }

            // Import the module — ComputationGraphBuilder.__exit__ registers the executor
            py.import(entry_module.as_str()).map_err(|e| {
                PythonLoaderError::ImportError(format!(
                    "Failed to import computation graph module '{}': {}",
                    entry_module, e
                ))
            })?;

            // Verify the graph was registered
            let executor = crate::python::computation_graph::get_graph_executor(&graph_name);
            if executor.is_none() {
                return Err(PythonLoaderError::RegistrationError(format!(
                    "Computation graph '{}' was not registered after importing '{}'. \
                     Ensure the module uses ComputationGraphBuilder with matching graph name.",
                    graph_name, entry_module
                )));
            }

            tracing::info!("Python computation graph imported: '{}'", graph_name);

            Ok(graph_name)
        })
    });

    let start = std::time::Instant::now();
    loop {
        if handle.is_finished() {
            let result = handle.join().map_err(|_| {
                PythonLoaderError::RuntimeError("Python CG import thread panicked".to_string())
            })??;
            return Ok(result);
        }
        if start.elapsed() > timeout {
            return Err(PythonLoaderError::RuntimeError(format!(
                "Python CG import timed out after {}s",
                timeout.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
```

</details>



### `cloacina::python::loader::py_var`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.loader.py_var](../../../cloaca/python/loader.md#py_var)

```rust
fn py_var (name : & str) -> PyResult < String >
```

Python binding: `cloaca.var(name)` — resolve a `CLOACINA_VAR_{NAME}` env var.

<details>
<summary>Source</summary>

```rust
fn py_var(name: &str) -> PyResult<String> {
    crate::var(name).map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
}
```

</details>



### `cloacina::python::loader::py_var_or`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.loader.py_var_or](../../../cloaca/python/loader.md#py_var_or)

```rust
fn py_var_or (name : & str , default : & str) -> String
```

Python binding: `cloaca.var_or(name, default)` — resolve with a fallback.

<details>
<summary>Source</summary>

```rust
fn py_var_or(name: &str, default: &str) -> String {
    crate::var_or(name, default)
}
```

</details>
