# cloaca.python.workflow <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.workflow.WorkflowBuilder`

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder](../../rust/cloacina/python/workflow.md#class-workflowbuilder)

Python wrapper for WorkflowBuilder

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(name: str, tenant: Optional[str], package: Optional[str], workflow: Optional[str]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::new](../../rust/cloacina/python/workflow.md#new)

Create a new WorkflowBuilder with namespace context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `tenant` | `Optional[str]` |  |
| `package` | `Optional[str]` |  |
| `workflow` | `Optional[str]` |  |


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
        let workflow_builder = crate::Workflow::builder(name).tenant(tenant_id);

        PyWorkflowBuilder {
            inner: workflow_builder,
            context,
        }
    }
```

</details>



##### `description`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">description</span>(description: str)</code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::description](../../rust/cloacina/python/workflow.md#description)

Set the workflow description

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `description` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn description(&mut self, description: &str) {
        self.inner = self.inner.clone().description(description);
    }
```

</details>



##### `tag`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">tag</span>(key: str, value: str)</code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::tag](../../rust/cloacina/python/workflow.md#tag)

Add a tag to the workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn tag(&mut self, key: &str, value: &str) {
        self.inner = self.inner.clone().tag(key, value);
    }
```

</details>



##### `add_task`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">add_task</span>(task: Any) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::add_task](../../rust/cloacina/python/workflow.md#add_task)

Add a task to the workflow by ID or function reference

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `Any` |  |


<details>
<summary>Source</summary>

```python
    pub fn add_task(&mut self, py: Python, task: PyObject) -> PyResult<()> {
        if let Ok(task_id) = task.extract::<String>(py) {
            let registry = crate::task::global_task_registry();

            let (tenant_id, package_name, workflow_id) = self.context.as_components();
            let task_namespace =
                crate::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);
            let guard = registry.read();

            let constructor = guard.get(&task_namespace).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Task '{}' not found in registry. Make sure it was decorated with @task.",
                    task_id
                ))
            })?;

            let task_instance = constructor();

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
                                    let registry = crate::task::global_task_registry();

                                    let (tenant_id, package_name, workflow_id) = self.context.as_components();
                                    let task_namespace = crate::TaskNamespace::new(tenant_id, package_name, workflow_id, &func_name);
                                    let guard = registry.read();

                                    let constructor = guard.get(&task_namespace).ok_or_else(|| {
                                        PyValueError::new_err(format!(
                                            "Task '{}' not found in registry. Make sure it was decorated with @task.",
                                            func_name
                                        ))
                                    })?;

                                    let task_instance = constructor();

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



##### `build`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">build</span>() -> <span style="color: var(--md-default-fg-color--light);">PyWorkflow</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::build](../../rust/cloacina/python/workflow.md#build)

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



##### `__enter__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__enter__</span>(slf: PyRef&lt;Self&gt;) -> <span style="color: var(--md-default-fg-color--light);">PyRef&lt;Self&gt;</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::__enter__](../../rust/cloacina/python/workflow.md#__enter__)

Context manager entry - establish workflow context for task decorators

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef<Self>` |  |


<details>
<summary>Source</summary>

```python
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_workflow_context(slf.context.clone());
        slf
    }
```

</details>



##### `__exit__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__exit__</span>(_py: Python, _exc_type: Optional[Any], _exc_value: Optional[Any], _traceback: Optional[Any]) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::__exit__](../../rust/cloacina/python/workflow.md#__exit__)

Context manager exit - clean up context and build workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_py` | `Python` |  |
| `_exc_type` | `Optional[Any]` |  |
| `_exc_value` | `Optional[Any]` |  |
| `_traceback` | `Optional[Any]` |  |


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

        let mut workflow = crate::Workflow::new(workflow_id);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Preserve description and tags set on the builder during the `with` block
        if let Some(desc) = self.inner.get_description() {
            workflow.set_description(desc);
        }
        for (key, value) in self.inner.get_tags() {
            workflow.add_tag(key, value);
        }

        let registry = crate::task::global_task_registry();
        let guard = registry.read();

        for (namespace, constructor) in guard.iter() {
            if namespace.tenant_id == tenant_id
                && namespace.package_name == package_name
                && namespace.workflow_id == workflow_id
            {
                let task_instance = constructor();
                workflow
                    .add_task(task_instance)
                    .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;
            }
        }

        drop(guard);

        workflow
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))?;
        let final_workflow = workflow.finalize();

        let workflow_name = final_workflow.name().to_string();
        crate::workflow::register_workflow_constructor(workflow_name, move || {
            final_workflow.clone()
        });

        Ok(false)
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflowBuilder::__repr__](../../rust/cloacina/python/workflow.md#__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!("WorkflowBuilder(name='{}')", self.inner.name())
    }
```

</details>





### `cloaca.python.workflow.Workflow`

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow](../../rust/cloacina/python/workflow.md#class-workflow)

Python wrapper for Workflow

#### Methods

##### `name`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">name</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::name](../../rust/cloacina/python/workflow.md#name)

Get workflow name

<details>
<summary>Source</summary>

```python
    pub fn name(&self) -> &str {
        self.inner.name()
    }
```

</details>



##### `description`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">description</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::description](../../rust/cloacina/python/workflow.md#description)

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



##### `version`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">version</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::version](../../rust/cloacina/python/workflow.md#version)

Get workflow version

<details>
<summary>Source</summary>

```python
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }
```

</details>



##### `topological_sort`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">topological_sort</span>() -> <span style="color: var(--md-default-fg-color--light);">List[str]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::topological_sort](../../rust/cloacina/python/workflow.md#topological_sort)

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



##### `get_execution_levels`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_execution_levels</span>() -> <span style="color: var(--md-default-fg-color--light);">List[List[str]]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::get_execution_levels](../../rust/cloacina/python/workflow.md#get_execution_levels)

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



##### `get_roots`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_roots</span>() -> <span style="color: var(--md-default-fg-color--light);">List[str]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::get_roots](../../rust/cloacina/python/workflow.md#get_roots)

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



##### `get_leaves`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get_leaves</span>() -> <span style="color: var(--md-default-fg-color--light);">List[str]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::get_leaves](../../rust/cloacina/python/workflow.md#get_leaves)

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



##### `validate`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">validate</span>() -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::validate](../../rust/cloacina/python/workflow.md#validate)

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



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::PyWorkflow::__repr__](../../rust/cloacina/python/workflow.md#__repr__)

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





## Functions

### `cloaca.python.workflow.register_workflow_constructor`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">register_workflow_constructor</span>(name: str, constructor: Any) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow::register_workflow_constructor](../../rust/cloacina/python/workflow.md#fn-register_workflow_constructor)

Register a workflow constructor function

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `constructor` | `Any` |  |


<details>
<summary>Source</summary>

```python
pub fn register_workflow_constructor(name: String, constructor: PyObject) -> PyResult<()> {
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
        crate::workflow::register_workflow_constructor(name, move || workflow.clone());

        Ok(())
    })
}
```

</details>
