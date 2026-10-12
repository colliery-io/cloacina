---
title: "cloacina-python.namespace"
description: "API reference of the Python module cloacina-python.namespace."
sidebar:
  order: 10
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.TaskNamespace"></span>`class TaskNamespace`

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace](../../rust/cloacina-python/namespace/#struct.PyTaskNamespace)

Python wrapper for TaskNamespace

#### Methods

##### <span id="TaskNamespace.new"></span>`new`

```python
def new(tenant_id:  str, package_name:  str, workflow_id:  str, task_id:  str) -> Self
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::new](../../rust/cloacina-python/namespace/#PyTaskNamespace.new)

Create a new TaskNamespace

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | ` str` |  |
| `package_name` | ` str` |  |
| `workflow_id` | ` str` |  |
| `task_id` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }
```

</details>



##### <span id="TaskNamespace.from_string"></span>`from_string`

```python
def from_string(namespace_str:  str) -> PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::from_string](../../rust/cloacina-python/namespace/#PyTaskNamespace.from_string)

Parse TaskNamespace from string format "tenant::package::workflow::task"

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace_str` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn from_string(namespace_str: &str) -> PyResult<Self> {
        cloacina::TaskNamespace::from_string(namespace_str)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("Invalid namespace format: {}", e)))
    }
```

</details>



##### <span id="TaskNamespace.tenant_id"></span>`tenant_id`

```python
def tenant_id() ->  str
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::tenant_id](../../rust/cloacina-python/namespace/#PyTaskNamespace.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```python
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }
```

</details>



##### <span id="TaskNamespace.package_name"></span>`package_name`

```python
def package_name() ->  str
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::package_name](../../rust/cloacina-python/namespace/#PyTaskNamespace.package_name)

Get package name

<details>
<summary>Source</summary>

```python
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }
```

</details>



##### <span id="TaskNamespace.workflow_id"></span>`workflow_id`

```python
def workflow_id() ->  str
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::workflow_id](../../rust/cloacina-python/namespace/#PyTaskNamespace.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```python
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }
```

</details>



##### <span id="TaskNamespace.task_id"></span>`task_id`

```python
def task_id() ->  str
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::task_id](../../rust/cloacina-python/namespace/#PyTaskNamespace.task_id)

Get task ID

<details>
<summary>Source</summary>

```python
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }
```

</details>



##### <span id="TaskNamespace.parent"></span>`parent`

```python
def parent() -> Self
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::parent](../../rust/cloacina-python/namespace/#PyTaskNamespace.parent)

Get parent namespace (without task_id)

<details>
<summary>Source</summary>

```python
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



##### <span id="TaskNamespace.is_child_of"></span>`is_child_of`

```python
def is_child_of(parent:  PyTaskNamespace) -> bool
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::is_child_of](../../rust/cloacina-python/namespace/#PyTaskNamespace.is_child_of)

Check if this namespace is a child of another

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `parent` | ` PyTaskNamespace` |  |


<details>
<summary>Source</summary>

```python
    pub fn is_child_of(&self, parent: &PyTaskNamespace) -> bool {
        self.inner.tenant_id == parent.inner.tenant_id
            && self.inner.package_name == parent.inner.package_name
            && self.inner.workflow_id == parent.inner.workflow_id
            && !self.inner.task_id.is_empty()
            && parent.inner.task_id.is_empty()
    }
```

</details>



##### <span id="TaskNamespace.is_sibling_of"></span>`is_sibling_of`

```python
def is_sibling_of(other:  PyTaskNamespace) -> bool
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::is_sibling_of](../../rust/cloacina-python/namespace/#PyTaskNamespace.is_sibling_of)

Check if this namespace is a sibling of another (same parent)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | ` PyTaskNamespace` |  |


<details>
<summary>Source</summary>

```python
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



##### <span id="TaskNamespace.__str__"></span>`__str__`

```python
def __str__() -> str
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::__str__](../../rust/cloacina-python/namespace/#PyTaskNamespace.__str__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
```

</details>



##### <span id="TaskNamespace.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::__repr__](../../rust/cloacina-python/namespace/#PyTaskNamespace.__repr__)

String representation

<details>
<summary>Source</summary>

```python
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



##### <span id="TaskNamespace.__eq__"></span>`__eq__`

```python
def __eq__(other:  PyTaskNamespace) -> bool
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::__eq__](../../rust/cloacina-python/namespace/#PyTaskNamespace.__eq__)

Equality comparison

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | ` PyTaskNamespace` |  |


<details>
<summary>Source</summary>

```python
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }
```

</details>



##### <span id="TaskNamespace.__hash__"></span>`__hash__`

```python
def __hash__() -> int
```

> **Rust Implementation**: [cloacina-python::namespace::PyTaskNamespace::__hash__](../../rust/cloacina-python/namespace/#PyTaskNamespace.__hash__)

Hash for use in sets/dicts

<details>
<summary>Source</summary>

```python
    pub fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.inner.hash(&mut hasher);
        hasher.finish()
    }
```

</details>





