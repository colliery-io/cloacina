# cloacina::python::workflow_context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::workflow_context::WorkflowContext`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.workflow_context.WorkflowContext](../../../cloaca/python/workflow_context.md#class-workflowcontext)

WorkflowContext provides namespace management for Python workflows

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `String` |  |
| `package_name` | `String` |  |
| `workflow_id` | `String` |  |

#### Methods

##### `new`

```rust
fn new (tenant_id : & str , package_name : & str , workflow_id : & str) -> Self
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.new](../../../cloaca/python/workflow_context.md#new)

Create a new WorkflowContext

<details>
<summary>Source</summary>

```rust
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

```rust
fn tenant_id (& self) -> & str
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.tenant_id](../../../cloaca/python/workflow_context.md#tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
```

</details>



##### `package_name`

```rust
fn package_name (& self) -> & str
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.package_name](../../../cloaca/python/workflow_context.md#package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
```

</details>



##### `workflow_id`

```rust
fn workflow_id (& self) -> & str
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.workflow_id](../../../cloaca/python/workflow_context.md#workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
```

</details>



##### `task_namespace`

```rust
fn task_namespace (& self , task_id : & str) -> PyTaskNamespace
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.task_namespace](../../../cloaca/python/workflow_context.md#task_namespace)

Generate a TaskNamespace for a task within this workflow context

<details>
<summary>Source</summary>

```rust
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

```rust
fn resolve_dependency (& self , task_name : & str) -> PyTaskNamespace
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.resolve_dependency](../../../cloaca/python/workflow_context.md#resolve_dependency)

Resolve a dependency task name to a full TaskNamespace within this context

<details>
<summary>Source</summary>

```rust
    pub fn resolve_dependency(&self, task_name: &str) -> PyTaskNamespace {
        self.task_namespace(task_name)
    }
```

</details>



##### `workflow_namespace`

```rust
fn workflow_namespace (& self) -> PyTaskNamespace
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.workflow_namespace](../../../cloaca/python/workflow_context.md#workflow_namespace)

Get the workflow namespace (without task_id)

<details>
<summary>Source</summary>

```rust
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

```rust
fn contains_namespace (& self , namespace : & PyTaskNamespace) -> bool
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.contains_namespace](../../../cloaca/python/workflow_context.md#contains_namespace)

Check if a namespace belongs to this workflow context

<details>
<summary>Source</summary>

```rust
    pub fn contains_namespace(&self, namespace: &PyTaskNamespace) -> bool {
        namespace.tenant_id() == self.tenant_id
            && namespace.package_name() == self.package_name
            && namespace.workflow_id() == self.workflow_id
    }
```

</details>



##### `__str__`

```rust
fn __str__ (& self) -> String
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.__str__](../../../cloaca/python/workflow_context.md#__str__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __str__(&self) -> String {
        format!(
            "{}::{}::{}",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.__repr__](../../../cloaca/python/workflow_context.md#__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowContext('{}', '{}', '{}')",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }
```

</details>



##### `__eq__`

```rust
fn __eq__ (& self , other : & PyWorkflowContext) -> bool
```

> **Python API**: [cloaca.python.workflow_context.WorkflowContext.__eq__](../../../cloaca/python/workflow_context.md#__eq__)

Equality comparison

<details>
<summary>Source</summary>

```rust
    pub fn __eq__(&self, other: &PyWorkflowContext) -> bool {
        self.tenant_id == other.tenant_id
            && self.package_name == other.package_name
            && self.workflow_id == other.workflow_id
    }
```

</details>



##### `default` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn default () -> Self
```

Get the default workflow context (for backward compatibility)

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        Self {
            tenant_id: "public".to_string(),
            package_name: "embedded".to_string(),
            workflow_id: "default".to_string(),
        }
    }
```

</details>



##### `as_components` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn as_components (& self) -> (& str , & str , & str)
```

Convert to namespace components

<details>
<summary>Source</summary>

```rust
    pub fn as_components(&self) -> (&str, &str, &str) {
        (&self.tenant_id, &self.package_name, &self.workflow_id)
    }
```

</details>
