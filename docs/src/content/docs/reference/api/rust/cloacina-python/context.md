---
title: "cloacina-python::context"
description: "API reference of the Rust module cloacina-python::context."
editUrl: false
sidebar:
  order: 14
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

## Structs

### <span id="struct.PyContext"></span><code style="font-size: 0.85em">class Context</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.Context`](../../../cloaca/#class.Context)

PyContext - Python wrapper for Rust Context<serde_json::Value>

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina::Context<serde_json::Value>` |  |

#### Methods

##### <span id="PyContext.new"></span><code>new</code>

```rust
fn new(data: Option<&Bound<'_, PyDict>>) -> PyResult<Self>
```

> **Python API**: [`cloaca.Context.new`](../../../cloaca/#Context.new)

Creates a new empty context

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.get"></span><code>get</code>

```rust
fn get(
    &self,
    key: &str,
    default: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.Context.get`](../../../cloaca/#Context.get)

Gets a value from the context

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.set"></span><code>set</code>

```rust
fn set(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()>
```

> **Python API**: [`cloaca.Context.set`](../../../cloaca/#Context.set)

Sets a value in the context (insert or update)

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.update"></span><code>update</code>

```rust
fn update(
    &mut self) -> PyResult<(,
    key: &str) -> PyResult<(,
    value: &Bound<'_, PyAny>) -> PyResult<(,
) -> PyResult<()>
```

> **Python API**: [`cloaca.Context.update`](../../../cloaca/#Context.update)

Updates an existing value in the context

<details>
<summary>Source</summary>

```rust
    pub fn update(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.update(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!("Key not found: {}", e))
        })
    }
```

</details>

##### <span id="PyContext.insert"></span><code>insert</code>

```rust
fn insert(
    &mut self) -> PyResult<(,
    key: &str) -> PyResult<(,
    value: &Bound<'_, PyAny>) -> PyResult<(,
) -> PyResult<()>
```

> **Python API**: [`cloaca.Context.insert`](../../../cloaca/#Context.insert)

Inserts a new value into the context

<details>
<summary>Source</summary>

```rust
    pub fn insert(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value: serde_json::Value = depythonize(value)?;
        self.inner.insert(key, json_value).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Key already exists: {}", e))
        })
    }
```

</details>

##### <span id="PyContext.remove"></span><code>remove</code>

```rust
fn remove(&mut self, key: &str) -> PyResult<Option<PyObject>>
```

> **Python API**: [`cloaca.Context.remove`](../../../cloaca/#Context.remove)

Removes and returns a value from the context

<details>
<summary>Source</summary>

```rust
    pub fn remove(&mut self, key: &str) -> PyResult<Option<PyObject>> {
        match self.inner.remove(key) {
            Some(value) => Python::with_gil(|py| Ok(Some(pythonize(py, &value)?.into()))),
            None => Ok(None),
        }
    }
```

</details>

##### <span id="PyContext.to_dict"></span><code>to_​dict</code>

```rust
fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.Context.to_dict`](../../../cloaca/#Context.to_dict)

Returns the context as a Python dictionary

<details>
<summary>Source</summary>

```rust
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }
```

</details>

##### <span id="PyContext.update_from_dict"></span><code>update_​from_​dict</code>

```rust
fn update_from_dict(&mut self, data: &Bound<'_, PyDict>) -> PyResult<()>
```

> **Python API**: [`cloaca.Context.update_from_dict`](../../../cloaca/#Context.update_from_dict)

Updates the context with values from a Python dictionary

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.to_json"></span><code>to_​json</code>

```rust
fn to_json(&self) -> PyResult<String>
```

> **Python API**: [`cloaca.Context.to_json`](../../../cloaca/#Context.to_json)

Serializes the context to a JSON string

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.from_json"></span><code>from_​json</code>

```rust
fn from_json(json_str: &str) -> PyResult<Self>
```

> **Python API**: [`cloaca.Context.from_json`](../../../cloaca/#Context.from_json)

Creates a context from a JSON string

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.__len__"></span><code>_​_​len_​_​</code>

```rust
fn __len__(&self) -> usize
```

> **Python API**: [`cloaca.Context.__len__`](../../../cloaca/#Context.__len__)

Returns the number of key-value pairs in the context

<details>
<summary>Source</summary>

```rust
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }
```

</details>

##### <span id="PyContext.__contains__"></span><code>_​_​contains_​_​</code>

```rust
fn __contains__(&self, key: &str) -> bool
```

> **Python API**: [`cloaca.Context.__contains__`](../../../cloaca/#Context.__contains__)

Checks if a key exists in the context

<details>
<summary>Source</summary>

```rust
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }
```

</details>

##### <span id="PyContext.__repr__"></span><code>_​_​repr_​_​</code>

```rust
fn __repr__(&self) -> String
```

> **Python API**: [`cloaca.Context.__repr__`](../../../cloaca/#Context.__repr__)

String representation of the context

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        match self.inner.to_json() {
            Ok(json) => format!("Context({})", json),
            Err(_) => "Context(<serialization error>)".to_string(),
        }
    }
```

</details>

##### <span id="PyContext.__getitem__"></span><code>_​_​getitem_​_​</code>

```rust
fn __getitem__(&self, key: &str) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.Context.__getitem__`](../../../cloaca/#Context.__getitem__)

Dictionary-style item access

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.__setitem__"></span><code>_​_​setitem_​_​</code>

```rust
fn __setitem__(
    &mut self) -> PyResult<(,
    key: &str) -> PyResult<(,
    value: &Bound<'_, PyAny>) -> PyResult<(,
) -> PyResult<()>
```

> **Python API**: [`cloaca.Context.__setitem__`](../../../cloaca/#Context.__setitem__)

Dictionary-style item assignment

<details>
<summary>Source</summary>

```rust
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
```

</details>

##### <span id="PyContext.__delitem__"></span><code>_​_​delitem_​_​</code>

```rust
fn __delitem__(&mut self, key: &str) -> PyResult<()>
```

> **Python API**: [`cloaca.Context.__delitem__`](../../../cloaca/#Context.__delitem__)

Dictionary-style item deletion

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.secret"></span><code>secret</code>

```rust
fn secret(&self, py: Python<'_>, name: &str) -> PyResult<PyObject>
```

> **Python API**: [`cloaca.Context.secret`](../../../cloaca/#Context.secret)

Read a resolved secret by name, returning a dict of its `{field: value}` pairs (CLOACI-I-0133 / T-0864 — the Python read accessor).

Calls through to the SAME Rust [`cloacina::Context::secret`](../../../rust/cloacina-workflow/context/#Context.secret) accessor and
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

<details>
<summary>Source</summary>

```rust
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

##### <span id="PyContext.secret_field"></span><code>secret_​field</code>

```rust
fn secret_field(
    &self,
    py: Python<'_>,
    name: &str,
    field: &str,
) -> PyResult<String>
```

> **Python API**: [`cloaca.Context.secret_field`](../../../cloaca/#Context.secret_field)

Read a single field of a named secret (convenience over `Self::secret`).

Same resolver, alias-awareness, no-leak, and error mapping as
`Self::secret`; additionally raises `KeyError` when the secret exists but
has no field of that name.

<details>
<summary>Source</summary>

```rust
    pub fn secret_field(&self, py: Python<'_>, name: &str, field: &str) -> PyResult<String> {
        crate::gil::py_block_on(py, self.inner.secret_field(name, field))
            .map_err(secret_access_error_to_pyerr)
    }
```

</details>

##### <span id="PyContext.from_rust_context"></span><code>from_​rust_​context</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn from_rust_context(
    context: cloacina::Context<serde_json::Value>,
) -> Self
```

Create a PyContext from a Rust Context (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn from_rust_context(context: cloacina::Context<serde_json::Value>) -> Self {
        PyContext { inner: context }
    }
```

</details>

##### <span id="PyContext.into_inner"></span><code>into_​inner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn into_inner(self) -> cloacina::Context<serde_json::Value>
```

Extract the inner Rust Context (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn into_inner(self) -> cloacina::Context<serde_json::Value> {
        self.inner
    }
```

</details>

##### <span id="PyContext.clone_inner"></span><code>clone_​inner</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn clone_inner(&self) -> cloacina::Context<serde_json::Value>
```

Clone the inner Rust Context (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn clone_inner(&self) -> cloacina::Context<serde_json::Value> {
        self.inner.clone_data()
    }
```

</details>

##### <span id="PyContext.get_data_clone"></span><code>get_​data_​clone</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn get_data_clone(
    &self,
) -> std::collections::HashMap<String, serde_json::Value>
```

Get a clone of the context data as a HashMap (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn get_data_clone(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.inner.data().clone()
    }
```

</details>
