---
title: "cloacina-python::workflow"
description: "API reference of the Rust module cloacina-python::workflow."
editUrl: false
sidebar:
  order: 27
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Structs

### <span id="struct.PyWorkflowBuilder"></span><code style="font-size: 0.85em">class WorkflowBuilder</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.WorkflowBuilder`](../../../cloaca/#class.WorkflowBuilder)

Python wrapper for WorkflowBuilder

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::WorkflowBuilder` |  |
| `context` | `PyWorkflowContext` |  |

#### Methods

##### <span id="PyWorkflowBuilder.new"></span><code>new</code>

```rust
fn new(
    name: &str,
    tenant: Option<&str>,
    package: Option<&str>,
    workflow: Option<&str>,
) -> Self
```

> **Python API**: [`cloaca.WorkflowBuilder.new`](../../../cloaca/#WorkflowBuilder.new)

Create a new WorkflowBuilder with namespace context

<details>
<summary>Source</summary>

```rust
    pub fn new(
        name: &str,
        tenant: Option<&str>,
        package: Option<&str>,
        workflow: Option<&str>,
    ) -> Self {
        let context = PyWorkflowContext::new(
            tenant.unwrap_or("public"),
            package.unwrap_or("embedded"),
            workflow.unwrap_or(name),
        );

        let (tenant_id, _package_name, _workflow_id) = context.as_components();
        let workflow_builder = cloacina::Workflow::builder(name).tenant(tenant_id);

        PyWorkflowBuilder {
            inner: workflow_builder,
            context,
        }
    }
```

</details>

##### <span id="PyWorkflowBuilder.description"></span><code>description</code>

```rust
fn description(&mut self, description: &str)
```

> **Python API**: [`cloaca.WorkflowBuilder.description`](../../../cloaca/#WorkflowBuilder.description)

Set the workflow description

<details>
<summary>Source</summary>

```rust
    pub fn description(&mut self, description: &str) {
        self.inner = self.inner.clone().description(description);
    }
```

</details>

##### <span id="PyWorkflowBuilder.tag"></span><code>tag</code>

```rust
fn tag(&mut self, key: &str, value: &str)
```

> **Python API**: [`cloaca.WorkflowBuilder.tag`](../../../cloaca/#WorkflowBuilder.tag)

Add a tag to the workflow

<details>
<summary>Source</summary>

```rust
    pub fn tag(&mut self, key: &str, value: &str) {
        self.inner = self.inner.clone().tag(key, value);
    }
```

</details>

##### <span id="PyWorkflowBuilder.add_task"></span><code>add_​task</code>

```rust
fn add_task(&mut self, py: Python, task: PyObject) -> PyResult<()>
```

> **Python API**: [`cloaca.WorkflowBuilder.add_task`](../../../cloaca/#WorkflowBuilder.add_task)

Add a task to the workflow by ID or function reference

<details>
<summary>Source</summary>

```rust
    pub fn add_task(&mut self, py: Python, task: PyObject) -> PyResult<()> {
        match task.extract::<String>(py) {
            Ok(task_id) => {
                let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
                    PyValueError::new_err("WorkflowBuilder.add_task called outside a Runtime scope")
                })?;

                let (tenant_id, package_name, workflow_id) = self.context.as_components();
                let task_namespace =
                    cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);

                let task_instance = rt.get_task(&task_namespace).ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "Task '{}' not found in registry. Make sure it was decorated with @task.",
                        task_id
                    ))
                })?;

                self.inner = self
                    .inner
                    .clone()
                    .add_task(task_instance)
                    .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;

                Ok(())
            }
            _ => match task.bind(py).hasattr("__name__") {
                Ok(true) => match task.getattr(py, "__name__") {
                    Ok(name_obj) => match name_obj.extract::<String>(py) {
                        Ok(func_name) => {
                            let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
                                PyValueError::new_err(
                                    "WorkflowBuilder.add_task called outside a Runtime scope",
                                )
                            })?;

                            let (tenant_id, package_name, workflow_id) =
                                self.context.as_components();
                            let task_namespace = cloacina::TaskNamespace::new(
                                tenant_id,
                                package_name,
                                workflow_id,
                                &func_name,
                            );

                            let task_instance = rt.get_task(&task_namespace).ok_or_else(|| {
                                        PyValueError::new_err(format!(
                                            "Task '{}' not found in registry. Make sure it was decorated with @task.",
                                            func_name
                                        ))
                                    })?;

                            self.inner =
                                self.inner.clone().add_task(task_instance).map_err(|e| {
                                    PyValueError::new_err(format!("Failed to add task: {}", e))
                                })?;

                            Ok(())
                        }
                        Err(e) => Err(PyValueError::new_err(format!(
                            "Function has __name__ but it's not a string: {}",
                            e
                        ))),
                    },
                    Err(e) => Err(PyValueError::new_err(format!(
                        "Failed to get __name__ from function: {}",
                        e
                    ))),
                },
                Ok(false) => Err(PyValueError::new_err(
                    "Task must be either a string task ID or a function object with __name__ attribute",
                )),
                Err(e) => Err(PyValueError::new_err(format!(
                    "Failed to check if object has __name__ attribute: {}",
                    e
                ))),
            },
        }
    }
```

</details>

##### <span id="PyWorkflowBuilder.build"></span><code>build</code>

```rust
fn build(&self) -> PyResult<PyWorkflow>
```

> **Python API**: [`cloaca.WorkflowBuilder.build`](../../../cloaca/#WorkflowBuilder.build)

Build the workflow

<details>
<summary>Source</summary>

```rust
    pub fn build(&self) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .clone()
            .build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build workflow: {}", e)))?;
        Ok(PyWorkflow { inner: workflow })
    }
```

</details>

##### <span id="PyWorkflowBuilder.__enter__"></span><code>_​_​enter_​_​</code>

```rust
fn __enter__(slf: PyRef<Self>) -> PyRef<Self>
```

> **Python API**: [`cloaca.WorkflowBuilder.__enter__`](../../../cloaca/#WorkflowBuilder.__enter__)

Context manager entry - establish workflow context for task decorators

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_workflow_context(slf.context.clone());
        slf
    }
```

</details>

##### <span id="PyWorkflowBuilder.__exit__"></span><code>_​_​exit_​_​</code>

```rust
fn __exit__(
    &mut self,
    _py: Python,
    _exc_type: Option<&Bound<PyAny>>,
    _exc_value: Option<&Bound<PyAny>>,
    _traceback: Option<&Bound<PyAny>>,
) -> PyResult<bool>
```

> **Python API**: [`cloaca.WorkflowBuilder.__exit__`](../../../cloaca/#WorkflowBuilder.__exit__)

Context manager exit - clean up context and build workflow

<details>
<summary>Source</summary>

```rust
    pub fn __exit__(
        &mut self,
        _py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        pop_workflow_context();

        let (tenant_id, package_name, workflow_id) = self.context.as_components();

        let mut workflow = cloacina::Workflow::new(workflow_id);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Preserve description and tags set on the builder during the `with` block
        if let Some(desc) = self.inner.get_description() {
            workflow.set_description(desc);
        }
        for (key, value) in self.inner.get_tags() {
            workflow.add_tag(key, value);
        }

        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err("WorkflowBuilder.__exit__ called outside a Runtime scope")
        })?;
        for namespace in rt.task_namespaces() {
            if namespace.tenant_id == tenant_id
                && namespace.package_name == package_name
                && namespace.workflow_id == workflow_id
            {
                if let Some(task_instance) = rt.get_task(&namespace) {
                    workflow
                        .add_task(task_instance)
                        .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;
                }
            }
        }

        workflow
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))?;
        let final_workflow = workflow.finalize();

        let workflow_name = final_workflow.name().to_string();
        rt.register_workflow(workflow_name, move || final_workflow.clone());

        Ok(false)
    }
```

</details>

##### <span id="PyWorkflowBuilder.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.WorkflowBuilder.__repr__`](../../../cloaca/#WorkflowBuilder.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!("WorkflowBuilder(name='{}')", self.inner.name())
    }
```

</details>

### <span id="struct.PyWorkflow"></span><code style="font-size: 0.85em">class Workflow</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.Workflow`](../../../cloaca/#class.Workflow)

Python wrapper for Workflow

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::Workflow` |  |

#### Methods

##### <span id="PyWorkflow.name"></span><code>name</code>

```rust
fn name(&self) -> &str
```

> **Python API**: [`cloaca.Workflow.name`](../../../cloaca/#Workflow.name)

Get workflow name

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        self.inner.name()
    }
```

</details>

##### <span id="PyWorkflow.description"></span><code>description</code>

```rust
fn description(&self) -> String
```

> **Python API**: [`cloaca.Workflow.description`](../../../cloaca/#Workflow.description)

Get workflow description

<details>
<summary>Source</summary>

```rust
    pub fn description(&self) -> String {
        self.inner
            .metadata()
            .description
            .clone()
            .unwrap_or_default()
    }
```

</details>

##### <span id="PyWorkflow.version"></span><code>version</code>

```rust
fn version(&self) -> &str
```

> **Python API**: [`cloaca.Workflow.version`](../../../cloaca/#Workflow.version)

Get workflow version

<details>
<summary>Source</summary>

```rust
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }
```

</details>

##### <span id="PyWorkflow.topological_sort"></span><code>topological_​sort</code>

```rust
fn topological_sort(&self) -> PyResult<Vec<String>>
```

> **Python API**: [`cloaca.Workflow.topological_sort`](../../../cloaca/#Workflow.topological_sort)

Get topological sort of tasks

<details>
<summary>Source</summary>

```rust
    pub fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner
            .topological_sort()
            .map(|namespaces| namespaces.into_iter().map(|ns| ns.to_string()).collect())
            .map_err(|e| PyValueError::new_err(format!("Failed to sort tasks: {}", e)))
    }
```

</details>

##### <span id="PyWorkflow.get_execution_levels"></span><code>get_​execution_​levels</code>

```rust
fn get_execution_levels(&self) -> PyResult<Vec<Vec<String>>>
```

> **Python API**: [`cloaca.Workflow.get_execution_levels`](../../../cloaca/#Workflow.get_execution_levels)

Get execution levels (tasks that can run in parallel)

<details>
<summary>Source</summary>

```rust
    pub fn get_execution_levels(&self) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .get_execution_levels()
            .map(|levels| {
                levels
                    .into_iter()
                    .map(|level| level.into_iter().map(|ns| ns.to_string()).collect())
                    .collect()
            })
            .map_err(|e| PyValueError::new_err(format!("Failed to get execution levels: {}", e)))
    }
```

</details>

##### <span id="PyWorkflow.get_roots"></span><code>get_​roots</code>

```rust
fn get_roots(&self) -> Vec<String>
```

> **Python API**: [`cloaca.Workflow.get_roots`](../../../cloaca/#Workflow.get_roots)

Get root tasks (no dependencies)

<details>
<summary>Source</summary>

```rust
    pub fn get_roots(&self) -> Vec<String> {
        self.inner
            .get_roots()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }
```

</details>

##### <span id="PyWorkflow.get_leaves"></span><code>get_​leaves</code>

```rust
fn get_leaves(&self) -> Vec<String>
```

> **Python API**: [`cloaca.Workflow.get_leaves`](../../../cloaca/#Workflow.get_leaves)

Get leaf tasks (no dependents)

<details>
<summary>Source</summary>

```rust
    pub fn get_leaves(&self) -> Vec<String> {
        self.inner
            .get_leaves()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }
```

</details>

##### <span id="PyWorkflow.validate"></span><code>validate</code>

```rust
fn validate(&self) -> PyResult<()>
```

> **Python API**: [`cloaca.Workflow.validate`](../../../cloaca/#Workflow.validate)

Validate the workflow

<details>
<summary>Source</summary>

```rust
    pub fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))
    }
```

</details>

##### <span id="PyWorkflow.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.Workflow.__repr__`](../../../cloaca/#Workflow.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "Workflow(name='{}', tasks={})",
            self.inner.name(),
            self.inner.get_task_ids().len()
        )
    }
```

</details>

### <span id="struct.PyWorkflowParamsDecorator"></span><code style="font-size: 0.85em">class WorkflowParamsDecorator</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.WorkflowParamsDecorator`](../../../cloaca/#class.WorkflowParamsDecorator)

Pass-through decorator returned by [`workflow_params`](../../../rust/cloacina-python/workflow/#fn.workflow_params). CLOACI-T-0760: the declaration is parsed from source by the compiler at build time, so at runtime this just returns the decorated object unchanged. It exists only so that importing a package which uses `@cloaca.workflow_params(...)` doesn't fail (the reconciler imports the module via PyO3).

#### Methods

##### <span id="PyWorkflowParamsDecorator.__call__"></span><code>_​_​call_​_​</code>

```rust
fn __call__(&self, obj: PyObject) -> PyObject
```

> **Python API**: [`cloaca.WorkflowParamsDecorator.__call__`](../../../cloaca/#WorkflowParamsDecorator.__call__)

<details>
<summary>Source</summary>

```rust
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>

### <span id="struct.PyWorkflowSecretsDecorator"></span><code style="font-size: 0.85em">class WorkflowSecretsDecorator</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.WorkflowSecretsDecorator`](../../../cloaca/#class.WorkflowSecretsDecorator)

Pass-through decorator returned by [`workflow_secrets`](../../../rust/cloacina-python/workflow/#fn.workflow_secrets). CLOACI-T-0859: like [`PyWorkflowParamsDecorator`](../../../rust/cloacina-python/workflow/#struct.PyWorkflowParamsDecorator), the declaration is parsed from source by the compiler at build time into encrypted `InputSlot`s; at runtime it is a no-op pass-through so importing a package that uses `@cloaca.workflow_secrets(...)` doesn't fail.

#### Methods

##### <span id="PyWorkflowSecretsDecorator.__call__"></span><code>_​_​call_​_​</code>

```rust
fn __call__(&self, obj: PyObject) -> PyObject
```

> **Python API**: [`cloaca.WorkflowSecretsDecorator.__call__`](../../../cloaca/#WorkflowSecretsDecorator.__call__)

<details>
<summary>Source</summary>

```rust
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>

### <span id="struct.PyBoundarySchemaDecorator"></span><code style="font-size: 0.85em">class BoundarySchemaDecorator</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

Pass-through decorator returned by [`boundary_schema`](../../../rust/cloacina-python/workflow/#fn.boundary_schema). CLOACI-T-0770: like [`PyWorkflowParamsDecorator`](../../../rust/cloacina-python/workflow/#struct.PyWorkflowParamsDecorator), the declaration is parsed from source by the compiler at build time; at runtime this just returns the decorated object.

#### Methods

##### <span id="PyBoundarySchemaDecorator.__call__"></span><code>_​_​call_​_​</code>

```rust
fn __call__(&self, obj: PyObject) -> PyObject
```

<details>
<summary>Source</summary>

```rust
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>

## Functions

### <span id="fn.py_register_workflow"></span><code style="font-size: 0.85em">fn py_​register_​workflow</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.register_workflow_constructor`](../../../cloaca/#fn.register_workflow_constructor)

```rust
fn py_register_workflow(
    name: String) -> PyResult<(,
    constructor: PyObject) -> PyResult<(,
) -> PyResult<()>
```

Register a workflow constructor function on the thread-local Runtime scope. Exported to Python as `register_workflow_constructor` for backwards compatibility; the Rust symbol name is `py_register_workflow` so it does not collide with the (now deleted) `cloacina::register_workflow_constructor`.

<details>
<summary>Source</summary>

```rust
pub fn py_register_workflow(name: String, constructor: PyObject) -> PyResult<()> {
    Python::with_gil(|py| {
        let workflow_obj = constructor.call0(py).map_err(|e| {
            PyValueError::new_err(format!("Failed to call workflow constructor: {}", e))
        })?;

        let py_workflow: PyWorkflow = workflow_obj.extract(py).map_err(|e| {
            PyValueError::new_err(format!(
                "Failed to extract workflow from constructor: {}",
                e
            ))
        })?;

        let workflow = py_workflow.inner.clone();
        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err("register_workflow_constructor called outside a Runtime scope")
        })?;
        rt.register_workflow(name, move || workflow.clone());

        Ok(())
    })
}
```

</details>

### <span id="fn.workflow_params"></span><code style="font-size: 0.85em">fn workflow_​params</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.workflow_params`](../../../cloaca/#fn.workflow_params)

```rust
fn workflow_params(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyWorkflowParamsDecorator
```

`@cloaca.workflow_params(name=type, name=(type, default), ...)` — declares a packaged Python workflow's typed input params (CLOACI-I-0128 / T-0760).

Parity with Rust's `#[workflow(params(...))]`. The authoritative extraction
happens at build time (the compiler parses this decorator from source into
JSON-Schema-typed `InputSlot`s); at runtime it is a no-op pass-through so the
decorated function/workflow is unaffected. Supported scalar types: `str`,
`int`, `float`, `bool`, `list`, `dict`.

<details>
<summary>Source</summary>

```rust
pub fn workflow_params(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyWorkflowParamsDecorator {
    PyWorkflowParamsDecorator
}
```

</details>

### <span id="fn.workflow_secrets"></span><code style="font-size: 0.85em">fn workflow_​secrets</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.workflow_secrets`](../../../cloaca/#fn.workflow_secrets)

```rust
fn workflow_secrets(
    _args: Bound<'_, pyo3::types::PyTuple>,
) -> PyWorkflowSecretsDecorator
```

`@cloaca.workflow_secrets("db_prod", "api_token", ...)` — declares the SECRETS a packaged Python workflow requires (CLOACI-I-0133 / T-0859), the parity of Rust's `#[workflow(secrets(...))]`. Each name becomes a required encrypted `InputSlot` (`encrypted: true`) carried in the package manifest alongside the declared params; a secret is bound per-instance via a `{"$secret": "name"}` reference and resolved encrypted at fire time (its value never enters the durable `Context`). The authoritative extraction happens at build time (the compiler parses this decorator from source); at runtime it is a no-op pass-through.

<details>
<summary>Source</summary>

```rust
pub fn workflow_secrets(_args: Bound<'_, pyo3::types::PyTuple>) -> PyWorkflowSecretsDecorator {
    PyWorkflowSecretsDecorator
}
```

</details>

### <span id="fn.boundary_schema"></span><code style="font-size: 0.85em">fn boundary_​schema</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.boundary_schema`](../../../cloaca/#fn.boundary_schema)

```rust
fn boundary_schema(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyBoundarySchemaDecorator
```

`@cloaca.boundary_schema(field=type, ...)` — declares the typed shape of a Python accumulator's boundary (CLOACI-T-0770), the parity of deriving `schemars::JsonSchema` on a Rust boundary type. The compiler parses this from source at build time into the accumulator's declared input-interface slot (so inject/fire-with render typed forms); at runtime it's a no-op pass-through. Applied to the accumulator function:

```python
@cloaca.boundary_schema(bid=float, ask=float)
@cloaca.passthrough_accumulator
def py_alpha(event): ...
```

Supported scalar types: `str`, `int`, `float`, `bool`, `list`, `dict`.

<details>
<summary>Source</summary>

```rust
pub fn boundary_schema(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyBoundarySchemaDecorator {
    PyBoundarySchemaDecorator
}
```

</details>
