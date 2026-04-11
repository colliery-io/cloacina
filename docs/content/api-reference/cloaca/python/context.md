# cloaca.python.context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


## Classes

### `cloaca.python.context.Context`

> **Rust Implementation**: [cloacina::python::context::PyContext](../../rust/cloacina/python/context.md#class-context)

PyContext - Python wrapper for Rust Context<serde_json::Value>

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(data: Optional[dict]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::new](../../rust/cloacina/python/context.md#new)

Creates a new empty context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `Optional[dict]` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut context = crate::Context::new();

        if let Some(dict) = data {
            for (key, value) in dict.iter() {
                let key_str: String = key.extract()?;
                let json_value: serde_json::Value = depythonize(&value)?;
                context.insert(key_str, json_value).map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Failed to insert key: {}",
                        e
                    ))
                })?;
            }
        }

        Ok(PyContext { inner: context })
    }
```

</details>



##### `get`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">get</span>(key: str, default: Optional[Any]) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::get](../../rust/cloacina/python/context.md#get)

Gets a value from the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `default` | `Optional[Any]` |  |


<details>
<summary>Source</summary>

```python
    pub fn get(&self, key: &str, default: Option<&Bound<'_, PyAny>>) -> PyResult<PyObject> {
        match self.inner.get(key) {
            Some(value) => Python::with_gil(|py| Ok(pythonize(py, value)?.into())),
            None => match default {
                Some(default_value) => Ok(default_value.clone().into()),
                None => Python::with_gil(|py| Ok(py.None())),
            },
        }
    }
```

</details>



##### `set`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">set</span>(key: str, value: Any) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::set](../../rust/cloacina/python/context.md#set)

Sets a value in the context (insert or update)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Any` |  |


<details>
<summary>Source</summary>

```python
    pub fn set(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;

        if self.inner.get(key).is_some() {
            self.inner.update(key, json_value)
        } else {
            self.inner.insert(key, json_value)
        }
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to set key '{}': {}",
                key, e
            ))
        })
    }
```

</details>



##### `update`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">update</span>(key: str, value: Any) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::update](../../rust/cloacina/python/context.md#update)

Updates an existing value in the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Any` |  |


<details>
<summary>Source</summary>

```python
    pub fn update(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.update(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Key not found: {}", e))
        })
    }
```

</details>



##### `insert`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">insert</span>(key: str, value: Any) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::insert](../../rust/cloacina/python/context.md#insert)

Inserts a new value into the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Any` |  |


<details>
<summary>Source</summary>

```python
    pub fn insert(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.insert(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Key already exists: {}", e))
        })
    }
```

</details>



##### `remove`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">remove</span>(key: str) -> <span style="color: var(--md-default-fg-color--light);">Optional[Any]</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::remove](../../rust/cloacina/python/context.md#remove)

Removes and returns a value from the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn remove(&mut self, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.remove(key) {
            Some(value) => Python::with_gil(|py| Ok(Some(pythonize(py, &value)?.into()))),
            None => Ok(None),
        }
    }
```

</details>



##### `to_dict`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">to_dict</span>() -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::to_dict](../../rust/cloacina/python/context.md#to_dict)

Returns the context as a Python dictionary

<details>
<summary>Source</summary>

```python
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }
```

</details>



##### `update_from_dict`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">update_from_dict</span>(data: dict) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::update_from_dict](../../rust/cloacina/python/context.md#update_from_dict)

Updates the context with values from a Python dictionary

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `dict` |  |


<details>
<summary>Source</summary>

```python
    pub fn update_from_dict(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()> {
        for (key, value) in data.iter() {
            let key_str: String = key.extract()?;
            let json_value: serde_json::Value = depythonize(&value)?;

            if self.inner.get(&key_str).is_some() {
                self.inner.update(key_str, json_value)
            } else {
                self.inner.insert(key_str, json_value)
            }
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Failed to update from dict: {}",
                    e
                ))
            })?;
        }
        Ok(())
    }
```

</details>



##### `to_json`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">to_json</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::to_json](../../rust/cloacina/python/context.md#to_json)

Serializes the context to a JSON string

<details>
<summary>Source</summary>

```python
    pub fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to serialize to JSON: {}",
                e
            ))
        })
    }
```

</details>



##### `from_json`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">from_json</span>(json_str: str) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::from_json](../../rust/cloacina/python/context.md#from_json)

Creates a context from a JSON string

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `json_str` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn from_json(json_str: &str) -> PyResult<Self> {
        let context = crate::Context::from_json(json_str.to_string()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to deserialize from JSON: {}",
                e
            ))
        })?;
        Ok(PyContext { inner: context })
    }
```

</details>



##### `__len__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__len__</span>() -> <span style="color: var(--md-default-fg-color--light);">int</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::__len__](../../rust/cloacina/python/context.md#__len__)

Returns the number of key-value pairs in the context

<details>
<summary>Source</summary>

```python
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }
```

</details>



##### `__contains__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__contains__</span>(key: str) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::__contains__](../../rust/cloacina/python/context.md#__contains__)

Checks if a key exists in the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::__repr__](../../rust/cloacina/python/context.md#__repr__)

String representation of the context

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        match self.inner.to_json() {
            Ok(json) => format!("Context({})", json),
            Err(_) => "Context(<serialization error>)".to_string(),
        }
    }
```

</details>



##### `__getitem__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__getitem__</span>(key: str) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::__getitem__](../../rust/cloacina/python/context.md#__getitem__)

Dictionary-style item access

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn __getitem__(&self, key: &str) -> PyResult<PyObject> {
        let result = self.get(key, None)?;
        Python::with_gil(|py| {
            if result.is_none(py) {
                Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                    "Key not found: '{}'",
                    key
                )))
            } else {
                Ok(result)
            }
        })
    }
```

</details>



##### `__setitem__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__setitem__</span>(key: str, value: Any) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::__setitem__](../../rust/cloacina/python/context.md#__setitem__)

Dictionary-style item assignment

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Any` |  |


<details>
<summary>Source</summary>

```python
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
```

</details>



##### `__delitem__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__delitem__</span>(key: str) -> <span style="color: var(--md-default-fg-color--light);">None</span></code>
</div>

> **Rust Implementation**: [cloacina::python::context::PyContext::__delitem__](../../rust/cloacina/python/context.md#__delitem__)

Dictionary-style item deletion

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |


<details>
<summary>Source</summary>

```python
    pub fn __delitem__(&mut self, key: &str) -> PyResult<()> {
        match self.remove(key)? {
            Some(_) => Ok(()),
            None => Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Key not found: '{}'",
                key
            ))),
        }
    }
```

</details>
