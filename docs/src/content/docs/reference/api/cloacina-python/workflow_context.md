---
title: "cloacina-python.workflow_context"
description: "API reference of the Python module cloacina-python.workflow_context."
sidebar:
  order: 16
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.WorkflowContext"></span>`class WorkflowContext`

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext](../../rust/cloacina-python/workflow_context/#struct.PyWorkflowContext)

WorkflowContext provides namespace management for Python workflows

#### Methods

##### <span id="WorkflowContext.new"></span>`new`

```python
def new(tenant_id:  str, package_name:  str, workflow_id:  str) -> Self
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::new](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.new)

Create a new WorkflowContext

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | ` str` |  |
| `package_name` | ` str` |  |
| `workflow_id` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
        }
    }
```

</details>



##### <span id="WorkflowContext.tenant_id"></span>`tenant_id`

```python
def tenant_id() ->  str
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::tenant_id](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```python
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
```

</details>



##### <span id="WorkflowContext.package_name"></span>`package_name`

```python
def package_name() ->  str
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::package_name](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.package_name)

Get package name

<details>
<summary>Source</summary>

```python
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
```

</details>



##### <span id="WorkflowContext.workflow_id"></span>`workflow_id`

```python
def workflow_id() ->  str
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::workflow_id](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```python
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
```

</details>



##### <span id="WorkflowContext.task_namespace"></span>`task_namespace`

```python
def task_namespace(task_id:  str) -> PyTaskNamespace
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::task_namespace](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.task_namespace)

Generate a TaskNamespace for a task within this workflow context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_id` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn task_namespace(&self, task_id: &str) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(cloacina::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            task_id,
        ))
    }
```

</details>



##### <span id="WorkflowContext.resolve_dependency"></span>`resolve_dependency`

```python
def resolve_dependency(task_name:  str) -> PyTaskNamespace
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::resolve_dependency](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.resolve_dependency)

Resolve a dependency task name to a full TaskNamespace within this context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_name` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn resolve_dependency(&self, task_name: &str) -> PyTaskNamespace {
        self.task_namespace(task_name)
    }
```

</details>



##### <span id="WorkflowContext.workflow_namespace"></span>`workflow_namespace`

```python
def workflow_namespace() -> PyTaskNamespace
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::workflow_namespace](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.workflow_namespace)

Get the workflow namespace (without task_id)

<details>
<summary>Source</summary>

```python
    pub fn workflow_namespace(&self) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(cloacina::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            "",
        ))
    }
```

</details>



##### <span id="WorkflowContext.contains_namespace"></span>`contains_namespace`

```python
def contains_namespace(namespace:  PyTaskNamespace) -> bool
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::contains_namespace](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.contains_namespace)

Check if a namespace belongs to this workflow context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | ` PyTaskNamespace` |  |


<details>
<summary>Source</summary>

```python
    pub fn contains_namespace(&self, namespace: &PyTaskNamespace) -> bool {
        namespace.tenant_id() == self.tenant_id
            && namespace.package_name() == self.package_name
            && namespace.workflow_id() == self.workflow_id
    }
```

</details>



##### <span id="WorkflowContext.__str__"></span>`__str__`

```python
def __str__() -> str
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::__str__](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.__str__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __str__(&self) -> String {
        format!(
            "{}::{}::{}",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }
```

</details>



##### <span id="WorkflowContext.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::__repr__](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.__repr__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowContext('{}', '{}', '{}')",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }
```

</details>



##### <span id="WorkflowContext.__eq__"></span>`__eq__`

```python
def __eq__(other:  PyWorkflowContext) -> bool
```

> **Rust Implementation**: [cloacina-python::workflow_context::PyWorkflowContext::__eq__](../../rust/cloacina-python/workflow_context/#PyWorkflowContext.__eq__)

Equality comparison

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | ` PyWorkflowContext` |  |


<details>
<summary>Source</summary>

```python
    pub fn __eq__(&self, other: &PyWorkflowContext) -> bool {
        self.tenant_id == other.tenant_id
            && self.package_name == other.package_name
            && self.workflow_id == other.workflow_id
    }
```

</details>





