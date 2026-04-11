# cloaca.python.workflow_context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.workflow_context.WorkflowContext`

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext](../../rust/cloacina/python/workflow_context.md#class-workflowcontext)

WorkflowContext provides namespace management for Python workflows

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(tenant_id: str, package_name: str, workflow_id: str) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::new](../../rust/cloacina/python/workflow_context.md#new)

Create a new WorkflowContext

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `str` |  |
| `package_name` | `str` |  |
| `workflow_id` | `str` |  |


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



##### `tenant_id`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">tenant_id</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::tenant_id](../../rust/cloacina/python/workflow_context.md#tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```python
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
```

</details>



##### `package_name`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">package_name</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::package_name](../../rust/cloacina/python/workflow_context.md#package_name)

Get package name

<details>
<summary>Source</summary>

```python
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
```

</details>



##### `workflow_id`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">workflow_id</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::workflow_id](../../rust/cloacina/python/workflow_context.md#workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```python
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
```

</details>



##### `task_namespace`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">task_namespace</span>(task_id: str) -> <span style="color: var(--md-default-fg-color--light);">PyTaskNamespace</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::task_namespace](../../rust/cloacina/python/workflow_context.md#task_namespace)

Generate a TaskNamespace for a task within this workflow context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn task_namespace(&self, task_id: &str) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(crate::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            task_id,
        ))
    }
```

</details>



##### `resolve_dependency`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">resolve_dependency</span>(task_name: str) -> <span style="color: var(--md-default-fg-color--light);">PyTaskNamespace</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::resolve_dependency](../../rust/cloacina/python/workflow_context.md#resolve_dependency)

Resolve a dependency task name to a full TaskNamespace within this context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_name` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn resolve_dependency(&self, task_name: &str) -> PyTaskNamespace {
        self.task_namespace(task_name)
    }
```

</details>



##### `workflow_namespace`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">workflow_namespace</span>() -> <span style="color: var(--md-default-fg-color--light);">PyTaskNamespace</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::workflow_namespace](../../rust/cloacina/python/workflow_context.md#workflow_namespace)

Get the workflow namespace (without task_id)

<details>
<summary>Source</summary>

```python
    pub fn workflow_namespace(&self) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(crate::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            "",
        ))
    }
```

</details>



##### `contains_namespace`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">contains_namespace</span>(namespace: PyTaskNamespace) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::contains_namespace](../../rust/cloacina/python/workflow_context.md#contains_namespace)

Check if a namespace belongs to this workflow context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `PyTaskNamespace` |  |


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



##### `__str__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__str__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::__str__](../../rust/cloacina/python/workflow_context.md#__str__)

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



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::__repr__](../../rust/cloacina/python/workflow_context.md#__repr__)

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



##### `__eq__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__eq__</span>(other: PyWorkflowContext) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::workflow_context::PyWorkflowContext::__eq__](../../rust/cloacina/python/workflow_context.md#__eq__)

Equality comparison

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `PyWorkflowContext` |  |


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
