# cloacina::python::context <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


## Structs

### `cloacina::python::context::Context`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.context.Context](../../../cloaca/python/context.md#class-context)

PyContext - Python wrapper for Rust Context<serde_json::Value>

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `inner` | `crate :: Context < serde_json :: Value >` |  |

#### Methods

##### `new`

```rust
fn new (data : Option < & Bound < '_ , PyDict > >) -> PyResult < Self >
```

> **Python API**: [cloaca.python.context.Context.new](../../../cloaca/python/context.md#new)

Creates a new empty context

<details>
<summary>Source</summary>

```rust
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

```rust
fn get (& self , key : & str , default : Option < & Bound < '_ , PyAny > >) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.context.Context.get](../../../cloaca/python/context.md#get)

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



##### `set`

```rust
fn set (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloaca.python.context.Context.set](../../../cloaca/python/context.md#set)

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



##### `update`

```rust
fn update (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloaca.python.context.Context.update](../../../cloaca/python/context.md#update)

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



##### `insert`

```rust
fn insert (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloaca.python.context.Context.insert](../../../cloaca/python/context.md#insert)

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



##### `remove`

```rust
fn remove (& mut self , key : & str) -> PyResult < Option < PyObject > >
```

> **Python API**: [cloaca.python.context.Context.remove](../../../cloaca/python/context.md#remove)

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



##### `to_dict`

```rust
fn to_dict (& self , py : Python < '_ >) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.context.Context.to_dict](../../../cloaca/python/context.md#to_dict)

Returns the context as a Python dictionary

<details>
<summary>Source</summary>

```rust
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }
```

</details>



##### `update_from_dict`

```rust
fn update_from_dict (& mut self , data : & Bound < '_ , PyDict >) -> PyResult < () >
```

> **Python API**: [cloaca.python.context.Context.update_from_dict](../../../cloaca/python/context.md#update_from_dict)

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



##### `to_json`

```rust
fn to_json (& self) -> PyResult < String >
```

> **Python API**: [cloaca.python.context.Context.to_json](../../../cloaca/python/context.md#to_json)

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



##### `from_json`

```rust
fn from_json (json_str : & str) -> PyResult < Self >
```

> **Python API**: [cloaca.python.context.Context.from_json](../../../cloaca/python/context.md#from_json)

Creates a context from a JSON string

<details>
<summary>Source</summary>

```rust
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

```rust
fn __len__ (& self) -> usize
```

> **Python API**: [cloaca.python.context.Context.__len__](../../../cloaca/python/context.md#__len__)

Returns the number of key-value pairs in the context

<details>
<summary>Source</summary>

```rust
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }
```

</details>



##### `__contains__`

```rust
fn __contains__ (& self , key : & str) -> bool
```

> **Python API**: [cloaca.python.context.Context.__contains__](../../../cloaca/python/context.md#__contains__)

Checks if a key exists in the context

<details>
<summary>Source</summary>

```rust
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.context.Context.__repr__](../../../cloaca/python/context.md#__repr__)

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



##### `__getitem__`

```rust
fn __getitem__ (& self , key : & str) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.context.Context.__getitem__](../../../cloaca/python/context.md#__getitem__)

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



##### `__setitem__`

```rust
fn __setitem__ (& mut self , key : & str , value : & Bound < '_ , PyAny >) -> PyResult < () >
```

> **Python API**: [cloaca.python.context.Context.__setitem__](../../../cloaca/python/context.md#__setitem__)

Dictionary-style item assignment

<details>
<summary>Source</summary>

```rust
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
```

</details>



##### `__delitem__`

```rust
fn __delitem__ (& mut self , key : & str) -> PyResult < () >
```

> **Python API**: [cloaca.python.context.Context.__delitem__](../../../cloaca/python/context.md#__delitem__)

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



##### `from_rust_context` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn from_rust_context (context : crate :: Context < serde_json :: Value >) -> Self
```

Create a PyContext from a Rust Context (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn from_rust_context(context: crate::Context<serde_json::Value>) -> Self {
        PyContext { inner: context }
    }
```

</details>



##### `into_inner` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn into_inner (self) -> crate :: Context < serde_json :: Value >
```

Extract the inner Rust Context (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn into_inner(self) -> crate::Context<serde_json::Value> {
        self.inner
    }
```

</details>



##### `clone_inner` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn clone_inner (& self) -> crate :: Context < serde_json :: Value >
```

Clone the inner Rust Context (for internal use)

<details>
<summary>Source</summary>

```rust
    pub fn clone_inner(&self) -> crate::Context<serde_json::Value> {
        self.inner.clone_data()
    }
```

</details>



##### `get_data_clone` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


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
