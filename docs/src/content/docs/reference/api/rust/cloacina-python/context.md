---
title: "cloacina-python::context"
description: "API reference of the Rust module cloacina-python::context."
sidebar:
  order: 30
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


## Structs

### <span id="struct.PyContext"></span>`class Context`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.context.Context](../../../cloacina-python/context/#class.Context)

PyContext - Python wrapper for Rust Context<serde_json::Value>

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `cloacina :: Context < serde_json :: Value >` |  |

#### Methods

##### <span id="PyContext.new"></span>`new`

```rust
fn new (data : Option < & Bound < '_ , PyDict > >) -> PyResult < Self >
```

> **Python API**: [cloacina-python.context.Context.new](../../../cloacina-python/context/#Context.new)

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



##### <span id="PyContext.get"></span>`get`

```rust
fn get (& self , key : & str , default : Option < & Bound < '_ , PyAny > >) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.context.Context.get](../../../cloacina-python/context/#Context.get)

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



##### <span id="PyContext.set"></span>`set`

```rust
fn set (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloacina-python.context.Context.set](../../../cloacina-python/context/#Context.set)

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



##### <span id="PyContext.update"></span>`update`

```rust
fn update (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloacina-python.context.Context.update](../../../cloacina-python/context/#Context.update)

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



##### <span id="PyContext.insert"></span>`insert`

```rust
fn insert (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloacina-python.context.Context.insert](../../../cloacina-python/context/#Context.insert)

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



##### <span id="PyContext.remove"></span>`remove`

```rust
fn remove (& mut self , key : & str) -> PyResult < Option < PyObject > >
```

> **Python API**: [cloacina-python.context.Context.remove](../../../cloacina-python/context/#Context.remove)

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



##### <span id="PyContext.to_dict"></span>`to_dict`

```rust
fn to_dict (& self , py : Python < '_ >) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.context.Context.to_dict](../../../cloacina-python/context/#Context.to_dict)

Returns the context as a Python dictionary

<details>
<summary>Source</summary>

```rust
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }
```

</details>



##### <span id="PyContext.update_from_dict"></span>`update_from_dict`

```rust
fn update_from_dict (& mut self , data : & Bound < '_ , PyDict >) -> PyResult < () >
```

> **Python API**: [cloacina-python.context.Context.update_from_dict](../../../cloacina-python/context/#Context.update_from_dict)

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



##### <span id="PyContext.to_json"></span>`to_json`

```rust
fn to_json (& self) -> PyResult < String >
```

> **Python API**: [cloacina-python.context.Context.to_json](../../../cloacina-python/context/#Context.to_json)

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



##### <span id="PyContext.from_json"></span>`from_json`

```rust
fn from_json (json_str : & str) -> PyResult < Self >
```

> **Python API**: [cloacina-python.context.Context.from_json](../../../cloacina-python/context/#Context.from_json)

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



##### <span id="PyContext.__len__"></span>`__len__`

```rust
fn __len__ (& self) -> usize
```

> **Python API**: [cloacina-python.context.Context.__len__](../../../cloacina-python/context/#Context.__len__)

Returns the number of key-value pairs in the context

<details>
<summary>Source</summary>

```rust
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }
```

</details>



##### <span id="PyContext.__contains__"></span>`__contains__`

```rust
fn __contains__ (& self , key : & str) -> bool
```

> **Python API**: [cloacina-python.context.Context.__contains__](../../../cloacina-python/context/#Context.__contains__)

Checks if a key exists in the context

<details>
<summary>Source</summary>

```rust
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }
```

</details>



##### <span id="PyContext.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.context.Context.__repr__](../../../cloacina-python/context/#Context.__repr__)

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



##### <span id="PyContext.__getitem__"></span>`__getitem__`

```rust
fn __getitem__ (& self , key : & str) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.context.Context.__getitem__](../../../cloacina-python/context/#Context.__getitem__)

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



##### <span id="PyContext.__setitem__"></span>`__setitem__`

```rust
fn __setitem__ (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloacina-python.context.Context.__setitem__](../../../cloacina-python/context/#Context.__setitem__)

Dictionary-style item assignment

<details>
<summary>Source</summary>

```rust
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
```

</details>



##### <span id="PyContext.__delitem__"></span>`__delitem__`

```rust
fn __delitem__ (& mut self , key : & str) -> PyResult < () >
```

> **Python API**: [cloacina-python.context.Context.__delitem__](../../../cloacina-python/context/#Context.__delitem__)

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



##### <span id="PyContext.secret"></span>`secret`

```rust
fn secret (& self , py : Python < '_ > , name : & str) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.context.Context.secret](../../../cloacina-python/context/#Context.secret)

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



##### <span id="PyContext.secret_field"></span>`secret_field`

```rust
fn secret_field (& self , py : Python < '_ > , name : & str , field : & str) -> PyResult < String >
```

> **Python API**: [cloacina-python.context.Context.secret_field](../../../cloacina-python/context/#Context.secret_field)

Read a single field of a named secret (convenience over [`Self::secret`]).

Same resolver, alias-awareness, no-leak, and error mapping as
[`Self::secret`]; additionally raises `KeyError` when the secret exists but
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



##### <span id="PyContext.from_rust_context"></span>`from_rust_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn from_rust_context (context : cloacina :: Context < serde_json :: Value >) -> Self
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



##### <span id="PyContext.into_inner"></span>`into_inner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn into_inner (self) -> cloacina :: Context < serde_json :: Value >
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



##### <span id="PyContext.clone_inner"></span>`clone_inner`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn clone_inner (& self) -> cloacina :: Context < serde_json :: Value >
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



##### <span id="PyContext.get_data_clone"></span>`get_data_clone`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn get_data_clone (& self) -> std :: collections :: HashMap < String , serde_json :: Value >
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





## Functions

### <span id="fn.secret_access_error_to_pyerr"></span>`fn secret_access_error_to_pyerr`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn secret_access_error_to_pyerr (err : cloacina :: SecretAccessError) -> PyErr
```

Map a Rust [`cloacina::SecretAccessError`] onto a clear Python exception.

`NotConfigured`/`Backend` → `RuntimeError` (environment/backend faults),
`NotFound`/`FieldNotFound` → `KeyError` (a missing name/field), `NotGranted`
→ `PermissionError` (the grant gate denied it). The `Display` message is
non-plaintext by construction (it names the secret/field, never a value).

<details>
<summary>Source</summary>

```rust
fn secret_access_error_to_pyerr(err: cloacina::SecretAccessError) -> PyErr {
    use cloacina::SecretAccessError as E;
    let msg = err.to_string();
    match err {
        E::NotConfigured | E::Backend(_) => pyo3::exceptions::PyRuntimeError::new_err(msg),
        E::NotFound(_) | E::FieldNotFound { .. } => pyo3::exceptions::PyKeyError::new_err(msg),
        E::NotGranted(_) => pyo3::exceptions::PyPermissionError::new_err(msg),
    }
}
```

</details>



