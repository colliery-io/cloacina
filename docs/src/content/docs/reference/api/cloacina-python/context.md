---
title: "cloacina-python.context"
description: "API reference of the Python module cloacina-python.context."
sidebar:
  order: 8
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


## Classes

### <span id="class.Context"></span>`class Context`

> **Rust Implementation**: [cloacina-python::context::PyContext](../../rust/cloacina-python/context/#struct.PyContext)

PyContext - Python wrapper for Rust Context<serde_json::Value>

#### Methods

##### <span id="Context.new"></span>`new`

```python
def new(data: Option < & Bound < '_ , PyDict > >) -> PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::new](../../rust/cloacina-python/context/#PyContext.new)

Creates a new empty context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `Option < & Bound < '_ , PyDict > >` |  |


<details>
<summary>Source</summary>

```python
    pub fn new(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut context = cloacina::Context::new();

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



##### <span id="Context.get"></span>`get`

```python
def get(key:  str, default: Option < & Bound < '_ , PyAny > >) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::get](../../rust/cloacina-python/context/#PyContext.get)

Gets a value from the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |
| `default` | `Option < & Bound < '_ , PyAny > >` |  |


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



##### <span id="Context.set"></span>`set`

```python
def set(key:  str, value:  Bound < '_ , PyAny >) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::set](../../rust/cloacina-python/context/#PyContext.set)

Sets a value in the context (insert or update)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |
| `value` | ` Bound < '_ , PyAny >` |  |


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



##### <span id="Context.update"></span>`update`

```python
def update(key:  str, value:  Bound < '_ , PyAny >) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::update](../../rust/cloacina-python/context/#PyContext.update)

Updates an existing value in the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |
| `value` | ` Bound < '_ , PyAny >` |  |


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



##### <span id="Context.insert"></span>`insert`

```python
def insert(key:  str, value:  Bound < '_ , PyAny >) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::insert](../../rust/cloacina-python/context/#PyContext.insert)

Inserts a new value into the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |
| `value` | ` Bound < '_ , PyAny >` |  |


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



##### <span id="Context.remove"></span>`remove`

```python
def remove(key:  str) -> PyResult < Option < PyObject > >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::remove](../../rust/cloacina-python/context/#PyContext.remove)

Removes and returns a value from the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |


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



##### <span id="Context.to_dict"></span>`to_dict`

```python
def to_dict() -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::to_dict](../../rust/cloacina-python/context/#PyContext.to_dict)

Returns the context as a Python dictionary

<details>
<summary>Source</summary>

```python
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }
```

</details>



##### <span id="Context.update_from_dict"></span>`update_from_dict`

```python
def update_from_dict(data:  Bound < '_ , PyDict >) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::update_from_dict](../../rust/cloacina-python/context/#PyContext.update_from_dict)

Updates the context with values from a Python dictionary

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | ` Bound < '_ , PyDict >` |  |


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



##### <span id="Context.to_json"></span>`to_json`

```python
def to_json() -> PyResult < String >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::to_json](../../rust/cloacina-python/context/#PyContext.to_json)

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



##### <span id="Context.from_json"></span>`from_json`

```python
def from_json(json_str:  str) -> PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::from_json](../../rust/cloacina-python/context/#PyContext.from_json)

Creates a context from a JSON string

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `json_str` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn from_json(json_str: &str) -> PyResult<Self> {
        let context = cloacina::Context::from_json(json_str.to_string()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to deserialize from JSON: {}",
                e
            ))
        })?;
        Ok(PyContext { inner: context })
    }
```

</details>



##### <span id="Context.__len__"></span>`__len__`

```python
def __len__() -> int
```

> **Rust Implementation**: [cloacina-python::context::PyContext::__len__](../../rust/cloacina-python/context/#PyContext.__len__)

Returns the number of key-value pairs in the context

<details>
<summary>Source</summary>

```python
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }
```

</details>



##### <span id="Context.__contains__"></span>`__contains__`

```python
def __contains__(key:  str) -> bool
```

> **Rust Implementation**: [cloacina-python::context::PyContext::__contains__](../../rust/cloacina-python/context/#PyContext.__contains__)

Checks if a key exists in the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }
```

</details>



##### <span id="Context.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::context::PyContext::__repr__](../../rust/cloacina-python/context/#PyContext.__repr__)

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



##### <span id="Context.__getitem__"></span>`__getitem__`

```python
def __getitem__(key:  str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::__getitem__](../../rust/cloacina-python/context/#PyContext.__getitem__)

Dictionary-style item access

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |


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



##### <span id="Context.__setitem__"></span>`__setitem__`

```python
def __setitem__(key:  str, value:  Bound < '_ , PyAny >) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::__setitem__](../../rust/cloacina-python/context/#PyContext.__setitem__)

Dictionary-style item assignment

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |
| `value` | ` Bound < '_ , PyAny >` |  |


<details>
<summary>Source</summary>

```python
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
```

</details>



##### <span id="Context.__delitem__"></span>`__delitem__`

```python
def __delitem__(key:  str) -> PyResult < () >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::__delitem__](../../rust/cloacina-python/context/#PyContext.__delitem__)

Dictionary-style item deletion

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | ` str` |  |


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



##### <span id="Context.secret"></span>`secret`

```python
def secret(name:  str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::secret](../../rust/cloacina-python/context/#PyContext.secret)

Read a resolved secret by name, returning a dict of its `{field: value}` pairs (CLOACI-I-0133 / T-0864 — the Python read accessor).

Calls through to the SAME Rust [`cloacina::Context::secret`] accessor and
the SAME `SecretResolver` the runtime attached to this context at
execution time, so it is alias-aware through the instance's `{"$secret"}`
map (T-0859): `name` may be either the declared local binding name or the
concrete secret name.
No-leak: the resolved value is returned to the caller ONLY. It is never
written back into the context's serialized `data`, so it cannot reach
`schedules.params`, the fires log, or execution history.
Raises `RuntimeError` when no resolver is configured on this execution
scope (`CLOACINA_SECRET_KEK` unset) or the backend fails, `KeyError` when
the secret is not found, `PermissionError` when the name is not granted.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn secret(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
        let fields = crate::gil::py_block_on(py, self.inner.secret(name))
            .map_err(secret_access_error_to_pyerr)?;
        let dict = PyDict::new(py);
        for (field, value) in fields {
            dict.set_item(field, value)?;
        }
        Ok(dict.into())
    }
```

</details>



##### <span id="Context.secret_field"></span>`secret_field`

```python
def secret_field(name:  str, field:  str) -> PyResult < String >
```

> **Rust Implementation**: [cloacina-python::context::PyContext::secret_field](../../rust/cloacina-python/context/#PyContext.secret_field)

Read a single field of a named secret (convenience over [`Self::secret`]).

Same resolver, alias-awareness, no-leak, and error mapping as
[`Self::secret`]; additionally raises `KeyError` when the secret exists but
has no field of that name.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | ` str` |  |
| `field` | ` str` |  |


<details>
<summary>Source</summary>

```python
    pub fn secret_field(&self, py: Python<'_>, name: &str, field: &str) -> PyResult<String> {
        crate::gil::py_block_on(py, self.inner.secret_field(name, field))
            .map_err(secret_access_error_to_pyerr)
    }
```

</details>





