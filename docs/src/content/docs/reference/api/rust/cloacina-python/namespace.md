---
title: "cloacina-python::namespace"
description: "API reference of the Rust module cloacina-python::namespace."
editUrl: false
sidebar:
  order: 18
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Structs

### <span id="struct.PyTaskNamespace"></span><code style="font-size: 0.85em">class TaskNamespace</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.TaskNamespace`](../../../cloaca/#class.TaskNamespace)

Python wrapper for TaskNamespace

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::TaskNamespace` |  |

#### Methods

##### <span id="PyTaskNamespace.new"></span><code>new</code>

```rust
fn new(
    tenant_id: &str,
    package_name: &str,
    workflow_id: &str,
    task_id: &str,
) -> Self
```

> **Python API**: [`cloaca.TaskNamespace.new`](../../../cloaca/#TaskNamespace.new)

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

##### <span id="PyTaskNamespace.from_string"></span><code>from_​string</code>

```rust
fn from_string(namespace_str: &str) -> PyResult<Self>
```

> **Python API**: [`cloaca.TaskNamespace.from_string`](../../../cloaca/#TaskNamespace.from_string)

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

##### <span id="PyTaskNamespace.tenant_id"></span><code>tenant_​id</code>

```rust
fn tenant_id(&self) -> &str
```

> **Python API**: [`cloaca.TaskNamespace.tenant_id`](../../../cloaca/#TaskNamespace.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }
```

</details>

##### <span id="PyTaskNamespace.package_name"></span><code>package_​name</code>

```rust
fn package_name(&self) -> &str
```

> **Python API**: [`cloaca.TaskNamespace.package_name`](../../../cloaca/#TaskNamespace.package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }
```

</details>

##### <span id="PyTaskNamespace.workflow_id"></span><code>workflow_​id</code>

```rust
fn workflow_id(&self) -> &str
```

> **Python API**: [`cloaca.TaskNamespace.workflow_id`](../../../cloaca/#TaskNamespace.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }
```

</details>

##### <span id="PyTaskNamespace.task_id"></span><code>task_​id</code>

```rust
fn task_id(&self) -> &str
```

> **Python API**: [`cloaca.TaskNamespace.task_id`](../../../cloaca/#TaskNamespace.task_id)

Get task ID

<details>
<summary>Source</summary>

```rust
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }
```

</details>

##### <span id="PyTaskNamespace.parent"></span><code>parent</code>

```rust
fn parent(&self) -> Self
```

> **Python API**: [`cloaca.TaskNamespace.parent`](../../../cloaca/#TaskNamespace.parent)

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

##### <span id="PyTaskNamespace.is_child_of"></span><code>is_​child_​of</code>

```rust
fn is_child_of(&self, parent: &PyTaskNamespace) -> bool
```

> **Python API**: [`cloaca.TaskNamespace.is_child_of`](../../../cloaca/#TaskNamespace.is_child_of)

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

##### <span id="PyTaskNamespace.is_sibling_of"></span><code>is_​sibling_​of</code>

```rust
fn is_sibling_of(&self, other: &PyTaskNamespace) -> bool
```

> **Python API**: [`cloaca.TaskNamespace.is_sibling_of`](../../../cloaca/#TaskNamespace.is_sibling_of)

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

##### <span id="PyTaskNamespace.__str__"></span><code>_​_​str_​_​</code>

```rust
fn __str__(&self) -> String
```

> **Python API**: [`cloaca.TaskNamespace.__str__`](../../../cloaca/#TaskNamespace.__str__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
```

</details>

##### <span id="PyTaskNamespace.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.TaskNamespace.__repr__`](../../../cloaca/#TaskNamespace.__repr__)

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

##### <span id="PyTaskNamespace.__eq__"></span><code>_​_​eq_​_​</code>

```rust
fn __eq__(&self, other: &PyTaskNamespace) -> bool
```

> **Python API**: [`cloaca.TaskNamespace.__eq__`](../../../cloaca/#TaskNamespace.__eq__)

Equality comparison

<details>
<summary>Source</summary>

```rust
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }
```

</details>

##### <span id="PyTaskNamespace.__hash__"></span><code>_​_​hash_​_​</code>

```rust
fn __hash__(&self) -> u64
```

> **Python API**: [`cloaca.TaskNamespace.__hash__`](../../../cloaca/#TaskNamespace.__hash__)

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

##### <span id="PyTaskNamespace.from_rust"></span><code>from_​rust</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_rust(namespace: cloacina::TaskNamespace) -> Self
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

##### <span id="PyTaskNamespace.to_rust"></span><code>to_​rust</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn to_rust(&self) -> cloacina::TaskNamespace
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
