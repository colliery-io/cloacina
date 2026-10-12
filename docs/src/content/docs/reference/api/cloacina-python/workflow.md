---
title: "cloacina-python.workflow"
description: "API reference of the Python module cloacina-python.workflow."
sidebar:
  order: 15
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.WorkflowBuilder"></span>`class WorkflowBuilder`

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder](../../rust/cloacina-python/workflow/#struct.PyWorkflowBuilder)

Python wrapper for WorkflowBuilder

#### Methods

##### <span id="WorkflowBuilder.new"></span>`new`

```python
def new(
    name:  str,
    tenant: Option < & str >,
    package: Option < & str >,
    workflow: Option < & str >,
) -> Self
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::new](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.new)

Create a new WorkflowBuilder with namespace context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | ` str` |  |
| `tenant` | `Option < & str >` |  |
| `package` | `Option < & str >` |  |
| `workflow` | `Option < & str >` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="WorkflowBuilder.description"></span>`description`

```python
def description(description:  str)
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::description](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.description)

Set the workflow description

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `description` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn description(&mut self, description: &str) {
        self.inner = self.inner.clone().description(description);
    }
```

</details>



##### <span id="WorkflowBuilder.tag"></span>`tag`

```python
def tag(key:  str, value:  str)
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::tag](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.tag)

Add a tag to the workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |
| `value` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn tag(&mut self, key: &str, value: &str) {
        self.inner = self.inner.clone().tag(key, value);
    }
```

</details>



##### <span id="WorkflowBuilder.add_task"></span>`add_task`

```python
def add_task(task: PyObject) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::add_task](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.add_task)

Add a task to the workflow by ID or function reference

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `PyObject` |  |


<details>
<summary>Source</summary>

```python
    pub fn add_task(&mut self, py: Python, task: PyObject) -> PyResult<()> {
        if let Ok(task_id) = task.extract::<String>(py) {
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
        } else {
            match task.bind(py).hasattr("__name__") {
                Ok(true) => {
                    match task.getattr(py, "__name__") {
                        Ok(name_obj) => {
                            match name_obj.extract::<String>(py) {
                                Ok(func_name) => {
                                    let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
                                        PyValueError::new_err(
                                            "WorkflowBuilder.add_task called outside a Runtime scope",
                                        )
                                    })?;

                                    let (tenant_id, package_name, workflow_id) = self.context.as_components();
                                    let task_namespace = cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &func_name);

                                    let task_instance = rt.get_task(&task_namespace).ok_or_else(|| {
                                        PyValueError::new_err(format!(
                                            "Task '{}' not found in registry. Make sure it was decorated with @task.",
                                            func_name
                                        ))
                                    })?;

                                    self.inner = self.inner.clone().add_task(task_instance)
                                        .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;

                                    Ok(())
                                },
                                Err(e) => {
                                    Err(PyValueError::new_err(format!(
                                        "Function has __name__ but it's not a string: {}",
                                        e
                                    )))
                                }
                            }
                        },
                        Err(e) => {
                            Err(PyValueError::new_err(format!(
                                "Failed to get __name__ from function: {}",
                                e
                            )))
                        }
                    }
                },
                Ok(false) => {
                    Err(PyValueError::new_err(
                        "Task must be either a string task ID or a function object with __name__ attribute"
                    ))
                },
                Err(e) => {
                    Err(PyValueError::new_err(format!(
                        "Failed to check if object has __name__ attribute: {}",
                        e
                    )))
                }
            }
        }
    }
```

</details>



##### <span id="WorkflowBuilder.build"></span>`build`

```python
def build() -> PyResult < PyWorkflow >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::build](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.build)

Build the workflow

<details>
<summary>Source</summary>

```python
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



##### <span id="WorkflowBuilder.__enter__"></span>`__enter__`

```python
def __enter__(slf: PyRef < Self >) -> PyRef < Self >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::__enter__](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.__enter__)

Context manager entry - establish workflow context for task decorators

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef < Self >` |  |


<details>
<summary>Source</summary>

```python
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_workflow_context(slf.context.clone());
        slf
    }
```

</details>



##### <span id="WorkflowBuilder.__exit__"></span>`__exit__`

```python
def __exit__(
    _py: Python,
    _exc_type: Option < & Bound < PyAny > >,
    _exc_value: Option < & Bound < PyAny > >,
    _traceback: Option < & Bound < PyAny > >,
) -> PyResult < bool >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::__exit__](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.__exit__)

Context manager exit - clean up context and build workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_py` | `Python` |  |
| `_exc_type` | `Option < & Bound < PyAny > >` |  |
| `_exc_value` | `Option < & Bound < PyAny > >` |  |
| `_traceback` | `Option < & Bound < PyAny > >` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="WorkflowBuilder.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowBuilder::__repr__](../../rust/cloacina-python/workflow/#PyWorkflowBuilder.__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!("WorkflowBuilder(name='{}')", self.inner.name())
    }
```

</details>





### <span id="class.Workflow"></span>`class Workflow`

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow](../../rust/cloacina-python/workflow/#struct.PyWorkflow)

Python wrapper for Workflow

#### Methods

##### <span id="Workflow.name"></span>`name`

```python
def name() ->  str
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::name](../../rust/cloacina-python/workflow/#PyWorkflow.name)

Get workflow name

<details>
<summary>Source</summary>

```python
    pub fn name(&self) -> &str {
        self.inner.name()
    }
```

</details>



##### <span id="Workflow.description"></span>`description`

```python
def description() -> str
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::description](../../rust/cloacina-python/workflow/#PyWorkflow.description)

Get workflow description

<details>
<summary>Source</summary>

```python
    pub fn description(&self) -> String {
        self.inner
            .metadata()
            .description
            .clone()
            .unwrap_or_default()
    }
```

</details>



##### <span id="Workflow.version"></span>`version`

```python
def version() ->  str
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::version](../../rust/cloacina-python/workflow/#PyWorkflow.version)

Get workflow version

<details>
<summary>Source</summary>

```python
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }
```

</details>



##### <span id="Workflow.topological_sort"></span>`topological_sort`

```python
def topological_sort() -> PyResult < Vec < String > >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::topological_sort](../../rust/cloacina-python/workflow/#PyWorkflow.topological_sort)

Get topological sort of tasks

<details>
<summary>Source</summary>

```python
    pub fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner
            .topological_sort()
            .map(|namespaces| namespaces.into_iter().map(|ns| ns.to_string()).collect())
            .map_err(|e| PyValueError::new_err(format!("Failed to sort tasks: {}", e)))
    }
```

</details>



##### <span id="Workflow.get_execution_levels"></span>`get_execution_levels`

```python
def get_execution_levels() -> PyResult < Vec < Vec < String > > >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::get_execution_levels](../../rust/cloacina-python/workflow/#PyWorkflow.get_execution_levels)

Get execution levels (tasks that can run in parallel)

<details>
<summary>Source</summary>

```python
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



##### <span id="Workflow.get_roots"></span>`get_roots`

```python
def get_roots() -> Vec < String >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::get_roots](../../rust/cloacina-python/workflow/#PyWorkflow.get_roots)

Get root tasks (no dependencies)

<details>
<summary>Source</summary>

```python
    pub fn get_roots(&self) -> Vec<String> {
        self.inner
            .get_roots()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }
```

</details>



##### <span id="Workflow.get_leaves"></span>`get_leaves`

```python
def get_leaves() -> Vec < String >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::get_leaves](../../rust/cloacina-python/workflow/#PyWorkflow.get_leaves)

Get leaf tasks (no dependents)

<details>
<summary>Source</summary>

```python
    pub fn get_leaves(&self) -> Vec<String> {
        self.inner
            .get_leaves()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }
```

</details>



##### <span id="Workflow.validate"></span>`validate`

```python
def validate() -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::validate](../../rust/cloacina-python/workflow/#PyWorkflow.validate)

Validate the workflow

<details>
<summary>Source</summary>

```python
    pub fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))
    }
```

</details>



##### <span id="Workflow.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflow::__repr__](../../rust/cloacina-python/workflow/#PyWorkflow.__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "Workflow(name='{}', tasks={})",
            self.inner.name(),
            self.inner.get_task_ids().len()
        )
    }
```

</details>





### <span id="class.WorkflowParamsDecorator"></span>`class WorkflowParamsDecorator`

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowParamsDecorator](../../rust/cloacina-python/workflow/#struct.PyWorkflowParamsDecorator)

Pass-through decorator returned by [`workflow_params`]. CLOACI-T-0760: the declaration is parsed from source by the compiler at build time, so at runtime this just returns the decorated object unchanged. It exists only so that importing a package which uses `@cloaca.workflow_params(...)` doesn't fail (the reconciler imports the module via PyO3).

#### Methods

##### <span id="WorkflowParamsDecorator.__call__"></span>`__call__`

```python
def __call__(obj: PyObject) -> PyObject
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowParamsDecorator::__call__](../../rust/cloacina-python/workflow/#PyWorkflowParamsDecorator.__call__)

<details>
<summary>Source</summary>

```python
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>





### <span id="class.WorkflowSecretsDecorator"></span>`class WorkflowSecretsDecorator`

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowSecretsDecorator](../../rust/cloacina-python/workflow/#struct.PyWorkflowSecretsDecorator)

Pass-through decorator returned by [`workflow_secrets`]. CLOACI-T-0859: like [`PyWorkflowParamsDecorator`], the declaration is parsed from source by the compiler at build time into encrypted `InputSlot`s; at runtime it is a no-op pass-through so importing a package that uses `@cloaca.workflow_secrets(...)` doesn't fail.

#### Methods

##### <span id="WorkflowSecretsDecorator.__call__"></span>`__call__`

```python
def __call__(obj: PyObject) -> PyObject
```

> **Rust Implementation**: [cloacina-python::workflow::PyWorkflowSecretsDecorator::__call__](../../rust/cloacina-python/workflow/#PyWorkflowSecretsDecorator.__call__)

<details>
<summary>Source</summary>

```python
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>





### <span id="class.BoundarySchemaDecorator"></span>`class BoundarySchemaDecorator`

> **Rust Implementation**: [cloacina-python::workflow::PyBoundarySchemaDecorator](../../rust/cloacina-python/workflow/#struct.PyBoundarySchemaDecorator)

Pass-through decorator returned by [`boundary_schema`]. CLOACI-T-0770: like [`PyWorkflowParamsDecorator`], the declaration is parsed from source by the compiler at build time; at runtime this just returns the decorated object.

#### Methods

##### <span id="BoundarySchemaDecorator.__call__"></span>`__call__`

```python
def __call__(obj: PyObject) -> PyObject
```

> **Rust Implementation**: [cloacina-python::workflow::PyBoundarySchemaDecorator::__call__](../../rust/cloacina-python/workflow/#PyBoundarySchemaDecorator.__call__)

<details>
<summary>Source</summary>

```python
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>





## Functions

### <span id="fn.register_workflow_constructor"></span>`register_workflow_constructor`

```python
def register_workflow_constructor(name: str, constructor: PyObject) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::workflow::py_register_workflow](../../rust/cloacina-python/workflow/#fn.py_register_workflow)

Register a workflow constructor function on the thread-local Runtime scope. Exported to Python as `register_workflow_constructor` for backwards compatibility; the Rust symbol name is `py_register_workflow` so it does not collide with the (now deleted) `cloacina::register_workflow_constructor`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `constructor` | `PyObject` |  |


<details>
<summary>Source</summary>

```python
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



### <span id="fn.workflow_params"></span>`workflow_params`

```python
def workflow_params(
    _kwargs: Option < Bound < '_ , pyo3 :: types :: PyDict > >,
) -> PyWorkflowParamsDecorator
```

> **Rust Implementation**: [cloacina-python::workflow::workflow_params](../../rust/cloacina-python/workflow/#fn.workflow_params)

`@cloaca.workflow_params(name=type, name=(type, default), ...)` — declares a packaged Python workflow's typed input params (CLOACI-I-0128 / T-0760).

Parity with Rust's `#[workflow(params(...))]`. The authoritative extraction
happens at build time (the compiler parses this decorator from source into
JSON-Schema-typed `InputSlot`s); at runtime it is a no-op pass-through so the
decorated function/workflow is unaffected. Supported scalar types: `str`,
`int`, `float`, `bool`, `list`, `dict`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_kwargs` | `Option < Bound < '_ , pyo3 :: types :: PyDict > >` |  |


<details>
<summary>Source</summary>

```python
pub fn workflow_params(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyWorkflowParamsDecorator {
    PyWorkflowParamsDecorator
}
```

</details>



### <span id="fn.workflow_secrets"></span>`workflow_secrets`

```python
def workflow_secrets(
    _args: Bound < '_ , pyo3 :: types :: PyTuple >,
) -> PyWorkflowSecretsDecorator
```

> **Rust Implementation**: [cloacina-python::workflow::workflow_secrets](../../rust/cloacina-python/workflow/#fn.workflow_secrets)

`@cloaca.workflow_secrets("db_prod", "api_token", ...)` — declares the SECRETS a packaged Python workflow requires (CLOACI-I-0133 / T-0859), the parity of Rust's `#[workflow(secrets(...))]`. Each name becomes a required encrypted `InputSlot` (`encrypted: true`) carried in the package manifest alongside the declared params; a secret is bound per-instance via a `{"$secret": "name"}` reference and resolved encrypted at fire time (its value never enters the durable `Context`). The authoritative extraction happens at build time (the compiler parses this decorator from source); at runtime it is a no-op pass-through.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_args` | `Bound < '_ , pyo3 :: types :: PyTuple >` |  |


<details>
<summary>Source</summary>

```python
pub fn workflow_secrets(_args: Bound<'_, pyo3::types::PyTuple>) -> PyWorkflowSecretsDecorator {
    PyWorkflowSecretsDecorator
}
```

</details>



### <span id="fn.boundary_schema"></span>`boundary_schema`

```python
def boundary_schema(
    _kwargs: Option < Bound < '_ , pyo3 :: types :: PyDict > >,
) -> PyBoundarySchemaDecorator
```

> **Rust Implementation**: [cloacina-python::workflow::boundary_schema](../../rust/cloacina-python/workflow/#fn.boundary_schema)

`@cloaca.boundary_schema(field=type, ...)` — declares the typed shape of a Python accumulator's boundary (CLOACI-T-0770), the parity of deriving `schemars::JsonSchema` on a Rust boundary type. The compiler parses this from source at build time into the accumulator's declared input-interface slot (so inject/fire-with render typed forms); at runtime it's a no-op pass-through. Applied to the accumulator function:

```python
@cloaca.boundary_schema(bid=float, ask=float)
@cloaca.passthrough_accumulator
def py_alpha(event): ...
```
Supported scalar types: `str`, `int`, `float`, `bool`, `list`, `dict`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_kwargs` | `Option < Bound < '_ , pyo3 :: types :: PyDict > >` |  |


<details>
<summary>Source</summary>

```python
pub fn boundary_schema(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyBoundarySchemaDecorator {
    PyBoundarySchemaDecorator
}
```

</details>



