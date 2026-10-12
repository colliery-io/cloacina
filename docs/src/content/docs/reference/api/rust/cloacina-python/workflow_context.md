---
title: "cloacina-python::workflow_context"
description: "API reference of the Rust module cloacina-python::workflow_context."
sidebar:
  order: 44
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


## Structs

### <span id="struct.PyWorkflowContext"></span>`class WorkflowContext`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.workflow_context.WorkflowContext](../../../cloacina-python/workflow_context/#class.WorkflowContext)

WorkflowContext provides namespace management for Python workflows

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `String` |  |
| `package_name` | `String` |  |
| `workflow_id` | `String` |  |

#### Methods

##### <span id="PyWorkflowContext.new"></span>`new`

```rust
fn new (tenant_id : & str , package_name : & str , workflow_id : & str) -> Self
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.new](../../../cloacina-python/workflow_context/#WorkflowContext.new)

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



##### <span id="PyWorkflowContext.tenant_id"></span>`tenant_id`

```rust
fn tenant_id (& self) -> & str
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.tenant_id](../../../cloacina-python/workflow_context/#WorkflowContext.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
```

</details>



##### <span id="PyWorkflowContext.package_name"></span>`package_name`

```rust
fn package_name (& self) -> & str
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.package_name](../../../cloacina-python/workflow_context/#WorkflowContext.package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
```

</details>



##### <span id="PyWorkflowContext.workflow_id"></span>`workflow_id`

```rust
fn workflow_id (& self) -> & str
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.workflow_id](../../../cloacina-python/workflow_context/#WorkflowContext.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
```

</details>



##### <span id="PyWorkflowContext.task_namespace"></span>`task_namespace`

```rust
fn task_namespace (& self , task_id : & str) -> PyTaskNamespace
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.task_namespace](../../../cloacina-python/workflow_context/#WorkflowContext.task_namespace)

Generate a TaskNamespace for a task within this workflow context

<details>
<summary>Source</summary>

```rust
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



##### <span id="PyWorkflowContext.resolve_dependency"></span>`resolve_dependency`

```rust
fn resolve_dependency (& self , task_name : & str) -> PyTaskNamespace
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.resolve_dependency](../../../cloacina-python/workflow_context/#WorkflowContext.resolve_dependency)

Resolve a dependency task name to a full TaskNamespace within this context

<details>
<summary>Source</summary>

```rust
    pub fn resolve_dependency(&self, task_name: &str) -> PyTaskNamespace {
        self.task_namespace(task_name)
    }
```

</details>



##### <span id="PyWorkflowContext.workflow_namespace"></span>`workflow_namespace`

```rust
fn workflow_namespace (& self) -> PyTaskNamespace
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.workflow_namespace](../../../cloacina-python/workflow_context/#WorkflowContext.workflow_namespace)

Get the workflow namespace (without task_id)

<details>
<summary>Source</summary>

```rust
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



##### <span id="PyWorkflowContext.contains_namespace"></span>`contains_namespace`

```rust
fn contains_namespace (& self , namespace : & PyTaskNamespace) -> bool
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.contains_namespace](../../../cloacina-python/workflow_context/#WorkflowContext.contains_namespace)

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



##### <span id="PyWorkflowContext.__str__"></span>`__str__`

```rust
fn __str__ (& self) -> String
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.__str__](../../../cloacina-python/workflow_context/#WorkflowContext.__str__)

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



##### <span id="PyWorkflowContext.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.__repr__](../../../cloacina-python/workflow_context/#WorkflowContext.__repr__)

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



##### <span id="PyWorkflowContext.__eq__"></span>`__eq__`

```rust
fn __eq__ (& self , other : & PyWorkflowContext) -> bool
```

> **Python API**: [cloacina-python.workflow_context.WorkflowContext.__eq__](../../../cloacina-python/workflow_context/#WorkflowContext.__eq__)

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



##### <span id="PyWorkflowContext.default"></span>`default`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



##### <span id="PyWorkflowContext.as_components"></span>`as_components`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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





