---
title: "cloacina-python::loader"
description: "Python workflow package loader."
sidebar:
  order: 33
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python workflow package loader.

Imports a Python workflow module via PyO3, triggering `@task` decorator
registration, then collects the registered tasks and builds the workflow.
This is the bridge between extracted `.cloacina` packages and the
cloacina task execution engine.

## Enums

### <span id="enum.PythonLoaderError"></span>`enum PythonLoaderError`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Error type for Python package loading operations.

#### Variants

- **`ImportError`**
- **`ValidationError`**
- **`RegistrationError`**
- **`RuntimeError`**



## Functions

### <span id="fn.provider_scope_of_caller"></span>`fn provider_scope_of_caller`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn provider_scope_of_caller () -> Option < cloacina :: registry :: loader :: ProviderScope >
```

The provider scope installed on the CALLING thread (CLOACI-T-0925).

The reconciler installs it around a package load — it says which staged
`providers/` tree this package's `cloaca.constructor(from = ..)` calls resolve
against. Imports run on a thread we spawn, and a thread-local does not follow
a thread hop, so we capture it here and re-install it there — the same
treatment `ScopedRuntime` and `ScopedRegistration` already get.

<details>
<summary>Source</summary>

```rust
fn provider_scope_of_caller() -> Option<cloacina::registry::loader::ProviderScope> {
    cloacina::registry::loader::current_provider_scope()
}
```

</details>



### <span id="fn.enter_provider_scope"></span>`fn enter_provider_scope`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn enter_provider_scope (scope : Option < cloacina :: registry :: loader :: ProviderScope > ,) -> cloacina :: registry :: loader :: ScopedProviderSearch
```

Re-install a captured provider scope on this thread for the import's duration. `None` (embedded / direct import / tests) leaves resolution on the process-wide path, exactly as before CLOACI-T-0925.

<details>
<summary>Source</summary>

```rust
fn enter_provider_scope(
    scope: Option<cloacina::registry::loader::ProviderScope>,
) -> cloacina::registry::loader::ScopedProviderSearch {
    cloacina::registry::loader::ScopedProviderSearch::enter_opt(scope)
}
```

</details>



### <span id="fn.ensure_cloaca_module"></span>`fn ensure_cloaca_module`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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

    // The `cloaca` authorship contract — the single source of truth lives in
    // `crate::register_authoring` (the maturin wheel `#[pymodule]` calls the
    // exact same fn), so wheel/server symbol drift is structurally impossible
    // (CLOACI-I-0137). Host-only symbols (runner/admin) are wheel-only by
    // design — the server IS the runner, so its `cloaca` is authorship-only.
    crate::register_authoring(&module)?;

    // Register in sys.modules so `import cloaca` works
    sys_modules.set_item("cloaca", &module)?;

    Ok(())
}
```

</details>



### <span id="fn.validate_no_stdlib_shadowing"></span>`fn validate_no_stdlib_shadowing`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



### <span id="fn.import_and_register_python_workflow"></span>`fn import_and_register_python_workflow`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn import_and_register_python_workflow (workflow_dir : & Path , vendor_dir : & Path , entry_module : & str , package_name : & str , tenant_id : & str , runtime : Arc < Runtime > ,) -> Result < Vec < TaskNamespace > , PythonLoaderError >
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
    runtime: Arc<Runtime>,
) -> Result<Vec<TaskNamespace>, PythonLoaderError> {
    // Default: use package_name as workflow_name
    import_and_register_python_workflow_named(
        workflow_dir,
        vendor_dir,
        entry_module,
        package_name,
        package_name,
        tenant_id,
        runtime,
    )
}
```

</details>



### <span id="fn.import_and_register_python_workflow_named"></span>`fn import_and_register_python_workflow_named`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn import_and_register_python_workflow_named (workflow_dir : & Path , vendor_dir : & Path , entry_module : & str , package_name : & str , workflow_name : & str , tenant_id : & str , runtime : Arc < Runtime > ,) -> Result < Vec < TaskNamespace > , PythonLoaderError >
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
    runtime: Arc<Runtime>,
) -> Result<Vec<TaskNamespace>, PythonLoaderError> {
    // A wedged interpreter cannot run another import — fail fast instead of
    // parking a thread on a GIL that will never be released (CLOACI-T-0919).
    crate::import_guard::ensure_python_runtime_usable(package_name)?;

    // SECURITY: Check for stdlib shadowing before importing
    validate_no_stdlib_shadowing(workflow_dir, vendor_dir)?;

    let workflow_dir = workflow_dir.to_path_buf();
    let vendor_dir = vendor_dir.to_path_buf();
    let entry_module = entry_module.to_string();
    let package_name = package_name.to_string();
    let workflow_name = workflow_name.to_string();
    let tenant_id = tenant_id.to_string();
    let timeout = import_timeout();

    // CLOACI-T-0919: the supervisor needs the import thread's Python thread id
    // so it can inject an async exception if module-scope code never returns.
    let ident = ImportThreadIdent::new();
    let thread_ident = ident.clone();
    let supervise_label = package_name.clone();

    // PyO3 operations must happen on a thread that can acquire the GIL.
    // Wrap in a timeout to catch infinite loops during import.
    // CLOACI-T-0925: the loader installs the package's provider scope on the
    // calling thread; capture it here so the import thread we are about to spawn
    // resolves `cloaca.constructor(from = ..)` against THIS package's staged
    // providers (a thread-local does not follow a thread hop).
    let provider_scope = provider_scope_of_caller();

    let handle = std::thread::spawn(move || -> Result<Vec<TaskNamespace>, PythonLoaderError> {
        // Install the scoped runtime on this worker thread so every
        // @task/@trigger/@workflow_builder registration triggered by the
        // import below resolves it via `current_runtime()`.
        let _scope =
            ScopedRuntime::new(runtime.clone()).map_err(PythonLoaderError::RuntimeError)?;
        // CLOACI-T-0921: stamp every registration this import triggers with the
        // owning tenant + package.
        let _registration = crate::registration_scope::ScopedRegistration::for_package(
            Some(tenant_id.as_str()),
            Some(package_name.as_str()),
        );
        // CLOACI-T-0925: whose providers this import resolves against.
        let _provider_scope = enter_provider_scope(provider_scope);
        Python::with_gil(|py| {
            // 0. Publish this thread's Python ident BEFORE any user code runs,
            //    so a module-scope hang is interruptible (CLOACI-T-0919).
            thread_ident.record(py);

            // 1. Ensure cloaca module is available
            ensure_cloaca_module(py)?;

            // 2. Add paths to sys.path (append, not insert — avoid shadowing stdlib)
            let sys = py.import("sys")?;
            let path = sys.getattr("path")?;

            // CLOACI-T-0840: a PREVIOUS version of this package may have
            // imported the same module tree in this interpreter — a bare
            // re-import would be a sys.modules no-op and register NOTHING
            // (the decorators never re-run), silently loading an empty
            // workflow. Evict the package's module subtree and prune stale
            // sys.path roots that can still resolve it, so THIS version's
            // source actually executes. In-flight tasks from the old version
            // are unaffected — their Arc'd PyObjects stay alive independent
            // of sys.modules.
            let top_level = entry_module
                .split('.')
                .next()
                .unwrap_or(entry_module.as_str())
                .to_string();
            {
                let modules = sys.getattr("modules")?;
                let keys: Vec<String> = modules
                    .call_method0("keys")?
                    .try_iter()?
                    .filter_map(|k| k.ok().and_then(|k| k.extract::<String>().ok()))
                    .collect();
                let prefix = format!("{top_level}.");
                for key in keys {
                    if key == top_level || key.starts_with(&prefix) {
                        tracing::debug!(
                            module = %key,
                            "evicting cached module before package (re)import (CLOACI-T-0840)"
                        );
                        modules.call_method1("__delitem__", (key,))?;
                    }
                }

                // Prune sys.path roots (other than this load's dirs) that can
                // resolve the top-level module — without this, an OLD staging
                // dir earlier in sys.path would win the fresh import and load
                // the previous version's source.
                let entries: Vec<String> = path
                    .try_iter()?
                    .filter_map(|e| e.ok().and_then(|e| e.extract::<String>().ok()))
                    .collect();
                for entry in entries {
                    let entry_path = std::path::Path::new(&entry);
                    if entry_path == workflow_dir.as_path() || entry_path == vendor_dir.as_path() {
                        continue;
                    }
                    let resolves = entry_path.join(&top_level).join("__init__.py").exists()
                        || entry_path.join(format!("{top_level}.py")).exists();
                    if resolves {
                        tracing::debug!(
                            path = %entry,
                            "pruning stale sys.path root for package (CLOACI-T-0840)"
                        );
                        path.call_method1("remove", (entry,))?;
                    }
                }
            }

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
            //     during the module import, wrap them, and register into the scoped Runtime.
            let rt = current_runtime().ok_or_else(|| {
                PythonLoaderError::RuntimeError(
                    "No Runtime installed for Python workflow import".to_string(),
                )
            })?;
            let python_triggers = crate::trigger::drain_python_triggers();
            for trigger_def in python_triggers {
                let trigger_name = trigger_def.name.clone();
                let wrapper =
                    std::sync::Arc::new(crate::trigger::PythonTriggerWrapper::new(&trigger_def));
                // The constructor returns the same Arc'd wrapper each time — the
                // PythonTriggerWrapper is Send+Sync and holds a PyObject that's
                // accessed only under the GIL.
                let wrapper_for_closure = wrapper.clone();
                rt.register_trigger(trigger_name.clone(), move || wrapper_for_closure.clone());
                tracing::info!("Registered Python trigger: {}", trigger_name);
            }

            // 6. Collect registered tasks (from the scoped Runtime) and build workflow.
            let (t, p, w) = context.as_components();

            let mut namespaces = Vec::new();
            let mut workflow = cloacina::Workflow::new(w);
            workflow.set_tenant(t);
            workflow.set_package(p);

            // Scan the Runtime's task namespaces for matches. We don't have a
            // filter API yet, so enumerate through known namespace prefixes by
            // probing the task registry via a lightweight iteration helper.
            for namespace in rt.task_namespaces() {
                if namespace.tenant_id == t
                    && namespace.package_name == p
                    && namespace.workflow_id == w
                {
                    if let Some(task_instance) = rt.get_task(&namespace) {
                        namespaces.push(namespace.clone());
                        workflow.add_task(task_instance).map_err(|e| {
                            PythonLoaderError::RegistrationError(format!(
                                "Failed to add task: {}",
                                e
                            ))
                        })?;
                    }
                }
            }

            // T-0545 M3a: a "reactor library" module declares only
            // `@cloaca.reactor` classes — no tasks. Allow that as long as
            // *something* was registered (tasks OR reactors), since the
            // reconciler will dispatch the reactors into the scheduler
            // post-import.
            if namespaces.is_empty() && rt.reactor_names().is_empty() {
                return Err(PythonLoaderError::RegistrationError(format!(
                    "Empty package: importing '{}' registered no tasks and no reactors. \
                     Ensure the module uses @cloaca.task or @cloaca.reactor decorators.",
                    entry_module
                )));
            }

            // 7. Validate and register workflow — only if tasks were
            //    registered. A reactor-only "library" package skips the
            //    workflow registration entirely (workflow validation
            //    rejects empty workflows).
            if !namespaces.is_empty() {
                workflow.validate().map_err(|e| {
                    PythonLoaderError::ValidationError(format!("Workflow validation failed: {}", e))
                })?;
                let final_workflow = workflow.finalize();
                let workflow_name = final_workflow.name().to_string();
                rt.register_workflow(workflow_name, move || final_workflow.clone());
                tracing::info!(
                    "Python workflow imported: {} tasks registered for {}::{}::{}",
                    namespaces.len(),
                    t,
                    p,
                    w
                );
            } else {
                tracing::info!(
                    "Python reactor-library package imported: {} reactors, no workflow tasks ({}::{}::{})",
                    rt.reactor_names().len(),
                    t,
                    p,
                    w
                );
            }

            Ok(namespaces)
        })
    });

    // Wait with timeout — and, on timeout, escalate through the interrupt →
    // surface ladder instead of abandoning a GIL-holding thread (T-0919).
    supervise_import(&supervise_label, handle, ident, timeout)
}
```

</details>



### <span id="fn.import_python_computation_graph"></span>`fn import_python_computation_graph`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn import_python_computation_graph (workflow_dir : & Path , vendor_dir : & Path , entry_module : & str , graph_name : & str , tenant_id : Option < & str > , package_name : Option < & str > , runtime : Arc < Runtime > ,) -> Result < String , PythonLoaderError >
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
    // CLOACI-T-0921: whose import this is. Stamped onto every `@graph` /
    // `@*_accumulator` registration the import triggers, so two tenants'
    // same-named graphs no longer share one executor slot.
    tenant_id: Option<&str>,
    package_name: Option<&str>,
    runtime: Arc<Runtime>,
) -> Result<String, PythonLoaderError> {
    crate::import_guard::ensure_python_runtime_usable(graph_name)?;
    validate_no_stdlib_shadowing(workflow_dir, vendor_dir)?;

    let workflow_dir = workflow_dir.to_path_buf();
    let vendor_dir = vendor_dir.to_path_buf();
    let entry_module = entry_module.to_string();
    let graph_name = graph_name.to_string();
    // CLOACI-T-0921: whose import this is, stamped onto every registration.
    let registration_scope =
        crate::registration_scope::RegistrationScope::new(tenant_id, package_name);
    let timeout = import_timeout();

    // CLOACI-T-0919: same import-hang ladder as the workflow path.
    let ident = ImportThreadIdent::new();
    let thread_ident = ident.clone();
    let supervise_label = graph_name.clone();

    // CLOACI-T-0925: carry the caller's provider scope onto the import thread.
    let provider_scope = provider_scope_of_caller();

    let handle = std::thread::spawn(move || -> Result<String, PythonLoaderError> {
        let _scope =
            ScopedRuntime::new(runtime.clone()).map_err(PythonLoaderError::RuntimeError)?;
        let _registration = crate::registration_scope::ScopedRegistration::new(registration_scope);
        let _provider_scope = enter_provider_scope(provider_scope);
        Python::with_gil(|py| {
            thread_ident.record(py);
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
            let executor = crate::computation_graph::get_graph_executor(&graph_name);
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

    supervise_import(&supervise_label, handle, ident, timeout)
}
```

</details>



### <span id="fn.py_var"></span>`fn py_var`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.loader.py_var](../../../cloacina-python/loader/#fn.py_var)

```rust
fn py_var (name : & str) -> PyResult < String >
```

Python binding: `cloaca.var(name)` — resolve a `CLOACINA_VAR_{NAME}` env var.

<details>
<summary>Source</summary>

```rust
pub(crate) fn py_var(name: &str) -> PyResult<String> {
    cloacina::var(name).map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
}
```

</details>



### <span id="fn.py_var_or"></span>`fn py_var_or`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.loader.py_var_or](../../../cloacina-python/loader/#fn.py_var_or)

```rust
fn py_var_or (name : & str , default : & str) -> String
```

Python binding: `cloaca.var_or(name, default)` — resolve with a fallback.

<details>
<summary>Source</summary>

```rust
pub(crate) fn py_var_or(name: &str, default: &str) -> String {
    cloacina::var_or(name, default)
}
```

</details>



