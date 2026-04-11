# cloacina::python::namespace <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::namespace::TaskNamespace`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.namespace.TaskNamespace](../../../cloaca/python/namespace.md#class-tasknamespace)

Python wrapper for TaskNamespace

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: TaskNamespace` |  |

#### Methods

##### `new`

```rust
fn new (tenant_id : & str , package_name : & str , workflow_id : & str , task_id : & str) -> Self
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.new](../../../cloaca/python/namespace.md#new)

Create a new TaskNamespace

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: crate::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }
```

</details>



##### `from_string`

```rust
fn from_string (namespace_str : & str) -> PyResult < Self >
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.from_string](../../../cloaca/python/namespace.md#from_string)

Parse TaskNamespace from string format "tenant::package::workflow::task"

<details>
<summary>Source</summary>

```rust
    pub fn from_string(namespace_str: &str) -> PyResult<Self> {
        crate::TaskNamespace::from_string(namespace_str)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("Invalid namespace format: {}", e)))
    }
```

</details>



##### `tenant_id`

```rust
fn tenant_id (& self) -> & str
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.tenant_id](../../../cloaca/python/namespace.md#tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }
```

</details>



##### `package_name`

```rust
fn package_name (& self) -> & str
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.package_name](../../../cloaca/python/namespace.md#package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }
```

</details>



##### `workflow_id`

```rust
fn workflow_id (& self) -> & str
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.workflow_id](../../../cloaca/python/namespace.md#workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }
```

</details>



##### `task_id`

```rust
fn task_id (& self) -> & str
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.task_id](../../../cloaca/python/namespace.md#task_id)

Get task ID

<details>
<summary>Source</summary>

```rust
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }
```

</details>



##### `parent`

```rust
fn parent (& self) -> Self
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.parent](../../../cloaca/python/namespace.md#parent)

Get parent namespace (without task_id)

<details>
<summary>Source</summary>

```rust
    pub fn parent(&self) -> Self {
        Self {
            inner: crate::TaskNamespace::new(
                &self.inner.tenant_id,
                &self.inner.package_name,
                &self.inner.workflow_id,
                "",
            ),
        }
    }
```

</details>



##### `is_child_of`

```rust
fn is_child_of (& self , parent : & PyTaskNamespace) -> bool
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.is_child_of](../../../cloaca/python/namespace.md#is_child_of)

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



##### `is_sibling_of`

```rust
fn is_sibling_of (& self , other : & PyTaskNamespace) -> bool
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.is_sibling_of](../../../cloaca/python/namespace.md#is_sibling_of)

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



##### `__str__`

```rust
fn __str__ (& self) -> String
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.__str__](../../../cloaca/python/namespace.md#__str__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.__repr__](../../../cloaca/python/namespace.md#__repr__)

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



##### `__eq__`

```rust
fn __eq__ (& self , other : & PyTaskNamespace) -> bool
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.__eq__](../../../cloaca/python/namespace.md#__eq__)

Equality comparison

<details>
<summary>Source</summary>

```rust
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }
```

</details>



##### `__hash__`

```rust
fn __hash__ (& self) -> u64
```

> **Python API**: [cloaca.python.namespace.TaskNamespace.__hash__](../../../cloaca/python/namespace.md#__hash__)

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



##### `from_rust` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_rust (namespace : crate :: TaskNamespace) -> Self
```

Convert from Rust TaskNamespace (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn from_rust(namespace: crate::TaskNamespace) -> Self {
        Self { inner: namespace }
    }
```

</details>



##### `to_rust` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn to_rust (& self) -> crate :: TaskNamespace
```

Convert to Rust TaskNamespace (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn to_rust(&self) -> crate::TaskNamespace {
        self.inner.clone()
    }
```

</details>
