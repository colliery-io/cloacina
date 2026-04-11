# cloaca.python.namespace <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.namespace.TaskNamespace`

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace](../../rust/cloacina/python/namespace.md#class-tasknamespace)

Python wrapper for TaskNamespace

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(tenant_id: str, package_name: str, workflow_id: str, task_id: str) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::new](../../rust/cloacina/python/namespace.md#new)

Create a new TaskNamespace

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `str` |  |
| `package_name` | `str` |  |
| `workflow_id` | `str` |  |
| `task_id` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: crate::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }
```

</details>



##### `from_string`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">from_string</span>(namespace_str: str) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::from_string](../../rust/cloacina/python/namespace.md#from_string)

Parse TaskNamespace from string format "tenant::package::workflow::task"

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace_str` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn from_string(namespace_str: &str) -> PyResult<Self> {
        crate::TaskNamespace::from_string(namespace_str)
            .map(|inner| Self { inner })
            .map_err(|e| PyValueError::new_err(format!("Invalid namespace format: {}", e)))
    }
```

</details>



##### `tenant_id`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">tenant_id</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::tenant_id](../../rust/cloacina/python/namespace.md#tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```python
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }
```

</details>



##### `package_name`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">package_name</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::package_name](../../rust/cloacina/python/namespace.md#package_name)

Get package name

<details>
<summary>Source</summary>

```python
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }
```

</details>



##### `workflow_id`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">workflow_id</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::workflow_id](../../rust/cloacina/python/namespace.md#workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```python
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }
```

</details>



##### `task_id`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">task_id</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::task_id](../../rust/cloacina/python/namespace.md#task_id)

Get task ID

<details>
<summary>Source</summary>

```python
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }
```

</details>



##### `parent`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">parent</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::parent](../../rust/cloacina/python/namespace.md#parent)

Get parent namespace (without task_id)

<details>
<summary>Source</summary>

```python
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

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">is_child_of</span>(parent: PyTaskNamespace) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::is_child_of](../../rust/cloacina/python/namespace.md#is_child_of)

Check if this namespace is a child of another

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `parent` | `PyTaskNamespace` |  |


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



##### `is_sibling_of`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">is_sibling_of</span>(other: PyTaskNamespace) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::is_sibling_of](../../rust/cloacina/python/namespace.md#is_sibling_of)

Check if this namespace is a sibling of another (same parent)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `PyTaskNamespace` |  |


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



##### `__str__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__str__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::__str__](../../rust/cloacina/python/namespace.md#__str__)

String representation

<details>
<summary>Source</summary>

```python
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::__repr__](../../rust/cloacina/python/namespace.md#__repr__)

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



##### `__eq__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__eq__</span>(other: PyTaskNamespace) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::__eq__](../../rust/cloacina/python/namespace.md#__eq__)

Equality comparison

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `PyTaskNamespace` |  |


<details>
<summary>Source</summary>

```python
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }
```

</details>



##### `__hash__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__hash__</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::namespace::PyTaskNamespace::__hash__](../../rust/cloacina/python/namespace.md#__hash__)

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
