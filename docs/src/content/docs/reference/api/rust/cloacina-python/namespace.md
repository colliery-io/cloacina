---
title: "cloacina-python::namespace"
description: "API reference of the Rust module cloacina-python::namespace."
sidebar:
  order: 34
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


## Structs

### <span id="struct.PyTaskNamespace"></span>`class TaskNamespace`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.namespace.TaskNamespace](../../../cloacina-python/namespace/#class.TaskNamespace)

Python wrapper for TaskNamespace

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina :: TaskNamespace` |  |

#### Methods

##### <span id="PyTaskNamespace.new"></span>`new`

```rust
fn new (tenant_id : & str , package_name : & str , workflow_id : & str , task_id : & str) -> Self
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.new](../../../cloacina-python/namespace/#TaskNamespace.new)

Create a new TaskNamespace

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }
```

</details>



##### <span id="PyTaskNamespace.from_string"></span>`from_string`

```rust
fn from_string (namespace_str : & str) -> PyResult < Self >
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.from_string](../../../cloacina-python/namespace/#TaskNamespace.from_string)

Parse TaskNamespace from string format "tenant::package::workflow::task"

<details>
<summary>Source</summary>

```rust
    pub fn from_string(namespace_str: &str) -> PyResult<Self> {
        cloacina::TaskNamespace::from_string(namespace_str)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("Invalid namespace format: {}", e)))
    }
```

</details>



##### <span id="PyTaskNamespace.tenant_id"></span>`tenant_id`

```rust
fn tenant_id (& self) -> & str
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.tenant_id](../../../cloacina-python/namespace/#TaskNamespace.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }
```

</details>



##### <span id="PyTaskNamespace.package_name"></span>`package_name`

```rust
fn package_name (& self) -> & str
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.package_name](../../../cloacina-python/namespace/#TaskNamespace.package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }
```

</details>



##### <span id="PyTaskNamespace.workflow_id"></span>`workflow_id`

```rust
fn workflow_id (& self) -> & str
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.workflow_id](../../../cloacina-python/namespace/#TaskNamespace.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }
```

</details>



##### <span id="PyTaskNamespace.task_id"></span>`task_id`

```rust
fn task_id (& self) -> & str
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.task_id](../../../cloacina-python/namespace/#TaskNamespace.task_id)

Get task ID

<details>
<summary>Source</summary>

```rust
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }
```

</details>



##### <span id="PyTaskNamespace.parent"></span>`parent`

```rust
fn parent (& self) -> Self
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.parent](../../../cloacina-python/namespace/#TaskNamespace.parent)

Get parent namespace (without task_id)

<details>
<summary>Source</summary>

```rust
    pub fn parent(&self) -> Self {
        Self {
            inner: cloacina::TaskNamespace::new(
                &self.inner.tenant_id,
                &self.inner.package_name,
                &self.inner.workflow_id,
                "",
            ),
        }
    }
```

</details>



##### <span id="PyTaskNamespace.is_child_of"></span>`is_child_of`

```rust
fn is_child_of (& self , parent : & PyTaskNamespace) -> bool
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.is_child_of](../../../cloacina-python/namespace/#TaskNamespace.is_child_of)

Check if this namespace is a child of another

<details>
<summary>Source</summary>

```rust
    pub fn is_child_of(&self, parent: &PyTaskNamespace) -> bool {
        self.inner.tenant_id == parent.inner.tenant_id
            && self.inner.package_name == parent.inner.package_name
            && self.inner.workflow_id == parent.inner.workflow_id
            && !self.inner.task_id.is_empty()
            && parent.inner.task_id.is_empty()
    }
```

</details>



##### <span id="PyTaskNamespace.is_sibling_of"></span>`is_sibling_of`

```rust
fn is_sibling_of (& self , other : & PyTaskNamespace) -> bool
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.is_sibling_of](../../../cloacina-python/namespace/#TaskNamespace.is_sibling_of)

Check if this namespace is a sibling of another (same parent)

<details>
<summary>Source</summary>

```rust
    pub fn is_sibling_of(&self, other: &PyTaskNamespace) -> bool {
        self.inner.tenant_id == other.inner.tenant_id
            && self.inner.package_name == other.inner.package_name
            && self.inner.workflow_id == other.inner.workflow_id
            && !self.inner.task_id.is_empty()
            && !other.inner.task_id.is_empty()
            && self.inner.task_id != other.inner.task_id
    }
```

</details>



##### <span id="PyTaskNamespace.__str__"></span>`__str__`

```rust
fn __str__ (& self) -> String
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.__str__](../../../cloacina-python/namespace/#TaskNamespace.__str__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
```

</details>



##### <span id="PyTaskNamespace.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.__repr__](../../../cloacina-python/namespace/#TaskNamespace.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "TaskNamespace('{}', '{}', '{}', '{}')",
            self.inner.tenant_id,
            self.inner.package_name,
            self.inner.workflow_id,
            self.inner.task_id
        )
    }
```

</details>



##### <span id="PyTaskNamespace.__eq__"></span>`__eq__`

```rust
fn __eq__ (& self , other : & PyTaskNamespace) -> bool
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.__eq__](../../../cloacina-python/namespace/#TaskNamespace.__eq__)

Equality comparison

<details>
<summary>Source</summary>

```rust
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }
```

</details>



##### <span id="PyTaskNamespace.__hash__"></span>`__hash__`

```rust
fn __hash__ (& self) -> u64
```

> **Python API**: [cloacina-python.namespace.TaskNamespace.__hash__](../../../cloacina-python/namespace/#TaskNamespace.__hash__)

Hash for use in sets/dicts

<details>
<summary>Source</summary>

```rust
    pub fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
```

</details>



##### <span id="PyTaskNamespace.from_rust"></span>`from_rust`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn from_rust (namespace : cloacina :: TaskNamespace) -> Self
```

Convert from Rust TaskNamespace (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn from_rust(namespace: cloacina::TaskNamespace) -> Self {
        Self { inner: namespace }
    }
```

</details>



##### <span id="PyTaskNamespace.to_rust"></span>`to_rust`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn to_rust (& self) -> cloacina :: TaskNamespace
```

Convert to Rust TaskNamespace (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn to_rust(&self) -> cloacina::TaskNamespace {
        self.inner.clone()
    }
```

</details>





