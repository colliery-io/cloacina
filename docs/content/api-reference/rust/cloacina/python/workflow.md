# cloacina::python::workflow <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::workflow::WorkflowBuilder`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.workflow.WorkflowBuilder](../../../cloaca/python/workflow.md#class-workflowbuilder)

Python wrapper for WorkflowBuilder

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: WorkflowBuilder` |  |
| `context` | `PyWorkflowContext` |  |

#### Methods

##### `new`

```rust
fn new (name : & str , tenant : Option < & str > , package : Option < & str > , workflow : Option < & str > ,) -> Self
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.new](../../../cloaca/python/workflow.md#new)

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
        let workflow_builder = crate::Workflow::builder(name).tenant(tenant_id);

        PyWorkflowBuilder {
            inner: workflow_builder,
            context,
        }
    }
```

</details>



##### `description`

```rust
fn description (& mut self , description : & str)
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.description](../../../cloaca/python/workflow.md#description)

Set the workflow description

<details>
<summary>Source</summary>

```rust
    pub fn description(&mut self, description: &str) {
        self.inner = self.inner.clone().description(description);
    }
```

</details>



##### `tag`

```rust
fn tag (& mut self , key : & str , value : & str)
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.tag](../../../cloaca/python/workflow.md#tag)

Add a tag to the workflow

<details>
<summary>Source</summary>

```rust
    pub fn tag(&mut self, key: &str, value: &str) {
        self.inner = self.inner.clone().tag(key, value);
    }
```

</details>



##### `add_task`

```rust
fn add_task (& mut self , py : Python , task : PyObject) -> PyResult < () >
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.add_task](../../../cloaca/python/workflow.md#add_task)

Add a task to the workflow by ID or function reference

<details>
<summary>Source</summary>

```rust
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

```rust
fn build (& self) -> PyResult < PyWorkflow >
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.build](../../../cloaca/python/workflow.md#build)

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



##### `__enter__`

```rust
fn __enter__ (slf : PyRef < Self >) -> PyRef < Self >
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.__enter__](../../../cloaca/python/workflow.md#__enter__)

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



##### `__exit__`

```rust
fn __exit__ (& mut self , _py : Python , _exc_type : Option < & Bound < PyAny > > , _exc_value : Option < & Bound < PyAny > > , _traceback : Option < & Bound < PyAny > > ,) -> PyResult < bool >
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.__exit__](../../../cloaca/python/workflow.md#__exit__)

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

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.workflow.WorkflowBuilder.__repr__](../../../cloaca/python/workflow.md#__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!("WorkflowBuilder(name='{}')", self.inner.name())
    }
```

</details>





### `cloacina::python::workflow::Workflow`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.workflow.Workflow](../../../cloaca/python/workflow.md#class-workflow)

Python wrapper for Workflow

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: Workflow` |  |

#### Methods

##### `name`

```rust
fn name (& self) -> & str
```

> **Python API**: [cloaca.python.workflow.Workflow.name](../../../cloaca/python/workflow.md#name)

Get workflow name

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        self.inner.name()
    }
```

</details>



##### `description`

```rust
fn description (& self) -> String
```

> **Python API**: [cloaca.python.workflow.Workflow.description](../../../cloaca/python/workflow.md#description)

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



##### `version`

```rust
fn version (& self) -> & str
```

> **Python API**: [cloaca.python.workflow.Workflow.version](../../../cloaca/python/workflow.md#version)

Get workflow version

<details>
<summary>Source</summary>

```rust
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }
```

</details>



##### `topological_sort`

```rust
fn topological_sort (& self) -> PyResult < Vec < String > >
```

> **Python API**: [cloaca.python.workflow.Workflow.topological_sort](../../../cloaca/python/workflow.md#topological_sort)

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



##### `get_execution_levels`

```rust
fn get_execution_levels (& self) -> PyResult < Vec < Vec < String > > >
```

> **Python API**: [cloaca.python.workflow.Workflow.get_execution_levels](../../../cloaca/python/workflow.md#get_execution_levels)

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



##### `get_roots`

```rust
fn get_roots (& self) -> Vec < String >
```

> **Python API**: [cloaca.python.workflow.Workflow.get_roots](../../../cloaca/python/workflow.md#get_roots)

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



##### `get_leaves`

```rust
fn get_leaves (& self) -> Vec < String >
```

> **Python API**: [cloaca.python.workflow.Workflow.get_leaves](../../../cloaca/python/workflow.md#get_leaves)

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



##### `validate`

```rust
fn validate (& self) -> PyResult < () >
```

> **Python API**: [cloaca.python.workflow.Workflow.validate](../../../cloaca/python/workflow.md#validate)

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



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.workflow.Workflow.__repr__](../../../cloaca/python/workflow.md#__repr__)

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





## Functions

### `cloacina::python::workflow::register_workflow_constructor`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.workflow.register_workflow_constructor](../../../cloaca/python/workflow.md#register_workflow_constructor)

```rust
fn register_workflow_constructor (name : String , constructor : PyObject) -> PyResult < () >
```

Register a workflow constructor function

<details>
<summary>Source</summary>

```rust
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
