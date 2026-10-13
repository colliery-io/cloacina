---
title: "cloaca"
description: "API reference of the Python module cloaca."
editUrl: false
sidebar:
  order: 1
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

## Classes

### <span id="class.Context"></span><code style="font-size: 0.85em">class Context</code>

> **Rust Implementation**: [`cloacina-python::context::PyContext`](../rust/cloacina-python/context/#struct.PyContext)

PyContext - Python wrapper for Rust Context<serde_json::Value>

#### Methods

##### <span id="Context.new"></span><code>new</code>

```python
def new(data: Optional[Bound<'_, PyDict>]) -> Self
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::new`](../rust/cloacina-python/context/#PyContext.new)

Creates a new empty context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `Optional[Bound<'_, PyDict>]` |  |

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

##### <span id="Context.get"></span><code>get</code>

```python
def get(key: str, default: Optional[Bound<'_, PyAny>]) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::get`](../rust/cloacina-python/context/#PyContext.get)

Gets a value from the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `default` | `Optional[Bound<'_, PyAny>]` |  |

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

##### <span id="Context.set"></span><code>set</code>

```python
def set(key: str, value: Bound<'_, PyAny>) -> None
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::set`](../rust/cloacina-python/context/#PyContext.set)

Sets a value in the context (insert or update)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Bound<'_, PyAny>` |  |

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

##### <span id="Context.update"></span><code>update</code>

```python
def update(key: str, value: Bound<'_, PyAny>) -> None
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::update`](../rust/cloacina-python/context/#PyContext.update)

Updates an existing value in the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Bound<'_, PyAny>` |  |

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

##### <span id="Context.insert"></span><code>insert</code>

```python
def insert(key: str, value: Bound<'_, PyAny>) -> None
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::insert`](../rust/cloacina-python/context/#PyContext.insert)

Inserts a new value into the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Bound<'_, PyAny>` |  |

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

##### <span id="Context.remove"></span><code>remove</code>

```python
def remove(key: str) -> Optional[PyObject]
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::remove`](../rust/cloacina-python/context/#PyContext.remove)

Removes and returns a value from the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |

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

##### <span id="Context.to_dict"></span><code>to_​dict</code>

```python
def to_dict() -> PyObject
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::to_dict`](../rust/cloacina-python/context/#PyContext.to_dict)

Returns the context as a Python dictionary

<details>
<summary>Source</summary>

```rust
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        Ok(pythonize(py, self.inner.data())?.into())
    }
```

</details>

##### <span id="Context.update_from_dict"></span><code>update_​from_​dict</code>

```python
def update_from_dict(data: Bound<'_, PyDict>) -> None
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::update_from_dict`](../rust/cloacina-python/context/#PyContext.update_from_dict)

Updates the context with values from a Python dictionary

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `data` | `Bound<'_, PyDict>` |  |

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

##### <span id="Context.to_json"></span><code>to_​json</code>

```python
def to_json() -> str
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::to_json`](../rust/cloacina-python/context/#PyContext.to_json)

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

##### <span id="Context.from_json"></span><code>from_​json</code>

```python
def from_json(json_str: str) -> Self
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::from_json`](../rust/cloacina-python/context/#PyContext.from_json)

Creates a context from a JSON string

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `json_str` | `str` |  |

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

##### <span id="Context.__len__"></span><code>_​_​len_​_​</code>

```python
def __len__() -> int
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::__len__`](../rust/cloacina-python/context/#PyContext.__len__)

Returns the number of key-value pairs in the context

<details>
<summary>Source</summary>

```rust
    pub fn __len__(&self) -> usize {
        self.inner.data().len()
    }
```

</details>

##### <span id="Context.__contains__"></span><code>_​_​contains_​_​</code>

```python
def __contains__(key: str) -> bool
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::__contains__`](../rust/cloacina-python/context/#PyContext.__contains__)

Checks if a key exists in the context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __contains__(&self, key: &str) -> bool {
        self.inner.get(key).is_some()
    }
```

</details>

##### <span id="Context.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::__repr__`](../rust/cloacina-python/context/#PyContext.__repr__)

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

##### <span id="Context.__getitem__"></span><code>_​_​getitem_​_​</code>

```python
def __getitem__(key: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::__getitem__`](../rust/cloacina-python/context/#PyContext.__getitem__)

Dictionary-style item access

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |

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

##### <span id="Context.__setitem__"></span><code>_​_​setitem_​_​</code>

```python
def __setitem__(key: str, value: Bound<'_, PyAny>) -> None
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::__setitem__`](../rust/cloacina-python/context/#PyContext.__setitem__)

Dictionary-style item assignment

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `Bound<'_, PyAny>` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __setitem__(&mut self, key: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.set(key, value)
    }
```

</details>

##### <span id="Context.__delitem__"></span><code>_​_​delitem_​_​</code>

```python
def __delitem__(key: str) -> None
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::__delitem__`](../rust/cloacina-python/context/#PyContext.__delitem__)

Dictionary-style item deletion

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |

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

##### <span id="Context.secret"></span><code>secret</code>

```python
def secret(name: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::secret`](../rust/cloacina-python/context/#PyContext.secret)

Read a resolved secret by name, returning a dict of its `{field: value}` pairs (CLOACI-I-0133 / T-0864 — the Python read accessor).

Calls through to the SAME Rust [`cloacina::Context::secret`](../rust/cloacina-workflow/context/#Context.secret) accessor and
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
| `name` | `str` |  |

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

##### <span id="Context.secret_field"></span><code>secret_​field</code>

```python
def secret_field(name: str, field: str) -> str
```

> **Rust Implementation**: [`cloacina-python::context::PyContext::secret_field`](../rust/cloacina-python/context/#PyContext.secret_field)

Read a single field of a named secret (convenience over `Self::secret`).

Same resolver, alias-awareness, no-leak, and error mapping as
`Self::secret`; additionally raises `KeyError` when the secret exists but
has no field of that name.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `field` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn secret_field(&self, py: Python<'_>, name: &str, field: &str) -> PyResult<String> {
        crate::gil::py_block_on(py, self.inner.secret_field(name, field))
            .map_err(secret_access_error_to_pyerr)
    }
```

</details>

### <span id="class.TaskHandle"></span><code style="font-size: 0.85em">class TaskHandle</code>

> **Rust Implementation**: [`cloacina-python::task::PyTaskHandle`](../rust/cloacina-python/task/#struct.PyTaskHandle)

Python wrapper for TaskHandle providing defer_until capability.

#### Methods

##### <span id="TaskHandle.defer_until"></span><code>defer_​until</code>

```python
def defer_until(condition: PyObject, poll_interval_ms: int) -> None
```

> **Rust Implementation**: [`cloacina-python::task::PyTaskHandle::defer_until`](../rust/cloacina-python/task/#PyTaskHandle.defer_until)

Release the concurrency slot while polling an external condition.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `PyObject` |  |
| `poll_interval_ms` | `int` |  |

<details>
<summary>Source</summary>

```rust
    pub fn defer_until(
        &mut self,
        py: Python,
        condition: PyObject,
        poll_interval_ms: u64,
    ) -> PyResult<()> {
        let handle = self
            .inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("TaskHandle has already been consumed"))?;

        let poll_interval = Duration::from_millis(poll_interval_ms);

        // CLOACI-I-0136: one GIL-safe async→sync bridge (was a hand-inlined
        // allow_threads + Handle::current().block_on).
        crate::gil::py_block_on(py, async {
            handle
                .defer_until(
                    move || {
                        let result = Python::with_gil(|py| match condition.call0(py) {
                            Ok(r) => r.extract::<bool>(py).unwrap_or(false),
                            Err(e) => {
                                eprintln!("[cloaca] defer_until condition error: {}", e);
                                false
                            }
                        });
                        async move { result }
                    },
                    poll_interval,
                )
                .await
        })
        .map_err(|e| PyValueError::new_err(format!("defer_until failed: {}", e)))
    }
```

</details>

##### <span id="TaskHandle.is_slot_held"></span><code>is_​slot_​held</code>

```python
def is_slot_held() -> bool
```

> **Rust Implementation**: [`cloacina-python::task::PyTaskHandle::is_slot_held`](../rust/cloacina-python/task/#PyTaskHandle.is_slot_held)

Returns whether the handle currently holds a concurrency slot.

<details>
<summary>Source</summary>

```rust
    pub fn is_slot_held(&self) -> PyResult<bool> {
        let handle = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("TaskHandle has already been consumed"))?;
        Ok(handle.is_slot_held())
    }
```

</details>

### <span id="class.TaskDecorator"></span><code style="font-size: 0.85em">class TaskDecorator</code>

> **Rust Implementation**: [`cloacina-python::task::TaskDecorator`](../rust/cloacina-python/task/#struct.TaskDecorator)

Decorator class that holds task configuration

#### Methods

##### <span id="TaskDecorator.__call__"></span><code>_​_​call_​_​</code>

```python
def __call__(func: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::task::TaskDecorator::__call__`](../rust/cloacina-python/task/#TaskDecorator.__call__)

<details>
<summary>Source</summary>

```rust
    pub fn __call__(&self, py: Python, func: PyObject) -> PyResult<PyObject> {
        let context = current_workflow_context()?;

        let task_id = if let Some(id) = &self.id {
            id.clone()
        } else {
            func.getattr(py, "__name__")?.extract::<String>(py)?
        };

        let has_handle = {
            let code = func.getattr(py, "__code__")?;
            let argcount: usize = code.getattr(py, "co_argcount")?.extract(py)?;
            if argcount >= 2 {
                let varnames: Vec<String> = code.getattr(py, "co_varnames")?.extract(py)?;
                matches!(
                    varnames.get(1).map(|s| s.as_str()),
                    Some("handle" | "task_handle")
                )
            } else {
                false
            }
        };

        let deps = match self.convert_dependencies_to_namespaces(py, &context) {
            Ok(deps) => deps,
            Err(e) => {
                eprintln!("Error converting dependencies: {}", e);
                return Err(e);
            }
        };
        let policy = self.retry_policy.clone();
        let trigger_rules = self.trigger_rules.clone();
        let function = func.clone_ref(py);
        let on_success_cb = self.on_success.as_ref().map(|f| f.clone_ref(py));
        let on_failure_cb = self.on_failure.as_ref().map(|f| f.clone_ref(py));

        // Validate `invokes` / `post_invocation` at decoration time. The
        // referenced graph must already be registered (the ComputationGraphBuilder's
        // `with` block must precede the @task that invokes it) and must be
        // trigger-less. Mirrors Rust T-0540 M3 / M5.
        if self.post_invocation.is_some() && self.invokes.is_none() {
            return Err(PyValueError::new_err(
                "@cloaca.task: `post_invocation` requires `invokes=...` to be set",
            ));
        }
        let cg_invocation = match self.invokes.as_ref() {
            None => None,
            Some(handle) => {
                let bound = handle.bind(py);
                if !bound.hasattr("NAME")? {
                    return Err(PyValueError::new_err(
                        "@cloaca.task(invokes=...): expected a ComputationGraphBuilder \
                         instance (must expose a `NAME` attribute)",
                    ));
                }
                let graph_name: String = bound.getattr("NAME")?.extract()?;
                let executor = crate::computation_graph::get_graph_executor(&graph_name)
                    .ok_or_else(|| {
                        PyValueError::new_err(format!(
                            "@cloaca.task(invokes=...): graph '{}' is not registered. \
                             The `with ComputationGraphBuilder(...)` block must run before \
                             the @cloaca.task decorator that references it.",
                            graph_name
                        ))
                    })?;
                if executor.has_reactor {
                    return Err(PyValueError::new_err(format!(
                        "@cloaca.task(invokes=...): graph '{}' is reactor-triggered. \
                         Tasks may only invoke trigger-less graphs (omit `reactor=...` \
                         on the ComputationGraphBuilder).",
                        graph_name
                    )));
                }
                let _ = executor; // existence verified above; runtime lookup happens at exec
                let post_invocation = self.post_invocation.as_ref().map(|f| f.clone_ref(py));
                Some(CGInvocation {
                    graph_name,
                    post_invocation,
                })
            }
        };
        let cg_invocation_arc = cg_invocation.map(Arc::new);

        let shared_function = Arc::new(function);
        let shared_on_success = on_success_cb.map(Arc::new);
        let shared_on_failure = on_failure_cb.map(Arc::new);
        let (tenant_id, package_name, workflow_id) = context.as_components();
        let namespace =
            cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);

        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err(
                "@task decorator called outside a Runtime scope — install a ScopedRuntime \
                 before importing Python workflow modules",
            )
        })?;
        py.allow_threads(|| {
            rt.register_task(namespace.clone(), {
                let task_id_clone = task_id.clone();
                let deps_clone = deps.clone();
                let policy_clone = policy.clone();
                let function_arc = shared_function.clone();
                let on_success_arc = shared_on_success.clone();
                let on_failure_arc = shared_on_failure.clone();
                let cg_invocation_arc_inner = cg_invocation_arc.clone();
                let trigger_rules_clone = trigger_rules.clone();
                move || {
                    let function_clone = Python::with_gil(|py| function_arc.clone_ref(py));
                    let on_success_clone =
                        Python::with_gil(|py| on_success_arc.as_ref().map(|f| f.clone_ref(py)));
                    let on_failure_clone =
                        Python::with_gil(|py| on_failure_arc.as_ref().map(|f| f.clone_ref(py)));
                    let cg_invocation_clone = cg_invocation_arc_inner.as_ref().map(|cg| {
                        Python::with_gil(|py| CGInvocation {
                            graph_name: cg.graph_name.clone(),
                            post_invocation: cg.post_invocation.as_ref().map(|f| f.clone_ref(py)),
                        })
                    });
                    Arc::new(PythonTaskWrapper {
                        id: task_id_clone.clone(),
                        dependencies: deps_clone.clone(),
                        retry_policy: policy_clone.clone(),
                        python_function: function_clone,
                        on_success_callback: on_success_clone,
                        on_failure_callback: on_failure_clone,
                        requires_handle: has_handle,
                        cg_invocation: cg_invocation_clone,
                        trigger_rules: trigger_rules_clone.clone(),
                    }) as Arc<dyn cloacina::Task>
                }
            });
        });

        Ok(func)
    }
```

</details>

### <span id="class.TriggerResult"></span><code style="font-size: 0.85em">class TriggerResult</code>

> **Rust Implementation**: [`cloacina-python::bindings::trigger::PyTriggerResult`](../rust/cloacina-python/bindings/trigger/#struct.PyTriggerResult)

Python TriggerResult class - represents the result of a trigger poll.

Use `TriggerResult.skip()` when the condition is not met.
Use `TriggerResult.fire(context=None)` when the condition is met.

#### Methods

##### <span id="TriggerResult.skip"></span><code>skip</code>

```python
def skip() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::trigger::PyTriggerResult::skip`](../rust/cloacina-python/bindings/trigger/#PyTriggerResult.skip)

Create a Skip result - condition not met, continue polling.

<details>
<summary>Source</summary>

```rust
    fn skip() -> Self {
        PyTriggerResult {
            is_fire: false,
            data: None,
        }
    }
```

</details>

##### <span id="TriggerResult.fire"></span><code>fire</code>

```python
def fire(context: Optional[PyContext]) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::trigger::PyTriggerResult::fire`](../rust/cloacina-python/bindings/trigger/#PyTriggerResult.fire)

Create a Fire result - condition met, trigger the workflow.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `Optional[PyContext]` | Optional context to pass to the workflow |

<details>
<summary>Source</summary>

```rust
    fn fire(context: Option<&PyContext>) -> Self {
        let data = context.map(|c| c.get_data_clone());
        PyTriggerResult {
            is_fire: true,
            data,
        }
    }
```

</details>

##### <span id="TriggerResult.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::trigger::PyTriggerResult::__repr__`](../rust/cloacina-python/bindings/trigger/#PyTriggerResult.__repr__)

<details>
<summary>Source</summary>

```rust
    fn __repr__(&self) -> String {
        if !self.is_fire {
            "TriggerResult.Skip".to_string()
        } else if self.data.is_none() {
            "TriggerResult.Fire(None)".to_string()
        } else {
            "TriggerResult.Fire(<context>)".to_string()
        }
    }
```

</details>

##### <span id="TriggerResult.is_fire_result"></span><code>is_​fire_​result</code>

```python
def is_fire_result() -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::trigger::PyTriggerResult::is_fire_result`](../rust/cloacina-python/bindings/trigger/#PyTriggerResult.is_fire_result)

Check if this is a Fire result

<details>
<summary>Source</summary>

```rust
    fn is_fire_result(&self) -> bool {
        self.is_fire
    }
```

</details>

##### <span id="TriggerResult.is_skip_result"></span><code>is_​skip_​result</code>

```python
def is_skip_result() -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::trigger::PyTriggerResult::is_skip_result`](../rust/cloacina-python/bindings/trigger/#PyTriggerResult.is_skip_result)

Check if this is a Skip result

<details>
<summary>Source</summary>

```rust
    fn is_skip_result(&self) -> bool {
        !self.is_fire
    }
```

</details>

### <span id="class.TriggerDecorator"></span><code style="font-size: 0.85em">class TriggerDecorator</code>

> **Rust Implementation**: [`cloacina-python::trigger::TriggerDecorator`](../rust/cloacina-python/trigger/#struct.TriggerDecorator)

Decorator for defining Python triggers.

```python
@cloaca.trigger(name="check_inbox", poll_interval="30s")
def check_inbox():
    # Return TriggerResult.fire(context) to fire, .skip() otherwise
    return TriggerResult.skip()
```

#### Methods

##### <span id="TriggerDecorator.__call__"></span><code>_​_​call_​_​</code>

```python
def __call__(func: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger::TriggerDecorator::__call__`](../rust/cloacina-python/trigger/#TriggerDecorator.__call__)

<details>
<summary>Source</summary>

```rust
    pub fn __call__(&self, py: Python, func: PyObject) -> PyResult<PyObject> {
        let trigger_name = if let Some(name) = &self.name {
            name.clone()
        } else {
            func.getattr(py, "__name__")?.extract::<String>(py)?
        };

        let def = PythonTriggerDef {
            name: trigger_name.clone(),
            poll_interval: self.poll_interval,
            allow_concurrent: self.allow_concurrent,
            python_function: func.clone_ref(py),
            workflow_name: self.workflow_name.clone(),
            cron_expression: self.cron_expression.clone(),
            timezone: self.timezone.clone(),
        };

        PYTHON_TRIGGER_REGISTRY.lock().push(def);

        tracing::debug!("Registered Python trigger: {}", trigger_name);

        // Return the original function (decorator is transparent)
        Ok(func)
    }
```

</details>

### <span id="class.WorkflowBuilder"></span><code style="font-size: 0.85em">class WorkflowBuilder</code>

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder`](../rust/cloacina-python/workflow/#struct.PyWorkflowBuilder)

Python wrapper for WorkflowBuilder

#### Methods

##### <span id="WorkflowBuilder.new"></span><code>new</code>

```python
def new(
    name: str,
    tenant: Optional[str],
    package: Optional[str],
    workflow: Optional[str],
) -> Self
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::new`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.new)

Create a new WorkflowBuilder with namespace context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `tenant` | `Optional[str]` |  |
| `package` | `Optional[str]` |  |
| `workflow` | `Optional[str]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn new(
        name: &str,
        tenant: Option<&str>,
        package: Option<&str>,
        workflow: Option<&str>,
    ) -> Self {
        let context = PyWorkflowContext::new(
            tenant.unwrap_or("public"),
            package.unwrap_or("embedded"),
            workflow.unwrap_or(name),
        );

        let (tenant_id, _package_name, _workflow_id) = context.as_components();
        let workflow_builder = cloacina::Workflow::builder(name).tenant(tenant_id);

        PyWorkflowBuilder {
            inner: workflow_builder,
            context,
        }
    }
```

</details>

##### <span id="WorkflowBuilder.description"></span><code>description</code>

```python
def description(description: str)
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::description`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.description)

Set the workflow description

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `description` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn description(&mut self, description: &str) {
        self.inner = self.inner.clone().description(description);
    }
```

</details>

##### <span id="WorkflowBuilder.tag"></span><code>tag</code>

```python
def tag(key: str, value: str)
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::tag`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.tag)

Add a tag to the workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `value` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn tag(&mut self, key: &str, value: &str) {
        self.inner = self.inner.clone().tag(key, value);
    }
```

</details>

##### <span id="WorkflowBuilder.add_task"></span><code>add_​task</code>

```python
def add_task(task: PyObject) -> None
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::add_task`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.add_task)

Add a task to the workflow by ID or function reference

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task` | `PyObject` |  |

<details>
<summary>Source</summary>

```rust
    pub fn add_task(&mut self, py: Python, task: PyObject) -> PyResult<()> {
        match task.extract::<String>(py) {
            Ok(task_id) => {
                let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
                    PyValueError::new_err("WorkflowBuilder.add_task called outside a Runtime scope")
                })?;

                let (tenant_id, package_name, workflow_id) = self.context.as_components();
                let task_namespace =
                    cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);

                let task_instance = rt.get_task(&task_namespace).ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "Task '{}' not found in registry. Make sure it was decorated with @task.",
                        task_id
                    ))
                })?;

                self.inner = self
                    .inner
                    .clone()
                    .add_task(task_instance)
                    .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;

                Ok(())
            }
            _ => match task.bind(py).hasattr("__name__") {
                Ok(true) => match task.getattr(py, "__name__") {
                    Ok(name_obj) => match name_obj.extract::<String>(py) {
                        Ok(func_name) => {
                            let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
                                PyValueError::new_err(
                                    "WorkflowBuilder.add_task called outside a Runtime scope",
                                )
                            })?;

                            let (tenant_id, package_name, workflow_id) =
                                self.context.as_components();
                            let task_namespace = cloacina::TaskNamespace::new(
                                tenant_id,
                                package_name,
                                workflow_id,
                                &func_name,
                            );

                            let task_instance = rt.get_task(&task_namespace).ok_or_else(|| {
                                        PyValueError::new_err(format!(
                                            "Task '{}' not found in registry. Make sure it was decorated with @task.",
                                            func_name
                                        ))
                                    })?;

                            self.inner =
                                self.inner.clone().add_task(task_instance).map_err(|e| {
                                    PyValueError::new_err(format!("Failed to add task: {}", e))
                                })?;

                            Ok(())
                        }
                        Err(e) => Err(PyValueError::new_err(format!(
                            "Function has __name__ but it's not a string: {}",
                            e
                        ))),
                    },
                    Err(e) => Err(PyValueError::new_err(format!(
                        "Failed to get __name__ from function: {}",
                        e
                    ))),
                },
                Ok(false) => Err(PyValueError::new_err(
                    "Task must be either a string task ID or a function object with __name__ attribute",
                )),
                Err(e) => Err(PyValueError::new_err(format!(
                    "Failed to check if object has __name__ attribute: {}",
                    e
                ))),
            },
        }
    }
```

</details>

##### <span id="WorkflowBuilder.build"></span><code>build</code>

```python
def build() -> PyWorkflow
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::build`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.build)

Build the workflow

<details>
<summary>Source</summary>

```rust
    pub fn build(&self) -> PyResult<PyWorkflow> {
        let workflow = self
            .inner
            .clone()
            .build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build workflow: {}", e)))?;
        Ok(PyWorkflow { inner: workflow })
    }
```

</details>

##### <span id="WorkflowBuilder.__enter__"></span><code>_​_​enter_​_​</code>

```python
def __enter__(slf: PyRef<Self>) -> PyRef<Self>
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::__enter__`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.__enter__)

Context manager entry - establish workflow context for task decorators

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef<Self>` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_workflow_context(slf.context.clone());
        slf
    }
```

</details>

##### <span id="WorkflowBuilder.__exit__"></span><code>_​_​exit_​_​</code>

```python
def __exit__(
    _py: Python,
    _exc_type: Optional[Bound<PyAny>],
    _exc_value: Optional[Bound<PyAny>],
    _traceback: Optional[Bound<PyAny>],
) -> bool
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::__exit__`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.__exit__)

Context manager exit - clean up context and build workflow

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_py` | `Python` |  |
| `_exc_type` | `Optional[Bound<PyAny>]` |  |
| `_exc_value` | `Optional[Bound<PyAny>]` |  |
| `_traceback` | `Optional[Bound<PyAny>]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __exit__(
        &mut self,
        _py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        pop_workflow_context();

        let (tenant_id, package_name, workflow_id) = self.context.as_components();

        let mut workflow = cloacina::Workflow::new(workflow_id);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Preserve description and tags set on the builder during the `with` block
        if let Some(desc) = self.inner.get_description() {
            workflow.set_description(desc);
        }
        for (key, value) in self.inner.get_tags() {
            workflow.add_tag(key, value);
        }

        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err("WorkflowBuilder.__exit__ called outside a Runtime scope")
        })?;
        for namespace in rt.task_namespaces() {
            if namespace.tenant_id == tenant_id
                && namespace.package_name == package_name
                && namespace.workflow_id == workflow_id
            {
                if let Some(task_instance) = rt.get_task(&namespace) {
                    workflow
                        .add_task(task_instance)
                        .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;
                }
            }
        }

        workflow
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))?;
        let final_workflow = workflow.finalize();

        let workflow_name = final_workflow.name().to_string();
        rt.register_workflow(workflow_name, move || final_workflow.clone());

        Ok(false)
    }
```

</details>

##### <span id="WorkflowBuilder.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowBuilder::__repr__`](../rust/cloacina-python/workflow/#PyWorkflowBuilder.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!("WorkflowBuilder(name='{}')", self.inner.name())
    }
```

</details>

### <span id="class.Workflow"></span><code style="font-size: 0.85em">class Workflow</code>

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow`](../rust/cloacina-python/workflow/#struct.PyWorkflow)

Python wrapper for Workflow

#### Methods

##### <span id="Workflow.name"></span><code>name</code>

```python
def name() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::name`](../rust/cloacina-python/workflow/#PyWorkflow.name)

Get workflow name

<details>
<summary>Source</summary>

```rust
    pub fn name(&self) -> &str {
        self.inner.name()
    }
```

</details>

##### <span id="Workflow.description"></span><code>description</code>

```python
def description() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::description`](../rust/cloacina-python/workflow/#PyWorkflow.description)

Get workflow description

<details>
<summary>Source</summary>

```rust
    pub fn description(&self) -> String {
        self.inner
            .metadata()
            .description
            .clone()
            .unwrap_or_default()
    }
```

</details>

##### <span id="Workflow.version"></span><code>version</code>

```python
def version() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::version`](../rust/cloacina-python/workflow/#PyWorkflow.version)

Get workflow version

<details>
<summary>Source</summary>

```rust
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }
```

</details>

##### <span id="Workflow.topological_sort"></span><code>topological_​sort</code>

```python
def topological_sort() -> List[str]
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::topological_sort`](../rust/cloacina-python/workflow/#PyWorkflow.topological_sort)

Get topological sort of tasks

<details>
<summary>Source</summary>

```rust
    pub fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner
            .topological_sort()
            .map(|namespaces| namespaces.into_iter().map(|ns| ns.to_string()).collect())
            .map_err(|e| PyValueError::new_err(format!("Failed to sort tasks: {}", e)))
    }
```

</details>

##### <span id="Workflow.get_execution_levels"></span><code>get_​execution_​levels</code>

```python
def get_execution_levels() -> List[List[str]]
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::get_execution_levels`](../rust/cloacina-python/workflow/#PyWorkflow.get_execution_levels)

Get execution levels (tasks that can run in parallel)

<details>
<summary>Source</summary>

```rust
    pub fn get_execution_levels(&self) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .get_execution_levels()
            .map(|levels| {
                levels
                    .into_iter()
                    .map(|level| level.into_iter().map(|ns| ns.to_string()).collect())
                    .collect()
            })
            .map_err(|e| PyValueError::new_err(format!("Failed to get execution levels: {}", e)))
    }
```

</details>

##### <span id="Workflow.get_roots"></span><code>get_​roots</code>

```python
def get_roots() -> List[str]
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::get_roots`](../rust/cloacina-python/workflow/#PyWorkflow.get_roots)

Get root tasks (no dependencies)

<details>
<summary>Source</summary>

```rust
    pub fn get_roots(&self) -> Vec<String> {
        self.inner
            .get_roots()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }
```

</details>

##### <span id="Workflow.get_leaves"></span><code>get_​leaves</code>

```python
def get_leaves() -> List[str]
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::get_leaves`](../rust/cloacina-python/workflow/#PyWorkflow.get_leaves)

Get leaf tasks (no dependents)

<details>
<summary>Source</summary>

```rust
    pub fn get_leaves(&self) -> Vec<String> {
        self.inner
            .get_leaves()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }
```

</details>

##### <span id="Workflow.validate"></span><code>validate</code>

```python
def validate() -> None
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::validate`](../rust/cloacina-python/workflow/#PyWorkflow.validate)

Validate the workflow

<details>
<summary>Source</summary>

```rust
    pub fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))
    }
```

</details>

##### <span id="Workflow.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflow::__repr__`](../rust/cloacina-python/workflow/#PyWorkflow.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "Workflow(name='{}', tasks={})",
            self.inner.name(),
            self.inner.get_task_ids().len()
        )
    }
```

</details>

### <span id="class.WorkflowParamsDecorator"></span><code style="font-size: 0.85em">class WorkflowParamsDecorator</code>

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowParamsDecorator`](../rust/cloacina-python/workflow/#struct.PyWorkflowParamsDecorator)

Pass-through decorator returned by [`workflow_params`](../rust/cloacina-python/workflow/#fn.workflow_params). CLOACI-T-0760: the declaration is parsed from source by the compiler at build time, so at runtime this just returns the decorated object unchanged. It exists only so that importing a package which uses `@cloaca.workflow_params(...)` doesn't fail (the reconciler imports the module via PyO3).

#### Methods

##### <span id="WorkflowParamsDecorator.__call__"></span><code>_​_​call_​_​</code>

```python
def __call__(obj: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowParamsDecorator::__call__`](../rust/cloacina-python/workflow/#PyWorkflowParamsDecorator.__call__)

<details>
<summary>Source</summary>

```rust
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>

### <span id="class.WorkflowSecretsDecorator"></span><code style="font-size: 0.85em">class WorkflowSecretsDecorator</code>

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowSecretsDecorator`](../rust/cloacina-python/workflow/#struct.PyWorkflowSecretsDecorator)

Pass-through decorator returned by [`workflow_secrets`](../rust/cloacina-python/workflow/#fn.workflow_secrets). CLOACI-T-0859: like [`PyWorkflowParamsDecorator`](../rust/cloacina-python/workflow/#struct.PyWorkflowParamsDecorator), the declaration is parsed from source by the compiler at build time into encrypted `InputSlot`s; at runtime it is a no-op pass-through so importing a package that uses `@cloaca.workflow_secrets(...)` doesn't fail.

#### Methods

##### <span id="WorkflowSecretsDecorator.__call__"></span><code>_​_​call_​_​</code>

```python
def __call__(obj: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::workflow::PyWorkflowSecretsDecorator::__call__`](../rust/cloacina-python/workflow/#PyWorkflowSecretsDecorator.__call__)

<details>
<summary>Source</summary>

```rust
    fn __call__(&self, obj: PyObject) -> PyObject {
        obj
    }
```

</details>

### <span id="class.TaskNamespace"></span><code style="font-size: 0.85em">class TaskNamespace</code>

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace`](../rust/cloacina-python/namespace/#struct.PyTaskNamespace)

Python wrapper for TaskNamespace

#### Methods

##### <span id="TaskNamespace.new"></span><code>new</code>

```python
def new(tenant_id: str, package_name: str, workflow_id: str, task_id: str) -> Self
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::new`](../rust/cloacina-python/namespace/#PyTaskNamespace.new)

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

```rust
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str, task_id: &str) -> Self {
        Self {
            inner: cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, task_id),
        }
    }
```

</details>

##### <span id="TaskNamespace.from_string"></span><code>from_​string</code>

```python
def from_string(namespace_str: str) -> Self
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::from_string`](../rust/cloacina-python/namespace/#PyTaskNamespace.from_string)

Parse TaskNamespace from string format "tenant::package::workflow::task"

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace_str` | `str` |  |

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

##### <span id="TaskNamespace.tenant_id"></span><code>tenant_​id</code>

```python
def tenant_id() -> str
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::tenant_id`](../rust/cloacina-python/namespace/#PyTaskNamespace.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.inner.tenant_id
    }
```

</details>

##### <span id="TaskNamespace.package_name"></span><code>package_​name</code>

```python
def package_name() -> str
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::package_name`](../rust/cloacina-python/namespace/#PyTaskNamespace.package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.inner.package_name
    }
```

</details>

##### <span id="TaskNamespace.workflow_id"></span><code>workflow_​id</code>

```python
def workflow_id() -> str
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::workflow_id`](../rust/cloacina-python/namespace/#PyTaskNamespace.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.inner.workflow_id
    }
```

</details>

##### <span id="TaskNamespace.task_id"></span><code>task_​id</code>

```python
def task_id() -> str
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::task_id`](../rust/cloacina-python/namespace/#PyTaskNamespace.task_id)

Get task ID

<details>
<summary>Source</summary>

```rust
    pub fn task_id(&self) -> &str {
        &self.inner.task_id
    }
```

</details>

##### <span id="TaskNamespace.parent"></span><code>parent</code>

```python
def parent() -> Self
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::parent`](../rust/cloacina-python/namespace/#PyTaskNamespace.parent)

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

##### <span id="TaskNamespace.is_child_of"></span><code>is_​child_​of</code>

```python
def is_child_of(parent: PyTaskNamespace) -> bool
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::is_child_of`](../rust/cloacina-python/namespace/#PyTaskNamespace.is_child_of)

Check if this namespace is a child of another

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `parent` | `PyTaskNamespace` |  |

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

##### <span id="TaskNamespace.is_sibling_of"></span><code>is_​sibling_​of</code>

```python
def is_sibling_of(other: PyTaskNamespace) -> bool
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::is_sibling_of`](../rust/cloacina-python/namespace/#PyTaskNamespace.is_sibling_of)

Check if this namespace is a sibling of another (same parent)

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `PyTaskNamespace` |  |

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

##### <span id="TaskNamespace.__str__"></span><code>_​_​str_​_​</code>

```python
def __str__() -> str
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::__str__`](../rust/cloacina-python/namespace/#PyTaskNamespace.__str__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __str__(&self) -> String {
        self.inner.to_string()
    }
```

</details>

##### <span id="TaskNamespace.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::__repr__`](../rust/cloacina-python/namespace/#PyTaskNamespace.__repr__)

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

##### <span id="TaskNamespace.__eq__"></span><code>_​_​eq_​_​</code>

```python
def __eq__(other: PyTaskNamespace) -> bool
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::__eq__`](../rust/cloacina-python/namespace/#PyTaskNamespace.__eq__)

Equality comparison

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `PyTaskNamespace` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __eq__(&self, other: &PyTaskNamespace) -> bool {
        self.inner == other.inner
    }
```

</details>

##### <span id="TaskNamespace.__hash__"></span><code>_​_​hash_​_​</code>

```python
def __hash__() -> int
```

> **Rust Implementation**: [`cloacina-python::namespace::PyTaskNamespace::__hash__`](../rust/cloacina-python/namespace/#PyTaskNamespace.__hash__)

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

### <span id="class.WorkflowContext"></span><code style="font-size: 0.85em">class WorkflowContext</code>

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext`](../rust/cloacina-python/workflow_context/#struct.PyWorkflowContext)

WorkflowContext provides namespace management for Python workflows

#### Methods

##### <span id="WorkflowContext.new"></span><code>new</code>

```python
def new(tenant_id: str, package_name: str, workflow_id: str) -> Self
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::new`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.new)

Create a new WorkflowContext

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant_id` | `str` |  |
| `package_name` | `str` |  |
| `workflow_id` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn new(tenant_id: &str, package_name: &str, workflow_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            package_name: package_name.to_string(),
            workflow_id: workflow_id.to_string(),
        }
    }
```

</details>

##### <span id="WorkflowContext.tenant_id"></span><code>tenant_​id</code>

```python
def tenant_id() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::tenant_id`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.tenant_id)

Get tenant ID

<details>
<summary>Source</summary>

```rust
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
```

</details>

##### <span id="WorkflowContext.package_name"></span><code>package_​name</code>

```python
def package_name() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::package_name`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.package_name)

Get package name

<details>
<summary>Source</summary>

```rust
    pub fn package_name(&self) -> &str {
        &self.package_name
    }
```

</details>

##### <span id="WorkflowContext.workflow_id"></span><code>workflow_​id</code>

```python
def workflow_id() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::workflow_id`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.workflow_id)

Get workflow ID

<details>
<summary>Source</summary>

```rust
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
```

</details>

##### <span id="WorkflowContext.task_namespace"></span><code>task_​namespace</code>

```python
def task_namespace(task_id: str) -> PyTaskNamespace
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::task_namespace`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.task_namespace)

Generate a TaskNamespace for a task within this workflow context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_id` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn task_namespace(&self, task_id: &str) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(cloacina::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            task_id,
        ))
    }
```

</details>

##### <span id="WorkflowContext.resolve_dependency"></span><code>resolve_​dependency</code>

```python
def resolve_dependency(task_name: str) -> PyTaskNamespace
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::resolve_dependency`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.resolve_dependency)

Resolve a dependency task name to a full TaskNamespace within this context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `task_name` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn resolve_dependency(&self, task_name: &str) -> PyTaskNamespace {
        self.task_namespace(task_name)
    }
```

</details>

##### <span id="WorkflowContext.workflow_namespace"></span><code>workflow_​namespace</code>

```python
def workflow_namespace() -> PyTaskNamespace
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::workflow_namespace`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.workflow_namespace)

Get the workflow namespace (without task_id)

<details>
<summary>Source</summary>

```rust
    pub fn workflow_namespace(&self) -> PyTaskNamespace {
        PyTaskNamespace::from_rust(cloacina::TaskNamespace::new(
            &self.tenant_id,
            &self.package_name,
            &self.workflow_id,
            "",
        ))
    }
```

</details>

##### <span id="WorkflowContext.contains_namespace"></span><code>contains_​namespace</code>

```python
def contains_namespace(namespace: PyTaskNamespace) -> bool
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::contains_namespace`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.contains_namespace)

Check if a namespace belongs to this workflow context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `namespace` | `PyTaskNamespace` |  |

<details>
<summary>Source</summary>

```rust
    pub fn contains_namespace(&self, namespace: &PyTaskNamespace) -> bool {
        namespace.tenant_id() == self.tenant_id
            && namespace.package_name() == self.package_name
            && namespace.workflow_id() == self.workflow_id
    }
```

</details>

##### <span id="WorkflowContext.__str__"></span><code>_​_​str_​_​</code>

```python
def __str__() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::__str__`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.__str__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __str__(&self) -> String {
        format!(
            "{}::{}::{}",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }
```

</details>

##### <span id="WorkflowContext.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::__repr__`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowContext('{}', '{}', '{}')",
            self.tenant_id, self.package_name, self.workflow_id
        )
    }
```

</details>

##### <span id="WorkflowContext.__eq__"></span><code>_​_​eq_​_​</code>

```python
def __eq__(other: PyWorkflowContext) -> bool
```

> **Rust Implementation**: [`cloacina-python::workflow_context::PyWorkflowContext::__eq__`](../rust/cloacina-python/workflow_context/#PyWorkflowContext.__eq__)

Equality comparison

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `other` | `PyWorkflowContext` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __eq__(&self, other: &PyWorkflowContext) -> bool {
        self.tenant_id == other.tenant_id
            && self.package_name == other.package_name
            && self.workflow_id == other.workflow_id
    }
```

</details>

### <span id="class.RetryPolicy"></span><code style="font-size: 0.85em">class RetryPolicy</code>

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy`](../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryPolicy)

Python wrapper for RetryPolicy

#### Methods

##### <span id="RetryPolicy.builder"></span><code>builder</code>

```python
def builder() -> PyRetryPolicyBuilder
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::builder`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.builder)

Create a builder for constructing RetryPolicy

<details>
<summary>Source</summary>

```rust
    pub fn builder() -> PyRetryPolicyBuilder {
        PyRetryPolicyBuilder {
            max_attempts: None,
            backoff_strategy: None,
            initial_delay: None,
            max_delay: None,
            retry_condition: None,
            with_jitter: None,
        }
    }
```

</details>

##### <span id="RetryPolicy.default"></span><code>default</code>

```python
def default() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::default`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.default)

Create a default RetryPolicy

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        Self {
            inner: cloacina::retry::RetryPolicy::default(),
        }
    }
```

</details>

##### <span id="RetryPolicy.should_retry"></span><code>should_​retry</code>

```python
def should_retry(attempt: int, _error_type: str) -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::should_retry`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.should_retry)

Check if a retry should be attempted

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `int` |  |
| `_error_type` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn should_retry(&self, attempt: i32, _error_type: &str) -> bool {
        // For now, use a simple retry condition check
        // In the future, this could be enhanced to parse error_type
        attempt < self.inner.max_attempts
    }
```

</details>

##### <span id="RetryPolicy.calculate_delay"></span><code>calculate_​delay</code>

```python
def calculate_delay(attempt: int) -> float
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::calculate_delay`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.calculate_delay)

Calculate delay for a given attempt

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempt` | `int` |  |

<details>
<summary>Source</summary>

```rust
    pub fn calculate_delay(&self, attempt: i32) -> f64 {
        let duration = self.inner.calculate_delay(attempt);
        duration.as_secs_f64()
    }
```

</details>

##### <span id="RetryPolicy.max_attempts"></span><code>max_​attempts</code>

```python
def max_attempts() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::max_attempts`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.max_attempts)

Get maximum number of attempts

<details>
<summary>Source</summary>

```rust
    pub fn max_attempts(&self) -> i32 {
        self.inner.max_attempts
    }
```

</details>

##### <span id="RetryPolicy.initial_delay"></span><code>initial_​delay</code>

```python
def initial_delay() -> float
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::initial_delay`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.initial_delay)

Get initial delay in seconds

<details>
<summary>Source</summary>

```rust
    pub fn initial_delay(&self) -> f64 {
        self.inner.initial_delay.as_secs_f64()
    }
```

</details>

##### <span id="RetryPolicy.max_delay"></span><code>max_​delay</code>

```python
def max_delay() -> float
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::max_delay`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.max_delay)

Get maximum delay in seconds

<details>
<summary>Source</summary>

```rust
    pub fn max_delay(&self) -> f64 {
        self.inner.max_delay.as_secs_f64()
    }
```

</details>

##### <span id="RetryPolicy.with_jitter"></span><code>with_​jitter</code>

```python
def with_jitter() -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::with_jitter`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.with_jitter)

Check if jitter is enabled

<details>
<summary>Source</summary>

```rust
    pub fn with_jitter(&self) -> bool {
        self.inner.jitter
    }
```

</details>

##### <span id="RetryPolicy.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicy::__repr__`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicy.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "RetryPolicy(max_attempts={}, initial_delay={}s, max_delay={}s, jitter={})",
            self.max_attempts(),
            self.initial_delay(),
            self.max_delay(),
            self.with_jitter()
        )
    }
```

</details>

### <span id="class.RetryPolicyBuilder"></span><code style="font-size: 0.85em">class RetryPolicyBuilder</code>

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder`](../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryPolicyBuilder)

Python wrapper for RetryPolicy::Builder

#### Methods

##### <span id="RetryPolicyBuilder.max_attempts"></span><code>max_​attempts</code>

```python
def max_attempts(attempts: int) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::max_attempts`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.max_attempts)

Set maximum number of retry attempts

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `attempts` | `int` |  |

<details>
<summary>Source</summary>

```rust
    pub fn max_attempts(&self, attempts: i32) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_attempts = Some(attempts);
        new_builder
    }
```

</details>

##### <span id="RetryPolicyBuilder.initial_delay"></span><code>initial_​delay</code>

```python
def initial_delay(delay_seconds: float) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::initial_delay`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.initial_delay)

Set initial delay

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `delay_seconds` | `float` |  |

<details>
<summary>Source</summary>

```rust
    pub fn initial_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.initial_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }
```

</details>

##### <span id="RetryPolicyBuilder.max_delay"></span><code>max_​delay</code>

```python
def max_delay(delay_seconds: float) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::max_delay`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.max_delay)

Set maximum delay

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `delay_seconds` | `float` |  |

<details>
<summary>Source</summary>

```rust
    pub fn max_delay(&self, delay_seconds: f64) -> Self {
        let mut new_builder = self.clone();
        new_builder.max_delay = Some(Duration::from_secs_f64(delay_seconds));
        new_builder
    }
```

</details>

##### <span id="RetryPolicyBuilder.backoff_strategy"></span><code>backoff_​strategy</code>

```python
def backoff_strategy(strategy: PyBackoffStrategy) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::backoff_strategy`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.backoff_strategy)

Set backoff strategy

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `strategy` | `PyBackoffStrategy` |  |

<details>
<summary>Source</summary>

```rust
    pub fn backoff_strategy(&self, strategy: PyBackoffStrategy) -> Self {
        let mut new_builder = self.clone();
        new_builder.backoff_strategy = Some(strategy.inner);
        new_builder
    }
```

</details>

##### <span id="RetryPolicyBuilder.retry_condition"></span><code>retry_​condition</code>

```python
def retry_condition(condition: PyRetryCondition) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::retry_condition`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.retry_condition)

Set retry condition

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `condition` | `PyRetryCondition` |  |

<details>
<summary>Source</summary>

```rust
    pub fn retry_condition(&self, condition: PyRetryCondition) -> Self {
        let mut new_builder = self.clone();
        new_builder.retry_condition = Some(condition.inner);
        new_builder
    }
```

</details>

##### <span id="RetryPolicyBuilder.with_jitter"></span><code>with_​jitter</code>

```python
def with_jitter(jitter: bool) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::with_jitter`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.with_jitter)

Enable/disable jitter

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `jitter` | `bool` |  |

<details>
<summary>Source</summary>

```rust
    pub fn with_jitter(&self, jitter: bool) -> Self {
        let mut new_builder = self.clone();
        new_builder.with_jitter = Some(jitter);
        new_builder
    }
```

</details>

##### <span id="RetryPolicyBuilder.build"></span><code>build</code>

```python
def build() -> PyRetryPolicy
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryPolicyBuilder::build`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryPolicyBuilder.build)

Build the RetryPolicy

<details>
<summary>Source</summary>

```rust
    pub fn build(&self) -> PyRetryPolicy {
        let mut builder = cloacina::retry::RetryPolicy::builder();

        if let Some(attempts) = self.max_attempts {
            builder = builder.max_attempts(attempts);
        }
        if let Some(strategy) = &self.backoff_strategy {
            builder = builder.backoff_strategy(strategy.clone());
        }
        if let Some(delay) = self.initial_delay {
            builder = builder.initial_delay(delay);
        }
        if let Some(delay) = self.max_delay {
            builder = builder.max_delay(delay);
        }
        if let Some(condition) = &self.retry_condition {
            builder = builder.retry_condition(condition.clone());
        }
        if let Some(jitter) = self.with_jitter {
            builder = builder.with_jitter(jitter);
        }

        PyRetryPolicy {
            inner: builder.build(),
        }
    }
```

</details>

### <span id="class.BackoffStrategy"></span><code style="font-size: 0.85em">class BackoffStrategy</code>

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyBackoffStrategy`](../rust/cloacina-python/bindings/value_objects/retry/#struct.PyBackoffStrategy)

Python wrapper for BackoffStrategy

#### Methods

##### <span id="BackoffStrategy.fixed"></span><code>fixed</code>

```python
def fixed() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::fixed`](../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.fixed)

Fixed delay strategy

<details>
<summary>Source</summary>

```rust
    pub fn fixed() -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Fixed,
        }
    }
```

</details>

##### <span id="BackoffStrategy.linear"></span><code>linear</code>

```python
def linear(multiplier: float) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::linear`](../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.linear)

Linear backoff strategy

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `multiplier` | `float` |  |

<details>
<summary>Source</summary>

```rust
    pub fn linear(multiplier: f64) -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Linear { multiplier },
        }
    }
```

</details>

##### <span id="BackoffStrategy.exponential"></span><code>exponential</code>

```python
def exponential(base: float, multiplier: Optional[float]) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::exponential`](../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.exponential)

Exponential backoff strategy

CLOACI-T-0882: `multiplier` is `Option` and defaults to 1.0 via
`unwrap_or` below, but pyo3 does NOT infer optionality from `Option<T>`
— without an explicit `signature`, the argument stayed REQUIRED and
`BackoffStrategy.exponential(2.0)` raised
`TypeError: missing 1 required positional argument: 'multiplier'`.
Nothing had ever constructed one of these from Python, so the gap
between the declared default and the callable signature went unseen.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `base` | `float` |  |
| `multiplier` | `Optional[float]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn exponential(base: f64, multiplier: Option<f64>) -> Self {
        Self {
            inner: cloacina::retry::BackoffStrategy::Exponential {
                base,
                multiplier: multiplier.unwrap_or(1.0),
            },
        }
    }
```

</details>

##### <span id="BackoffStrategy.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyBackoffStrategy::__repr__`](../rust/cloacina-python/bindings/value_objects/retry/#PyBackoffStrategy.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        match &self.inner {
            cloacina::retry::BackoffStrategy::Fixed => "BackoffStrategy.Fixed".to_string(),
            cloacina::retry::BackoffStrategy::Linear { multiplier } => {
                format!("BackoffStrategy.Linear(multiplier={})", multiplier)
            }
            cloacina::retry::BackoffStrategy::Exponential { base, multiplier } => {
                format!(
                    "BackoffStrategy.Exponential(base={}, multiplier={})",
                    base, multiplier
                )
            }
            cloacina::retry::BackoffStrategy::Custom { function_name } => {
                format!("BackoffStrategy.Custom(function_name='{}')", function_name)
            }
        }
    }
```

</details>

### <span id="class.RetryCondition"></span><code style="font-size: 0.85em">class RetryCondition</code>

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryCondition`](../rust/cloacina-python/bindings/value_objects/retry/#struct.PyRetryCondition)

Python wrapper for RetryCondition

#### Methods

##### <span id="RetryCondition.never"></span><code>never</code>

```python
def never() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryCondition::never`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.never)

Never retry

<details>
<summary>Source</summary>

```rust
    pub fn never() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::Never,
        }
    }
```

</details>

##### <span id="RetryCondition.transient_only"></span><code>transient_​only</code>

```python
def transient_only() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryCondition::transient_only`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.transient_only)

Retry only on transient errors

<details>
<summary>Source</summary>

```rust
    pub fn transient_only() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::TransientOnly,
        }
    }
```

</details>

##### <span id="RetryCondition.all_errors"></span><code>all_​errors</code>

```python
def all_errors() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryCondition::all_errors`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.all_errors)

Retry on all errors

<details>
<summary>Source</summary>

```rust
    pub fn all_errors() -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::AllErrors,
        }
    }
```

</details>

##### <span id="RetryCondition.error_pattern"></span><code>error_​pattern</code>

```python
def error_pattern(patterns: List[str]) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryCondition::error_pattern`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.error_pattern)

Retry on specific error patterns

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `patterns` | `List[str]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn error_pattern(patterns: Vec<String>) -> Self {
        Self {
            inner: cloacina::retry::RetryCondition::ErrorPattern { patterns },
        }
    }
```

</details>

##### <span id="RetryCondition.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::value_objects::retry::PyRetryCondition::__repr__`](../rust/cloacina-python/bindings/value_objects/retry/#PyRetryCondition.__repr__)

String representation

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        match &self.inner {
            cloacina::retry::RetryCondition::Never => "RetryCondition.Never".to_string(),
            cloacina::retry::RetryCondition::TransientOnly => {
                "RetryCondition.TransientOnly".to_string()
            }
            cloacina::retry::RetryCondition::AllErrors => "RetryCondition.AllErrors".to_string(),
            cloacina::retry::RetryCondition::ErrorPattern { patterns } => {
                format!("RetryCondition.ErrorPattern(patterns={:?})", patterns)
            }
        }
    }
```

</details>

### <span id="class.ComputationGraphBuilder"></span><code style="font-size: 0.85em">class ComputationGraphBuilder</code>

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder`](../rust/cloacina-python/computation_graph/#struct.PyComputationGraphBuilder)

#### Methods

##### <span id="ComputationGraphBuilder.new"></span><code>new</code>

```python
def new(
    _py: Python<'_>,
    name: str,
    graph: Bound<'_, PyDict>,
    reactor: Optional[Bound<'_, PyAny>],
    react: Optional[Bound<'_, PyAny>],
) -> Self
```

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder::new`](../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.new)

<details>
<summary>Source</summary>

```rust
    pub fn new(
        _py: Python<'_>,
        name: &str,
        graph: &Bound<'_, PyDict>,
        reactor: Option<&Bound<'_, PyAny>>,
        react: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Hard-break migration: bundled `react={...}` form is gone — point users
        // at I-0101 with a clear error.
        if react.is_some() {
            return Err(PyValueError::new_err(
                "ComputationGraphBuilder no longer accepts the bundled `react={...}` kwarg. \
                 Declare a `@cloaca.reactor` class and pass `reactor=YourReactor`, \
                 or omit `reactor` entirely for a trigger-less graph invoked by \
                 `@cloaca.task(invokes=...)`. See initiative CLOACI-I-0101.",
            ));
        }

        let reactor_binding = match reactor {
            None => None,
            Some(r) => Some(extract_reactor_binding(r)?),
        };

        let nodes_decl = parse_graph_dict(graph)?;

        Ok(PyComputationGraphBuilder {
            name: name.to_string(),
            reactor_binding,
            nodes_decl,
        })
    }
```

</details>

##### <span id="ComputationGraphBuilder.NAME"></span><code>NAME</code>

```python
def NAME() -> str
```

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder::NAME`](../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.NAME)

The graph's declared name. Mirrors Rust's `Graph::NAME` so this instance can serve as the handle referenced by `@cloaca.task(invokes=...)`.

<details>
<summary>Source</summary>

```rust
    pub fn NAME(&self) -> String {
        self.name.clone()
    }
```

</details>

##### <span id="ComputationGraphBuilder.__enter__"></span><code>_​_​enter_​_​</code>

```python
def __enter__(slf: PyRef<Self>) -> PyRef<Self>
```

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder::__enter__`](../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.__enter__)

Context manager entry — establish graph context for @node decorators

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef<Self>` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_graph_context(slf.name.clone());
        slf
    }
```

</details>

##### <span id="ComputationGraphBuilder.__exit__"></span><code>_​_​exit_​_​</code>

```python
def __exit__(
    _exc_type: Optional[Bound<PyAny>],
    _exc_value: Optional[Bound<PyAny>],
    _traceback: Optional[Bound<PyAny>],
) -> bool
```

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder::__exit__`](../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.__exit__)

Context manager exit — validate nodes against topology, build executor

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_exc_type` | `Optional[Bound<PyAny>]` |  |
| `_exc_value` | `Optional[Bound<PyAny>]` |  |
| `_traceback` | `Optional[Bound<PyAny>]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn __exit__(
        &self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        pop_graph_context();

        let registered_nodes = drain_nodes();

        // Validate: every node in topology must have a registered function
        for decl in &self.nodes_decl {
            if !registered_nodes.contains_key(&decl.name) {
                return Err(PyAttributeError::new_err(format!(
                    "computation graph '{}' topology references node '{}' but no @cloaca.node function with that name was defined",
                    self.name, decl.name
                )));
            }
        }

        // Validate: every registered function must appear in topology
        for fn_name in registered_nodes.keys() {
            if !self.nodes_decl.iter().any(|d| d.name == *fn_name) {
                return Err(PyValueError::new_err(format!(
                    "function '{}' was decorated with @cloaca.node but does not appear in the graph topology",
                    fn_name
                )));
            }
        }

        // Trigger-less entry contract: in trigger-less form, no node may
        // declare cache `inputs=[...]` — those inputs only exist when a
        // reactor is feeding the graph. Trigger-less graphs receive their
        // input via the task `Context` passed by `@cloaca.task(invokes=...)`.
        if self.reactor_binding.is_none() {
            for decl in &self.nodes_decl {
                if !decl.cache_inputs.is_empty() {
                    return Err(PyValueError::new_err(format!(
                        "trigger-less computation graph '{}': node '{}' declares cache \
                         inputs={:?}, but trigger-less graphs have no accumulator inputs. \
                         Either remove the inputs (graph receives data through the task \
                         context) or bind a reactor with `reactor=YourReactor`.",
                        self.name, decl.name, decl.cache_inputs
                    )));
                }
            }
        } else if let Some(ref binding) = self.reactor_binding {
            // Split-form sanity: every cache input declared by a node must be
            // one of the reactor's accumulators. Catches typos at registration
            // time rather than producing silent `None`s at execution time.
            for decl in &self.nodes_decl {
                for input in &decl.cache_inputs {
                    if !binding.accumulators.contains(input) {
                        return Err(PyValueError::new_err(format!(
                            "computation graph '{}': node '{}' references cache input \
                             '{}' which is not in reactor '{}' accumulators {:?}",
                            self.name, decl.name, input, binding.name, binding.accumulators
                        )));
                    }
                }
            }
        }

        // Build the executor
        let node_map: HashMap<String, PyNodeDecl> = self
            .nodes_decl
            .iter()
            .cloned()
            .map(|n| (n.name.clone(), n))
            .collect();
        let execution_order = compute_execution_order(&self.nodes_decl);

        let (react_mode, accumulators, reactor_name) = match &self.reactor_binding {
            Some(b) => (b.mode.clone(), b.accumulators.clone(), Some(b.name.clone())),
            None => (String::new(), Vec::new(), None),
        };

        let executor = PythonGraphExecutor {
            name: self.name.clone(),
            node_functions: registered_nodes,
            node_map,
            execution_order,
            react_mode,
            accumulators,
            has_reactor: self.reactor_binding.is_some(),
            reactor_name,
        };

        // Register the executor globally (similar to workflow registration)
        register_graph_executor(self.name.clone(), executor, py)?;

        Ok(false)
    }
```

</details>

##### <span id="ComputationGraphBuilder.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder::__repr__`](../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "ComputationGraphBuilder(name='{}', nodes={})",
            self.name,
            self.nodes_decl.len()
        )
    }
```

</details>

##### <span id="ComputationGraphBuilder.execute"></span><code>execute</code>

```python
def execute(inputs: Bound<'_, PyDict>) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::PyComputationGraphBuilder::execute`](../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.execute)

Execute the computation graph with the given input cache.

`inputs` is a dict mapping source names to their data dicts.
Returns the terminal node's output dict.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `inputs` | `Bound<'_, PyDict>` |  |

<details>
<summary>Source</summary>

```rust
    pub fn execute(&self, py: Python<'_>, inputs: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let executor = get_graph_executor(&self.name).ok_or_else(|| {
            PyValueError::new_err(format!(
                "graph '{}' not built yet — call execute after the 'with' block exits",
                self.name
            ))
        })?;

        // Convert Python dict inputs → HashMap<String, PyObject>
        let mut cache: HashMap<String, PyObject> = HashMap::new();
        for (key, value) in inputs.iter() {
            let key_str: String = key.extract()?;
            cache.insert(key_str, value.unbind());
        }

        // Execute synchronously (this is a simplified path for tutorials)
        let result = executor.execute_sync(py, &cache)?;
        Ok(result)
    }
```

</details>

### <span id="class.DefaultRunner"></span><code style="font-size: 0.85em">class DefaultRunner</code>

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner`](../rust/cloacina-python/bindings/runner/#struct.PyDefaultRunner)

Python wrapper for DefaultRunner

#### Methods

##### <span id="DefaultRunner.new"></span><code>new</code>

```python
def new(database_url: str) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::new`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.new)

Create a new DefaultRunner with database connection

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn new(py: Python<'_>, database_url: &str) -> PyResult<Self> {
        let database_url = database_url.to_string();
        // Share the caller's ScopedRuntime (installed by the cloaca pymodule
        // init, and where `@task`/`WorkflowBuilder` register) so the runner's
        // scheduler can find workflows registered before the runner existed.
        let rt_arc = crate::runtime_scope::current_runtime();
        spawn_runtime(py, move |rt| {
            info!(
                "Creating DefaultRunner with database_url: {}",
                cloacina::logging::mask_db_url(&database_url)
            );
            let mut builder = cloacina::DefaultRunner::builder().database_url(&database_url);
            if let Some(arc) = rt_arc {
                builder = builder.runtime_arc(arc);
            }
            rt.block_on(builder.build())
        })
    }
```

</details>

##### <span id="DefaultRunner.with_config"></span><code>with_​config</code>

```python
def with_config(
    database_url: str,
    config: super::context::PyDefaultRunnerConfig,
) -> PyDefaultRunner
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::with_config`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.with_config)

Create a new DefaultRunner with custom configuration

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `str` |  |
| `config` | `super::context::PyDefaultRunnerConfig` |  |

<details>
<summary>Source</summary>

```rust
    pub fn with_config(
        py: Python<'_>,
        database_url: &str,
        config: &super::context::PyDefaultRunnerConfig,
    ) -> PyResult<PyDefaultRunner> {
        let database_url = database_url.to_string();
        let rust_config = config.to_rust_config();
        let rt_arc = crate::runtime_scope::current_runtime();
        spawn_runtime(py, move |rt| {
            let mut builder = cloacina::DefaultRunner::builder()
                .database_url(&database_url)
                .with_config(rust_config);
            if let Some(arc) = rt_arc {
                builder = builder.runtime_arc(arc);
            }
            rt.block_on(builder.build())
        })
    }
```

</details>

##### <span id="DefaultRunner.with_schema"></span><code>with_​schema</code>

```python
def with_schema(database_url: str, schema: str) -> PyDefaultRunner
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::with_schema`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.with_schema)

Create a new DefaultRunner with PostgreSQL schema-based multi-tenancy

Each tenant gets their own schema with independent tables, migrations,
and data. Requires a PostgreSQL database.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `database_url` | `str` | PostgreSQL connection string |
| `schema` | `str` | Schema name for tenant isolation (alphanumeric + underscores only) |

<details>
<summary>Source</summary>

```rust
    pub fn with_schema(
        py: Python<'_>,
        database_url: &str,
        schema: &str,
    ) -> PyResult<PyDefaultRunner> {
        if !database_url.starts_with("postgres://") && !database_url.starts_with("postgresql://") {
            return Err(PyValueError::new_err(
                "Schema-based multi-tenancy requires PostgreSQL. \
                 SQLite does not support database schemas. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }
        if schema.is_empty() {
            return Err(PyValueError::new_err("Schema name cannot be empty"));
        }
        if !schema.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(PyValueError::new_err(
                "Schema name must contain only alphanumeric characters and underscores",
            ));
        }

        let database_url = database_url.to_string();
        let schema = schema.to_string();
        let rt_arc = crate::runtime_scope::current_runtime();
        spawn_runtime(py, move |rt| {
            info!(
                "Creating DefaultRunner with schema: {} and database_url: {}",
                schema,
                cloacina::logging::mask_db_url(&database_url)
            );
            let mut builder = cloacina::DefaultRunner::builder()
                .database_url(&database_url)
                .schema(&schema);
            if let Some(arc) = rt_arc {
                builder = builder.runtime_arc(arc);
            }
            rt.block_on(builder.build())
        })
    }
```

</details>

##### <span id="DefaultRunner.execute"></span><code>execute</code>

```python
def execute(workflow_name: str, context: PyContext) -> PyWorkflowResult
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::execute`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.execute)

Execute a workflow by name with context

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `str` |  |
| `context` | `PyContext` |  |

<details>
<summary>Source</summary>

```rust
    pub fn execute(
        &self,
        workflow_name: &str,
        context: &PyContext,
        py: Python,
    ) -> PyResult<PyWorkflowResult> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::Execute {
            workflow_name: workflow_name.to_string(),
            context: context.clone_inner(),
            response_tx,
        };
        let result = self.send_and_recv(message, response_rx, py, "Workflow execution failed")?;
        Ok(PyWorkflowResult::from_result(result))
    }
```

</details>

##### <span id="DefaultRunner.shutdown"></span><code>shutdown</code>

```python
def shutdown() -> None
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::shutdown`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.shutdown)

Shutdown the runner and cleanup resources

<details>
<summary>Source</summary>

```rust
    pub fn shutdown(&self, py: Python) -> PyResult<()> {
        info!("Starting async runtime shutdown from Python");
        let result = py.allow_threads(|| self.runtime_handle.lock().unwrap().shutdown());
        match result {
            Ok(()) => {
                info!("Async runtime shutdown completed successfully");
                Ok(())
            }
            Err(e) => {
                error!("Async runtime shutdown failed: {}", e);
                Err(PyValueError::new_err(format!(
                    "Failed to shutdown async runtime: {}",
                    e
                )))
            }
        }
    }
```

</details>

##### <span id="DefaultRunner.register_workflow_instance"></span><code>register_​workflow_​instance</code>

```python
def register_workflow_instance(
    workflow_name: str,
    instance_name: str,
    cron_expression: str,
    timezone: str,
    params: crate::context::PyContext,
) -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::register_workflow_instance`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.register_workflow_instance)

Register a cron workflow for automatic execution at scheduled times

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow_name` | `str` | Name of the workflow to execute |
| `instance_name` | `str` |  |
| `cron_expression` | `str` | Standard cron expression (e.g., "0 2 * * *") |
| `timezone` | `str` | Timezone for cron interpretation (e.g., "UTC") |
| `params` | `crate::context::PyContext` |  |

**Returns:**

* Schedule ID as a string
Register a NAMED, PARAMETERIZED cron instance (CLOACI-I-0116):
`params` (a dict) is the instance's fully-resolved bound parameters,
merged into the context at every fire under flat top-level keys.

<details>
<summary>Source</summary>

```rust
    pub fn register_workflow_instance(
        &self,
        workflow_name: String,
        instance_name: String,
        cron_expression: String,
        timezone: String,
        params: &crate::context::PyContext,
        py: Python,
    ) -> PyResult<String> {
        // A cloaca Context carries the params dict; its JSON form is the
        // fully-resolved instance param object persisted on the schedule.
        let params_json = params.to_json()?;
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::RegisterWorkflowInstance {
            workflow_name,
            instance_name,
            cron_expression,
            timezone,
            params_json,
            response_tx,
        };
        self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to register workflow instance",
        )
    }
```

</details>

##### <span id="DefaultRunner.register_cron_workflow"></span><code>register_​cron_​workflow</code>

```python
def register_cron_workflow(
    workflow_name: str,
    cron_expression: str,
    timezone: str,
) -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::register_cron_workflow`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.register_cron_workflow)

<details>
<summary>Source</summary>

```rust
    pub fn register_cron_workflow(
        &self,
        workflow_name: String,
        cron_expression: String,
        timezone: String,
        py: Python,
    ) -> PyResult<String> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::RegisterCronWorkflow {
            workflow_name,
            cron_expression,
            timezone,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to register cron workflow")
    }
```

</details>

##### <span id="DefaultRunner.list_cron_schedules"></span><code>list_​cron_​schedules</code>

```python
def list_cron_schedules(
    enabled_only: Optional[bool],
    limit: Optional[int],
    offset: Optional[int],
) -> List[PyObject]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::list_cron_schedules`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.list_cron_schedules)

List all cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Optional[bool]` | If True, only return enabled schedules |
| `limit` | `Optional[int]` | Maximum number of schedules to return (default: 100) |
| `offset` | `Optional[int]` | Number of schedules to skip (default: 0) |

<details>
<summary>Source</summary>

```rust
    pub fn list_cron_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListCronSchedules {
            enabled_only: enabled_only.unwrap_or(false),
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let schedules =
            self.send_and_recv(message, response_rx, py, "Failed to list cron schedules")?;
        schedules
            .into_iter()
            .map(|s| schedule_to_cron_dict(s, py))
            .collect()
    }
```

</details>

##### <span id="DefaultRunner.set_cron_schedule_enabled"></span><code>set_​cron_​schedule_​enabled</code>

```python
def set_cron_schedule_enabled(schedule_id: str, enabled: bool) -> None
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::set_cron_schedule_enabled`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.set_cron_schedule_enabled)

Enable or disable a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |
| `enabled` | `bool` |  |

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_schedule_enabled(
        &self,
        schedule_id: String,
        enabled: bool,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::SetCronScheduleEnabled {
            schedule_id,
            enabled,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }
```

</details>

##### <span id="DefaultRunner.delete_cron_schedule"></span><code>delete_​cron_​schedule</code>

```python
def delete_cron_schedule(schedule_id: str) -> None
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::delete_cron_schedule`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.delete_cron_schedule)

Delete a cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn delete_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::DeleteCronSchedule {
            schedule_id,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to delete cron schedule")
    }
```

</details>

##### <span id="DefaultRunner.get_cron_schedule"></span><code>get_​cron_​schedule</code>

```python
def get_cron_schedule(schedule_id: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::get_cron_schedule`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_cron_schedule)

Get details of a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn get_cron_schedule(&self, schedule_id: String, py: Python) -> PyResult<PyObject> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronSchedule {
            schedule_id,
            response_tx,
        };
        let schedule =
            self.send_and_recv(message, response_rx, py, "Failed to get cron schedule")?;
        schedule_to_cron_dict(schedule, py)
    }
```

</details>

##### <span id="DefaultRunner.update_cron_schedule"></span><code>update_​cron_​schedule</code>

```python
def update_cron_schedule(schedule_id: str, cron_expression: str, timezone: str) -> None
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::update_cron_schedule`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.update_cron_schedule)

Update a cron schedule's expression and timezone

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |
| `cron_expression` | `str` |  |
| `timezone` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn update_cron_schedule(
        &self,
        schedule_id: String,
        cron_expression: String,
        timezone: String,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::UpdateCronSchedule {
            schedule_id,
            cron_expression,
            timezone,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to update cron schedule")
    }
```

</details>

##### <span id="DefaultRunner.get_cron_execution_history"></span><code>get_​cron_​execution_​history</code>

```python
def get_cron_execution_history(
    schedule_id: str,
    limit: Optional[int],
    offset: Optional[int],
) -> List[PyObject]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::get_cron_execution_history`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_cron_execution_history)

Get execution history for a specific cron schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `schedule_id` | `str` |  |
| `limit` | `Optional[int]` |  |
| `offset` | `Optional[int]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn get_cron_execution_history(
        &self,
        schedule_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronExecutionHistory {
            schedule_id,
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let executions = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get cron execution history",
        )?;
        executions
            .into_iter()
            .map(|e| cron_execution_to_dict(e, py))
            .collect()
    }
```

</details>

##### <span id="DefaultRunner.get_cron_execution_stats"></span><code>get_​cron_​execution_​stats</code>

```python
def get_cron_execution_stats(since: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::get_cron_execution_stats`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_cron_execution_stats)

Get execution statistics for cron schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `since` | `str` | Start time for statistics collection (ISO 8601 string) |

<details>
<summary>Source</summary>

```rust
    pub fn get_cron_execution_stats(&self, since: String, py: Python) -> PyResult<PyObject> {
        let since_dt = chrono::DateTime::parse_from_rfc3339(&since)
            .map_err(|e| PyValueError::new_err(format!("Invalid datetime format: {}", e)))?
            .with_timezone(&chrono::Utc);

        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetCronExecutionStats {
            since: since_dt,
            response_tx,
        };
        let stats = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get cron execution stats",
        )?;

        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("total_executions", stats.total_executions)?;
        dict.set_item("successful_executions", stats.successful_executions)?;
        dict.set_item("lost_executions", stats.lost_executions)?;
        dict.set_item("success_rate", stats.success_rate)?;
        Ok(dict.into())
    }
```

</details>

##### <span id="DefaultRunner.list_trigger_schedules"></span><code>list_​trigger_​schedules</code>

```python
def list_trigger_schedules(
    enabled_only: Optional[bool],
    limit: Optional[int],
    offset: Optional[int],
) -> List[PyObject]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::list_trigger_schedules`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.list_trigger_schedules)

List all trigger schedules

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `enabled_only` | `Optional[bool]` |  |
| `limit` | `Optional[int]` |  |
| `offset` | `Optional[int]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn list_trigger_schedules(
        &self,
        enabled_only: Option<bool>,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListTriggerSchedules {
            enabled_only: enabled_only.unwrap_or(false),
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let schedules =
            self.send_and_recv(message, response_rx, py, "Failed to list trigger schedules")?;
        schedules
            .into_iter()
            .map(|s| schedule_to_trigger_dict(s, py))
            .collect()
    }
```

</details>

##### <span id="DefaultRunner.get_trigger_schedule"></span><code>get_​trigger_​schedule</code>

```python
def get_trigger_schedule(trigger_name: str) -> Optional[PyObject]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::get_trigger_schedule`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_trigger_schedule)

Get details of a specific trigger schedule

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` |  |

<details>
<summary>Source</summary>

```rust
    pub fn get_trigger_schedule(
        &self,
        trigger_name: String,
        py: Python,
    ) -> PyResult<Option<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetTriggerSchedule {
            trigger_name,
            response_tx,
        };
        let schedule_opt =
            self.send_and_recv(message, response_rx, py, "Failed to get trigger schedule")?;
        match schedule_opt {
            Some(schedule) => Ok(Some(schedule_to_trigger_dict(schedule, py)?)),
            None => Ok(None),
        }
    }
```

</details>

##### <span id="DefaultRunner.set_trigger_enabled"></span><code>set_​trigger_​enabled</code>

```python
def set_trigger_enabled(trigger_name: str, enabled: bool) -> None
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::set_trigger_enabled`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.set_trigger_enabled)

Enable or disable a trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` |  |
| `enabled` | `bool` |  |

<details>
<summary>Source</summary>

```rust
    pub fn set_trigger_enabled(
        &self,
        trigger_name: String,
        enabled: bool,
        py: Python,
    ) -> PyResult<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::SetTriggerEnabled {
            trigger_name,
            enabled,
            response_tx,
        };
        self.send_and_recv(message, response_rx, py, "Failed to update trigger")
    }
```

</details>

##### <span id="DefaultRunner.get_trigger_execution_history"></span><code>get_​trigger_​execution_​history</code>

```python
def get_trigger_execution_history(
    trigger_name: str,
    limit: Optional[int],
    offset: Optional[int],
) -> List[PyObject]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::get_trigger_execution_history`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.get_trigger_execution_history)

Get execution history for a specific trigger

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `trigger_name` | `str` |  |
| `limit` | `Optional[int]` |  |
| `offset` | `Optional[int]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn get_trigger_execution_history(
        &self,
        trigger_name: String,
        limit: Option<i64>,
        offset: Option<i64>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::GetTriggerExecutionHistory {
            trigger_name,
            limit: limit.unwrap_or(100),
            offset: offset.unwrap_or(0),
            response_tx,
        };
        let executions = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to get trigger execution history",
        )?;
        executions
            .into_iter()
            .map(|e| trigger_execution_to_dict(e, py))
            .collect()
    }
```

</details>

##### <span id="DefaultRunner.subscribe_workflow_to_reactor"></span><code>subscribe_​workflow_​to_​reactor</code>

```python
def subscribe_workflow_to_reactor(
    reactor: str,
    workflow: str,
    tenant: Optional[str],
    when: Optional[str],
) -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::subscribe_workflow_to_reactor`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.subscribe_workflow_to_reactor)

Subscribe a workflow to a reactor's firings.

Each future fire of `reactor` (within `tenant`) will dispatch
`workflow` via the unified scheduler's reactor poll tick. The
dispatched workflow's input context carries the boundary cache
from the firing.

Idempotent — calling twice with the same `(reactor, workflow,
tenant)` upserts; a later call's `when` predicate replaces the
earlier one's.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `reactor` | `str` | The reactor name. |
| `workflow` | `str` | The workflow to dispatch on each firing. |
| `tenant` | `Optional[str]` | Tenant scope. Defaults to `"public"` if omitted. |
| `when` | `Optional[str]` | Optional CEL filter expression (CLOACI-T-0602). When provided, the scheduler evaluates it against the firing payload and only dispatches when it evaluates to true. The watermark still advances when it evaluates to false. Variables available: `payload` (dict, keys are boundary source names), `reactor` (str), `tenant` (str). Invalid CEL is rejected immediately with a ValueError — no row written. |

**Examples:**

```python
# Unfiltered — fires on every firing.
runner.subscribe_workflow_to_reactor("pricing", "alert")

# Filtered — fires only when the boundary's `quote` source
# carries a price above $100 in us-east.
runner.subscribe_workflow_to_reactor(
    "pricing", "alert",
    when="payload.quote.price > 100 && payload.quote.region == 'us-east'",
)
```

<details>
<summary>Source</summary>

```rust
    pub fn subscribe_workflow_to_reactor(
        &self,
        reactor: String,
        workflow: String,
        tenant: Option<String>,
        when: Option<String>,
        py: Python,
    ) -> PyResult<String> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::SubscribeWorkflowToReactor {
            reactor,
            workflow,
            tenant,
            predicate: when,
            response_tx,
        };
        self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to subscribe workflow to reactor",
        )
    }
```

</details>

##### <span id="DefaultRunner.unsubscribe_workflow_from_reactor"></span><code>unsubscribe_​workflow_​from_​reactor</code>

```python
def unsubscribe_workflow_from_reactor(
    reactor: str,
    workflow: str,
    tenant: Optional[str],
) -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::unsubscribe_workflow_from_reactor`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.unsubscribe_workflow_from_reactor)

Remove a workflow-to-reactor subscription.

Returns True if a row was deleted, False if no subscription
matched `(reactor, workflow, tenant)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `reactor` | `str` |  |
| `workflow` | `str` |  |
| `tenant` | `Optional[str]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn unsubscribe_workflow_from_reactor(
        &self,
        reactor: String,
        workflow: String,
        tenant: Option<String>,
        py: Python,
    ) -> PyResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::UnsubscribeWorkflowFromReactor {
            reactor,
            workflow,
            tenant,
            response_tx,
        };
        self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to unsubscribe workflow from reactor",
        )
    }
```

</details>

##### <span id="DefaultRunner.list_reactor_subscriptions"></span><code>list_​reactor_​subscriptions</code>

```python
def list_reactor_subscriptions(tenant: Optional[str]) -> List[PyObject]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::list_reactor_subscriptions`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.list_reactor_subscriptions)

List enabled reactor subscriptions for a tenant.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `tenant` | `Optional[str]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn list_reactor_subscriptions(
        &self,
        tenant: Option<String>,
        py: Python,
    ) -> PyResult<Vec<PyObject>> {
        let (response_tx, response_rx) = oneshot::channel();
        let message = RuntimeMessage::ListReactorSubscriptions {
            tenant,
            response_tx,
        };
        let subs = self.send_and_recv(
            message,
            response_rx,
            py,
            "Failed to list reactor subscriptions",
        )?;
        subs.into_iter()
            .map(|s| reactor_subscription_to_dict(s, py))
            .collect()
    }
```

</details>

##### <span id="DefaultRunner.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::__repr__`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DefaultRunner(thread_separated_async_runtime)".to_string()
    }
```

</details>

##### <span id="DefaultRunner.__enter__"></span><code>_​_​enter_​_​</code>

```python
def __enter__(slf: PyRef<Self>) -> PyRef<Self>
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::__enter__`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.__enter__)

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }
```

</details>

##### <span id="DefaultRunner.__exit__"></span><code>_​_​exit_​_​</code>

```python
def __exit__(
    _exc_type: Optional[Bound<PyAny>],
    _exc_value: Optional[Bound<PyAny>],
    _traceback: Optional[Bound<PyAny>],
) -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyDefaultRunner::__exit__`](../rust/cloacina-python/bindings/runner/#PyDefaultRunner.__exit__)

<details>
<summary>Source</summary>

```rust
    pub fn __exit__(
        &self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        self.shutdown(py)?;
        Ok(false) // Don't suppress exceptions
    }
```

</details>

### <span id="class.WorkflowResult"></span><code style="font-size: 0.85em">class WorkflowResult</code>

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult`](../rust/cloacina-python/bindings/runner/#struct.PyWorkflowResult)

Python wrapper for WorkflowExecutionResult

#### Methods

##### <span id="WorkflowResult.status"></span><code>status</code>

```python
def status() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult::status`](../rust/cloacina-python/bindings/runner/#PyWorkflowResult.status)

<details>
<summary>Source</summary>

```rust
    pub fn status(&self) -> String {
        format!("{:?}", self.inner.status)
    }
```

</details>

##### <span id="WorkflowResult.start_time"></span><code>start_​time</code>

```python
def start_time() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult::start_time`](../rust/cloacina-python/bindings/runner/#PyWorkflowResult.start_time)

<details>
<summary>Source</summary>

```rust
    pub fn start_time(&self) -> String {
        self.inner.start_time.to_rfc3339()
    }
```

</details>

##### <span id="WorkflowResult.end_time"></span><code>end_​time</code>

```python
def end_time() -> Optional[str]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult::end_time`](../rust/cloacina-python/bindings/runner/#PyWorkflowResult.end_time)

<details>
<summary>Source</summary>

```rust
    pub fn end_time(&self) -> Option<String> {
        self.inner.end_time.map(|t| t.to_rfc3339())
    }
```

</details>

##### <span id="WorkflowResult.final_context"></span><code>final_​context</code>

```python
def final_context() -> PyContext
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult::final_context`](../rust/cloacina-python/bindings/runner/#PyWorkflowResult.final_context)

<details>
<summary>Source</summary>

```rust
    pub fn final_context(&self) -> PyContext {
        let new_context = self.inner.final_context.clone_data();
        PyContext::from_rust_context(new_context)
    }
```

</details>

##### <span id="WorkflowResult.error_message"></span><code>error_​message</code>

```python
def error_message() -> Optional[str]
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult::error_message`](../rust/cloacina-python/bindings/runner/#PyWorkflowResult.error_message)

<details>
<summary>Source</summary>

```rust
    pub fn error_message(&self) -> Option<&str> {
        self.inner.error_message.as_deref()
    }
```

</details>

##### <span id="WorkflowResult.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::runner::PyWorkflowResult::__repr__`](../rust/cloacina-python/bindings/runner/#PyWorkflowResult.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "WorkflowResult(status={}, error={})",
            self.status(),
            self.error_message().unwrap_or("None")
        )
    }
```

</details>

### <span id="class.DefaultRunnerConfig"></span><code style="font-size: 0.85em">class DefaultRunnerConfig</code>

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig`](../rust/cloacina-python/bindings/context/#struct.PyDefaultRunnerConfig)

PyDefaultRunnerConfig - Python wrapper for Rust DefaultRunnerConfig

#### Methods

##### <span id="DefaultRunnerConfig.new"></span><code>new</code>

```python
def new(
    max_concurrent_tasks: Optional[int],
    scheduler_poll_interval_ms: Optional[int],
    task_timeout_seconds: Optional[int],
    workflow_timeout_seconds: Optional[int],
    db_pool_size: Optional[int],
    enable_recovery: Optional[bool],
    enable_cron_scheduling: Optional[bool],
    cron_poll_interval_seconds: Optional[int],
    cron_max_catchup_executions: Optional[int],
    cron_enable_recovery: Optional[bool],
    cron_recovery_interval_seconds: Optional[int],
    cron_lost_threshold_minutes: Optional[int],
    cron_max_recovery_age_seconds: Optional[int],
    cron_max_recovery_attempts: Optional[int],
) -> pyo3::PyResult<Self>
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::new`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.new)

Creates a new DefaultRunnerConfig with customizable parameters

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `max_concurrent_tasks` | `Optional[int]` |  |
| `scheduler_poll_interval_ms` | `Optional[int]` |  |
| `task_timeout_seconds` | `Optional[int]` |  |
| `workflow_timeout_seconds` | `Optional[int]` |  |
| `db_pool_size` | `Optional[int]` |  |
| `enable_recovery` | `Optional[bool]` |  |
| `enable_cron_scheduling` | `Optional[bool]` |  |
| `cron_poll_interval_seconds` | `Optional[int]` |  |
| `cron_max_catchup_executions` | `Optional[int]` |  |
| `cron_enable_recovery` | `Optional[bool]` |  |
| `cron_recovery_interval_seconds` | `Optional[int]` |  |
| `cron_lost_threshold_minutes` | `Optional[int]` |  |
| `cron_max_recovery_age_seconds` | `Optional[int]` |  |
| `cron_max_recovery_attempts` | `Optional[int]` |  |

<details>
<summary>Source</summary>

```rust
    pub fn new(
        max_concurrent_tasks: Option<usize>,
        scheduler_poll_interval_ms: Option<u64>,
        task_timeout_seconds: Option<u64>,
        workflow_timeout_seconds: Option<u64>,
        db_pool_size: Option<u32>,
        enable_recovery: Option<bool>,
        enable_cron_scheduling: Option<bool>,
        cron_poll_interval_seconds: Option<u64>,
        cron_max_catchup_executions: Option<usize>,
        cron_enable_recovery: Option<bool>,
        cron_recovery_interval_seconds: Option<u64>,
        cron_lost_threshold_minutes: Option<i32>,
        cron_max_recovery_age_seconds: Option<u64>,
        cron_max_recovery_attempts: Option<usize>,
    ) -> pyo3::PyResult<Self> {
        use std::time::Duration;

        let mut builder = cloacina::runner::DefaultRunnerConfig::builder();

        if let Some(val) = max_concurrent_tasks {
            builder = builder.max_concurrent_tasks(val);
        }
        if let Some(val) = scheduler_poll_interval_ms {
            builder = builder.scheduler_poll_interval(Duration::from_millis(val));
        }
        if let Some(val) = task_timeout_seconds {
            builder = builder.task_timeout(Duration::from_secs(val));
        }
        if let Some(val) = workflow_timeout_seconds {
            builder = builder.workflow_timeout(Some(Duration::from_secs(val)));
        }
        if let Some(val) = db_pool_size {
            builder = builder.db_pool_size(val);
        }
        if let Some(val) = enable_recovery {
            builder = builder.enable_recovery(val);
        }
        if let Some(val) = enable_cron_scheduling {
            builder = builder.enable_cron_scheduling(val);
        }
        if let Some(val) = cron_poll_interval_seconds {
            builder = builder.cron_poll_interval(Duration::from_secs(val));
        }
        if let Some(val) = cron_max_catchup_executions {
            builder = builder.cron_max_catchup_executions(val);
        }
        if let Some(val) = cron_enable_recovery {
            builder = builder.cron_enable_recovery(val);
        }
        if let Some(val) = cron_recovery_interval_seconds {
            builder = builder.cron_recovery_interval(Duration::from_secs(val));
        }
        if let Some(val) = cron_lost_threshold_minutes {
            builder = builder.cron_lost_threshold_minutes(val);
        }
        if let Some(val) = cron_max_recovery_age_seconds {
            builder = builder.cron_max_recovery_age(Duration::from_secs(val));
        }
        if let Some(val) = cron_max_recovery_attempts {
            builder = builder.cron_max_recovery_attempts(val);
        }

        Ok(PyDefaultRunnerConfig {
            inner: builder.build().map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid configuration: {}", e))
            })?,
        })
    }
```

</details>

##### <span id="DefaultRunnerConfig.default"></span><code>default</code>

```python
def default() -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::default`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.default)

Creates a DefaultRunnerConfig with all default values

<details>
<summary>Source</summary>

```rust
    pub fn default() -> Self {
        PyDefaultRunnerConfig {
            inner: cloacina::runner::DefaultRunnerConfig::default(),
        }
    }
```

</details>

##### <span id="DefaultRunnerConfig.max_concurrent_tasks"></span><code>max_​concurrent_​tasks</code>

```python
def max_concurrent_tasks() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::max_concurrent_tasks`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.max_concurrent_tasks)

<details>
<summary>Source</summary>

```rust
    pub fn max_concurrent_tasks(&self) -> usize {
        self.inner.max_concurrent_tasks()
    }
```

</details>

##### <span id="DefaultRunnerConfig.scheduler_poll_interval_ms"></span><code>scheduler_​poll_​interval_​ms</code>

```python
def scheduler_poll_interval_ms() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::scheduler_poll_interval_ms`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```rust
    pub fn scheduler_poll_interval_ms(&self) -> u64 {
        self.inner.scheduler_poll_interval().as_millis() as u64
    }
```

</details>

##### <span id="DefaultRunnerConfig.task_timeout_seconds"></span><code>task_​timeout_​seconds</code>

```python
def task_timeout_seconds() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::task_timeout_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.task_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn task_timeout_seconds(&self) -> u64 {
        self.inner.task_timeout().as_secs()
    }
```

</details>

##### <span id="DefaultRunnerConfig.workflow_timeout_seconds"></span><code>workflow_​timeout_​seconds</code>

```python
def workflow_timeout_seconds() -> Optional[int]
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::workflow_timeout_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.workflow_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn workflow_timeout_seconds(&self) -> Option<u64> {
        self.inner.workflow_timeout().map(|d| d.as_secs())
    }
```

</details>

##### <span id="DefaultRunnerConfig.db_pool_size"></span><code>db_​pool_​size</code>

```python
def db_pool_size() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::db_pool_size`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.db_pool_size)

<details>
<summary>Source</summary>

```rust
    pub fn db_pool_size(&self) -> u32 {
        self.inner.db_pool_size()
    }
```

</details>

##### <span id="DefaultRunnerConfig.enable_recovery"></span><code>enable_​recovery</code>

```python
def enable_recovery() -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::enable_recovery`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn enable_recovery(&self) -> bool {
        self.inner.enable_recovery()
    }
```

</details>

##### <span id="DefaultRunnerConfig.enable_cron_scheduling"></span><code>enable_​cron_​scheduling</code>

```python
def enable_cron_scheduling() -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::enable_cron_scheduling`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.enable_cron_scheduling)

<details>
<summary>Source</summary>

```rust
    pub fn enable_cron_scheduling(&self) -> bool {
        self.inner.enable_cron_scheduling()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_poll_interval_seconds"></span><code>cron_​poll_​interval_​seconds</code>

```python
def cron_poll_interval_seconds() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_poll_interval_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_poll_interval_seconds(&self) -> u64 {
        self.inner.cron_poll_interval().as_secs()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_max_catchup_executions"></span><code>cron_​max_​catchup_​executions</code>

```python
def cron_max_catchup_executions() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_max_catchup_executions`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_max_catchup_executions)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_catchup_executions(&self) -> usize {
        self.inner.cron_max_catchup_executions()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_enable_recovery"></span><code>cron_​enable_​recovery</code>

```python
def cron_enable_recovery() -> bool
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_enable_recovery`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn cron_enable_recovery(&self) -> bool {
        self.inner.cron_enable_recovery()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_recovery_interval_seconds"></span><code>cron_​recovery_​interval_​seconds</code>

```python
def cron_recovery_interval_seconds() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_recovery_interval_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_recovery_interval_seconds(&self) -> u64 {
        self.inner.cron_recovery_interval().as_secs()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_lost_threshold_minutes"></span><code>cron_​lost_​threshold_​minutes</code>

```python
def cron_lost_threshold_minutes() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_lost_threshold_minutes`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```rust
    pub fn cron_lost_threshold_minutes(&self) -> i32 {
        self.inner.cron_lost_threshold_minutes()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_max_recovery_age_seconds"></span><code>cron_​max_​recovery_​age_​seconds</code>

```python
def cron_max_recovery_age_seconds() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_max_recovery_age_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_age_seconds(&self) -> u64 {
        self.inner.cron_max_recovery_age().as_secs()
    }
```

</details>

##### <span id="DefaultRunnerConfig.cron_max_recovery_attempts"></span><code>cron_​max_​recovery_​attempts</code>

```python
def cron_max_recovery_attempts() -> int
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::cron_max_recovery_attempts`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```rust
    pub fn cron_max_recovery_attempts(&self) -> usize {
        self.inner.cron_max_recovery_attempts()
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_max_concurrent_tasks"></span><code>set_​max_​concurrent_​tasks</code>

```python
def set_max_concurrent_tasks(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_max_concurrent_tasks`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_max_concurrent_tasks)

<details>
<summary>Source</summary>

```rust
    pub fn set_max_concurrent_tasks(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.max_concurrent_tasks(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_scheduler_poll_interval_ms"></span><code>set_​scheduler_​poll_​interval_​ms</code>

```python
def set_scheduler_poll_interval_ms(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_scheduler_poll_interval_ms`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_scheduler_poll_interval_ms)

<details>
<summary>Source</summary>

```rust
    pub fn set_scheduler_poll_interval_ms(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.scheduler_poll_interval(std::time::Duration::from_millis(value)));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_task_timeout_seconds"></span><code>set_​task_​timeout_​seconds</code>

```python
def set_task_timeout_seconds(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_task_timeout_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_task_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_task_timeout_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.task_timeout(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_workflow_timeout_seconds"></span><code>set_​workflow_​timeout_​seconds</code>

```python
def set_workflow_timeout_seconds(value: Optional[int])
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_workflow_timeout_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_workflow_timeout_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_workflow_timeout_seconds(&mut self, value: Option<u64>) {
        self.inner =
            self.rebuild(|b| b.workflow_timeout(value.map(std::time::Duration::from_secs)));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_db_pool_size"></span><code>set_​db_​pool_​size</code>

```python
def set_db_pool_size(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_db_pool_size`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_db_pool_size)

<details>
<summary>Source</summary>

```rust
    pub fn set_db_pool_size(&mut self, value: u32) {
        self.inner = self.rebuild(|b| b.db_pool_size(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_enable_recovery"></span><code>set_​enable_​recovery</code>

```python
def set_enable_recovery(value: bool)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_enable_recovery`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn set_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_recovery(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_enable_cron_scheduling"></span><code>set_​enable_​cron_​scheduling</code>

```python
def set_enable_cron_scheduling(value: bool)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_enable_cron_scheduling`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_enable_cron_scheduling)

<details>
<summary>Source</summary>

```rust
    pub fn set_enable_cron_scheduling(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.enable_cron_scheduling(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_poll_interval_seconds"></span><code>set_​cron_​poll_​interval_​seconds</code>

```python
def set_cron_poll_interval_seconds(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_poll_interval_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_poll_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_poll_interval_seconds(&mut self, value: u64) {
        self.inner = self.rebuild(|b| b.cron_poll_interval(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_max_catchup_executions"></span><code>set_​cron_​max_​catchup_​executions</code>

```python
def set_cron_max_catchup_executions(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_max_catchup_executions`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_max_catchup_executions)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_catchup_executions(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_catchup_executions(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_enable_recovery"></span><code>set_​cron_​enable_​recovery</code>

```python
def set_cron_enable_recovery(value: bool)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_enable_recovery`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_enable_recovery)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_enable_recovery(&mut self, value: bool) {
        self.inner = self.rebuild(|b| b.cron_enable_recovery(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_recovery_interval_seconds"></span><code>set_​cron_​recovery_​interval_​seconds</code>

```python
def set_cron_recovery_interval_seconds(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_recovery_interval_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_recovery_interval_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_recovery_interval_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_recovery_interval(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_lost_threshold_minutes"></span><code>set_​cron_​lost_​threshold_​minutes</code>

```python
def set_cron_lost_threshold_minutes(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_lost_threshold_minutes`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_lost_threshold_minutes)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_lost_threshold_minutes(&mut self, value: i32) {
        self.inner = self.rebuild(|b| b.cron_lost_threshold_minutes(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_max_recovery_age_seconds"></span><code>set_​cron_​max_​recovery_​age_​seconds</code>

```python
def set_cron_max_recovery_age_seconds(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_max_recovery_age_seconds`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_max_recovery_age_seconds)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_recovery_age_seconds(&mut self, value: u64) {
        self.inner =
            self.rebuild(|b| b.cron_max_recovery_age(std::time::Duration::from_secs(value)));
    }
```

</details>

##### <span id="DefaultRunnerConfig.set_cron_max_recovery_attempts"></span><code>set_​cron_​max_​recovery_​attempts</code>

```python
def set_cron_max_recovery_attempts(value: int)
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::set_cron_max_recovery_attempts`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.set_cron_max_recovery_attempts)

<details>
<summary>Source</summary>

```rust
    pub fn set_cron_max_recovery_attempts(&mut self, value: usize) {
        self.inner = self.rebuild(|b| b.cron_max_recovery_attempts(value));
    }
```

</details>

##### <span id="DefaultRunnerConfig.to_dict"></span><code>to_​dict</code>

```python
def to_dict() -> PyObject
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::to_dict`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.to_dict)

Returns a dictionary representation of the configuration

<details>
<summary>Source</summary>

```rust
    pub fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new(py);

        dict.set_item("max_concurrent_tasks", self.inner.max_concurrent_tasks())?;
        dict.set_item(
            "scheduler_poll_interval_ms",
            self.inner.scheduler_poll_interval().as_millis() as u64,
        )?;
        dict.set_item("task_timeout_seconds", self.inner.task_timeout().as_secs())?;
        dict.set_item(
            "workflow_timeout_seconds",
            self.inner.workflow_timeout().map(|d| d.as_secs()),
        )?;
        dict.set_item("db_pool_size", self.inner.db_pool_size())?;
        dict.set_item("enable_recovery", self.inner.enable_recovery())?;
        dict.set_item(
            "enable_cron_scheduling",
            self.inner.enable_cron_scheduling(),
        )?;
        dict.set_item(
            "cron_poll_interval_seconds",
            self.inner.cron_poll_interval().as_secs(),
        )?;
        dict.set_item(
            "cron_max_catchup_executions",
            self.inner.cron_max_catchup_executions(),
        )?;
        dict.set_item("cron_enable_recovery", self.inner.cron_enable_recovery())?;
        dict.set_item(
            "cron_recovery_interval_seconds",
            self.inner.cron_recovery_interval().as_secs(),
        )?;
        dict.set_item(
            "cron_lost_threshold_minutes",
            self.inner.cron_lost_threshold_minutes(),
        )?;
        dict.set_item(
            "cron_max_recovery_age_seconds",
            self.inner.cron_max_recovery_age().as_secs(),
        )?;
        dict.set_item(
            "cron_max_recovery_attempts",
            self.inner.cron_max_recovery_attempts(),
        )?;

        Ok(dict.into())
    }
```

</details>

##### <span id="DefaultRunnerConfig.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::context::PyDefaultRunnerConfig::__repr__`](../rust/cloacina-python/bindings/context/#PyDefaultRunnerConfig.__repr__)

String representation of the configuration

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "DefaultRunnerConfig(max_concurrent_tasks={}, enable_cron_scheduling={}, db_pool_size={})",
            self.inner.max_concurrent_tasks(),
            self.inner.enable_cron_scheduling(),
            self.inner.db_pool_size()
        )
    }
```

</details>

### <span id="class.DatabaseAdmin"></span><code style="font-size: 0.85em">class DatabaseAdmin</code>

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyDatabaseAdmin`](../rust/cloacina-python/bindings/admin/#struct.PyDatabaseAdmin)

Python wrapper for DatabaseAdmin

Note: This class is only functional with PostgreSQL databases.
SQLite does not support database schemas or user management.

#### Methods

##### <span id="DatabaseAdmin.new"></span><code>new</code>

```python
def new(database_url: str) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyDatabaseAdmin::new`](../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.new)

<details>
<summary>Source</summary>

```rust
    pub fn new(database_url: String) -> PyResult<Self> {
        // Runtime check for PostgreSQL
        if !is_postgres_url(&database_url) {
            return Err(PyRuntimeError::new_err(
                "DatabaseAdmin requires a PostgreSQL connection. \
                 SQLite does not support database schemas or user management. \
                 Use a PostgreSQL URL like 'postgres://user:pass@host/db'",
            ));
        }

        // Parse the database URL to extract components
        let url = url::Url::parse(&database_url)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid database URL: {}", e)))?;

        let database_name = url.path().trim_start_matches('/');
        if database_name.is_empty() {
            return Err(PyRuntimeError::new_err(
                "Database name is required in URL path",
            ));
        }

        // Build connection string with all components
        let username = url.username();
        let password = url.password().unwrap_or("");
        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(5432);

        let connection_string = if password.is_empty() {
            format!("{}://{}@{}:{}", url.scheme(), username, host, port)
        } else {
            format!(
                "{}://{}:{}@{}:{}",
                url.scheme(),
                username,
                password,
                host,
                port
            )
        };

        let database = Database::new(&connection_string, database_name, 10);
        let admin = DatabaseAdmin::new(database);
        Ok(Self { inner: admin })
    }
```

</details>

##### <span id="DatabaseAdmin.create_tenant"></span><code>create_​tenant</code>

```python
def create_tenant(config: PyTenantConfig) -> PyTenantCredentials
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyDatabaseAdmin::create_tenant`](../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.create_tenant)

<details>
<summary>Source</summary>

```rust
    pub fn create_tenant(
        &self,
        py: Python<'_>,
        config: &PyTenantConfig,
    ) -> PyResult<PyTenantCredentials> {
        let tenant_config = TenantConfig {
            schema_name: config.inner.schema_name.clone(),
            username: config.inner.username.clone(),
            password: config.inner.password.clone(),
        };

        // CLOACI-I-0136: drive the async admin call through the one GIL-safe
        // bridge (releases the GIL before blocking) — previously this held the
        // GIL across the await on a fresh runtime.
        let credentials =
            crate::gil::py_block_on(py, async { self.inner.create_tenant(tenant_config).await })
                .map_err(|e: AdminError| {
                    PyRuntimeError::new_err(format!("Failed to create tenant: {}", e))
                })?;

        Ok(PyTenantCredentials { inner: credentials })
    }
```

</details>

##### <span id="DatabaseAdmin.remove_tenant"></span><code>remove_​tenant</code>

```python
def remove_tenant(schema_name: str, username: str) -> None
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyDatabaseAdmin::remove_tenant`](../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.remove_tenant)

<details>
<summary>Source</summary>

```rust
    pub fn remove_tenant(
        &self,
        py: Python<'_>,
        schema_name: String,
        username: String,
    ) -> PyResult<()> {
        // CLOACI-I-0136: GIL-safe bridge (see create_tenant).
        crate::gil::py_block_on(py, async {
            self.inner.remove_tenant(&schema_name, &username).await
        })
        .map_err(|e: AdminError| {
            PyRuntimeError::new_err(format!("Failed to remove tenant: {}", e))
        })?;

        Ok(())
    }
```

</details>

##### <span id="DatabaseAdmin.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyDatabaseAdmin::__repr__`](../rust/cloacina-python/bindings/admin/#PyDatabaseAdmin.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        "DatabaseAdmin()".to_string()
    }
```

</details>

### <span id="class.TenantConfig"></span><code style="font-size: 0.85em">class TenantConfig</code>

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantConfig`](../rust/cloacina-python/bindings/admin/#struct.PyTenantConfig)

Python wrapper for TenantConfig

#### Methods

##### <span id="TenantConfig.new"></span><code>new</code>

```python
def new(schema_name: str, username: str, password: Optional[str]) -> Self
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantConfig::new`](../rust/cloacina-python/bindings/admin/#PyTenantConfig.new)

<details>
<summary>Source</summary>

```rust
    pub fn new(schema_name: String, username: String, password: Option<String>) -> Self {
        Self {
            inner: TenantConfig {
                schema_name,
                username,
                password: password.unwrap_or_default(),
            },
        }
    }
```

</details>

##### <span id="TenantConfig.schema_name"></span><code>schema_​name</code>

```python
def schema_name() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantConfig::schema_name`](../rust/cloacina-python/bindings/admin/#PyTenantConfig.schema_name)

<details>
<summary>Source</summary>

```rust
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>

##### <span id="TenantConfig.username"></span><code>username</code>

```python
def username() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantConfig::username`](../rust/cloacina-python/bindings/admin/#PyTenantConfig.username)

<details>
<summary>Source</summary>

```rust
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>

##### <span id="TenantConfig.password"></span><code>password</code>

```python
def password() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantConfig::password`](../rust/cloacina-python/bindings/admin/#PyTenantConfig.password)

<details>
<summary>Source</summary>

```rust
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>

##### <span id="TenantConfig.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantConfig::__repr__`](../rust/cloacina-python/bindings/admin/#PyTenantConfig.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "TenantConfig(schema_name='{}', username='{}', password='***')",
            self.inner.schema_name, self.inner.username
        )
    }
```

</details>

### <span id="class.TenantCredentials"></span><code style="font-size: 0.85em">class TenantCredentials</code>

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantCredentials`](../rust/cloacina-python/bindings/admin/#struct.PyTenantCredentials)

Python wrapper for TenantCredentials

#### Methods

##### <span id="TenantCredentials.username"></span><code>username</code>

```python
def username() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantCredentials::username`](../rust/cloacina-python/bindings/admin/#PyTenantCredentials.username)

<details>
<summary>Source</summary>

```rust
    pub fn username(&self) -> String {
        self.inner.username.clone()
    }
```

</details>

##### <span id="TenantCredentials.password"></span><code>password</code>

```python
def password() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantCredentials::password`](../rust/cloacina-python/bindings/admin/#PyTenantCredentials.password)

<details>
<summary>Source</summary>

```rust
    pub fn password(&self) -> String {
        self.inner.password.clone()
    }
```

</details>

##### <span id="TenantCredentials.schema_name"></span><code>schema_​name</code>

```python
def schema_name() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantCredentials::schema_name`](../rust/cloacina-python/bindings/admin/#PyTenantCredentials.schema_name)

<details>
<summary>Source</summary>

```rust
    pub fn schema_name(&self) -> String {
        self.inner.schema_name.clone()
    }
```

</details>

##### <span id="TenantCredentials.connection_string"></span><code>connection_​string</code>

```python
def connection_string() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantCredentials::connection_string`](../rust/cloacina-python/bindings/admin/#PyTenantCredentials.connection_string)

<details>
<summary>Source</summary>

```rust
    pub fn connection_string(&self) -> String {
        self.inner.connection_string.clone()
    }
```

</details>

##### <span id="TenantCredentials.__repr__"></span><code>_​_​repr_​_​</code>

```python
def __repr__() -> str
```

> **Rust Implementation**: [`cloacina-python::bindings::admin::PyTenantCredentials::__repr__`](../rust/cloacina-python/bindings/admin/#PyTenantCredentials.__repr__)

<details>
<summary>Source</summary>

```rust
    pub fn __repr__(&self) -> String {
        format!(
            "TenantCredentials(username='{}', schema_name='{}', password='***', connection_string='***')",
            self.inner.username, self.inner.schema_name
        )
    }
```

</details>

## Functions

### <span id="fn.task"></span><code style="font-size: 0.85em">task</code>

```python
def task(
    id: Optional[str],
    dependencies: Optional[List[PyObject]],
    retry: Optional[crate::bindings::PyRetryPolicy],
    retry_attempts: Optional[int],
    retry_backoff: Optional[str],
    retry_delay_ms: Optional[int],
    retry_max_delay_ms: Optional[int],
    retry_condition: Optional[str],
    retry_jitter: Optional[bool],
    on_success: Optional[PyObject],
    on_failure: Optional[PyObject],
    invokes: Optional[PyObject],
    post_invocation: Optional[PyObject],
    trigger_rules: Optional[PyObject],
) -> TaskDecorator
```

> **Rust Implementation**: [`cloacina-python::task::task`](../rust/cloacina-python/task/#fn.task)

Python @task decorator function

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `Optional[str]` |  |
| `dependencies` | `Optional[List[PyObject]]` |  |
| `retry` | `Optional[crate::bindings::PyRetryPolicy]` |  |
| `retry_attempts` | `Optional[int]` |  |
| `retry_backoff` | `Optional[str]` |  |
| `retry_delay_ms` | `Optional[int]` |  |
| `retry_max_delay_ms` | `Optional[int]` |  |
| `retry_condition` | `Optional[str]` |  |
| `retry_jitter` | `Optional[bool]` |  |
| `on_success` | `Optional[PyObject]` |  |
| `on_failure` | `Optional[PyObject]` |  |
| `invokes` | `Optional[PyObject]` |  |
| `post_invocation` | `Optional[PyObject]` |  |
| `trigger_rules` | `Optional[PyObject]` |  |

<details>
<summary>Source</summary>

```rust
pub fn task(
    py: Python,
    id: Option<String>,
    dependencies: Option<Vec<PyObject>>,
    retry: Option<crate::bindings::PyRetryPolicy>,
    retry_attempts: Option<usize>,
    retry_backoff: Option<String>,
    retry_delay_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_condition: Option<String>,
    retry_jitter: Option<bool>,
    on_success: Option<PyObject>,
    on_failure: Option<PyObject>,
    invokes: Option<PyObject>,
    post_invocation: Option<PyObject>,
    trigger_rules: Option<PyObject>,
) -> PyResult<TaskDecorator> {
    // CLOACI-T-0882: two retry-authoring surfaces exist — the `retry_*` kwargs
    // and a `RetryPolicy` value object. Accept either, never both: silently
    // preferring one would make the ignored surface look effective, which is
    // exactly the "exposed but does nothing" failure this ticket was filed for.
    let kwargs_used: &[(&str, bool)] = &[
        ("retry_attempts", retry_attempts.is_some()),
        ("retry_backoff", retry_backoff.is_some()),
        ("retry_delay_ms", retry_delay_ms.is_some()),
        ("retry_max_delay_ms", retry_max_delay_ms.is_some()),
        ("retry_condition", retry_condition.is_some()),
        ("retry_jitter", retry_jitter.is_some()),
    ];
    let conflicting: Vec<&str> = kwargs_used
        .iter()
        .filter(|(_, used)| *used)
        .map(|(name, _)| *name)
        .collect();

    let retry_policy = match retry {
        Some(policy) => {
            if !conflicting.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "@task(retry=RetryPolicy(...)) cannot be combined with the \
                     retry_* keyword arguments (got: {}). Use the RetryPolicy \
                     object OR the keywords, not both.",
                    conflicting.join(", ")
                )));
            }
            policy.to_rust()
        }
        None => build_retry_policy(
            retry_attempts,
            retry_backoff,
            retry_delay_ms,
            retry_max_delay_ms,
            retry_condition,
            retry_jitter,
        )?,
    };

    // CLOACI-T-0763: parse + validate the trigger rule (default = Always).
    let trigger_rules = match trigger_rules {
        Some(obj) => crate::trigger_rules::parse_trigger_rules(py, &obj)?,
        None => serde_json::json!({ "type": "Always" }),
    };

    Ok(TaskDecorator {
        id,
        dependencies: dependencies.unwrap_or_default(),
        retry_policy,
        on_success,
        on_failure,
        invokes,
        post_invocation,
        trigger_rules,
    })
}
```

</details>

### <span id="fn.constructor"></span><code style="font-size: 0.85em">constructor</code>

```python
def constructor(
    id: str,
    from_: str,
    constructor: str,
    config: Optional[Bound<'_, PyDict>],
    grants: Optional[Bound<'_, PyDict>],
    dependencies: Optional[List[PyObject]],
    runtime: Optional[str],
) -> None
```

> **Rust Implementation**: [`cloacina-python::constructor::constructor`](../rust/cloacina-python/constructor/#fn.constructor)

Wire a packaged constructor-provider member into the current workflow as a DAG node. See the module docs for the full model; `from_` is the provider (Cargo package) name with an optional `@version` pin, `constructor` selects the member, `config` binds by NAME against the member's declared `#[config]` schema, and `grants` is the default-closed capability grant map (`{"http": [...], "tcp": [...], "fs": [...], "env": [...]}`).

`runtime` (CLOACI-T-0920) optionally PINS the provider's trust tier —
`"wasm"` or `"native"`. If pinned, the load fails unless the resolved
provider's runtime matches, naming both. Because `grants` are ENFORCED on wasm
but only ADVISORY on native, passing `grants` for a provider that resolves
NATIVE is a hard error unless acknowledged with `runtime="native"`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `id` | `str` |  |
| `from_` | `str` |  |
| `constructor` | `str` |  |
| `config` | `Optional[Bound<'_, PyDict>]` |  |
| `grants` | `Optional[Bound<'_, PyDict>]` |  |
| `dependencies` | `Optional[List[PyObject]]` |  |
| `runtime` | `Optional[str]` |  |

<details>
<summary>Source</summary>

```rust
pub fn constructor(
    py: Python,
    id: String,
    from_: String,
    constructor: String,
    config: Option<Bound<'_, PyDict>>,
    grants: Option<Bound<'_, PyDict>>,
    dependencies: Option<Vec<PyObject>>,
    runtime: Option<String>,
) -> PyResult<()> {
    let context = current_workflow_context()?;
    let (tenant_id, package_name, workflow_id) = context.as_components();
    let namespace = cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &id);

    // config: dict → (name, value) pairs in WRITTEN order; the loader reorders
    // them into the member's declaration order and binds by name (kwargs).
    let mut config_pairs: Vec<(String, serde_json::Value)> = Vec::new();
    if let Some(cfg) = &config {
        for (k, v) in cfg.iter() {
            let key: String = k.extract().map_err(|_| {
                PyValueError::new_err("cloaca.constructor: config keys must be strings")
            })?;
            let value: serde_json::Value = pythonize::depythonize(&v).map_err(|e| {
                PyValueError::new_err(format!(
                    "cloaca.constructor: config value for '{key}' is not JSON-convertible: {e}"
                ))
            })?;
            config_pairs.push((key, value));
        }
    }

    // grants: {"kind": ["pattern", ...]} → GrantSpec (default-closed when absent).
    let mut grant_pairs: Vec<(String, Vec<String>)> = Vec::new();
    if let Some(g) = &grants {
        for (k, v) in g.iter() {
            let kind: String = k.extract().map_err(|_| {
                PyValueError::new_err(
                    "cloaca.constructor: grant keys must be strings (http/tcp/fs/env)",
                )
            })?;
            let patterns: Vec<String> = v.extract().map_err(|_| {
                PyValueError::new_err(format!(
                    "cloaca.constructor: grants['{kind}'] must be a list of strings"
                ))
            })?;
            grant_pairs.push((kind, patterns));
        }
    }
    let grant_spec = cloacina::registry::loader::grants::GrantSpec::from_pairs(grant_pairs);

    // runtime: the optional trust-tier pin. Validated here so a typo is a clear
    // Python-side error rather than a silent "unpinned" degradation.
    let runtime_pin = match runtime.as_deref() {
        None => None,
        Some("wasm") => Some(cloacina::registry::loader::ProviderRuntime::Wasm),
        Some("native") => Some(cloacina::registry::loader::ProviderRuntime::Native),
        Some(other) => {
            return Err(PyValueError::new_err(format!(
                "cloaca.constructor: unknown runtime '{other}'; valid values are \
                 \"wasm\" and \"native\""
            )));
        }
    };

    // dependencies: strings or function objects (same convention as @task deps).
    let mut dep_namespaces: Vec<cloacina::TaskNamespace> = Vec::new();
    for (i, dep) in dependencies.unwrap_or_default().iter().enumerate() {
        let dep_id = match dep.extract::<String>(py) {
            Ok(s) => s,
            _ => match dep.getattr(py, "__name__") {
                Ok(name) => name.extract::<String>(py).map_err(|e| {
                    PyValueError::new_err(format!(
                        "cloaca.constructor: dependency {i} __name__ is not a string: {e}"
                    ))
                })?,
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "cloaca.constructor: dependency {i} must be a string or a task function"
                    )));
                }
            },
        };
        dep_namespaces.push(cloacina::TaskNamespace::new(
            tenant_id,
            package_name,
            workflow_id,
            &dep_id,
        ));
    }

    // Resolve the member through the host loader (wasmtime compile is blocking
    // work — release the GIL for it). Fails closed on a missing provider/member,
    // a bad config kwarg, or a malformed grant.
    let node = py
        .allow_threads(|| {
            cloacina::registry::loader::load_constructor_node_pinned(
                &id,
                &from_,
                &constructor,
                config_pairs,
                dep_namespaces,
                grant_spec,
                runtime_pin,
            )
        })
        .map_err(|e| {
            PyValueError::new_err(format!(
                "cloaca.constructor: resolve '{constructor}' from provider '{from_}': {e}"
            ))
        })?;

    // Register into the current scoped Runtime — the workflow DAG assembly picks
    // it up exactly like a @cloaca.task registration.
    let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
        PyValueError::new_err(
            "cloaca.constructor called outside a Runtime scope — install a ScopedRuntime \
             before importing Python workflow modules",
        )
    })?;
    py.allow_threads(|| {
        rt.register_task(namespace, move || node.clone());
    });

    Ok(())
}
```

</details>

### <span id="fn.context_value"></span><code style="font-size: 0.85em">context_​value</code>

```python
def context_value(key: str, operator: str, value: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::context_value`](../rust/cloacina-python/trigger_rules/#fn.context_value)

A context-value condition: `context_value("key", "Equals", value)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `operator` | `str` |  |
| `value` | `PyObject` |  |

<details>
<summary>Source</summary>

```rust
pub fn context_value(
    py: Python,
    key: String,
    operator: String,
    value: PyObject,
) -> PyResult<PyObject> {
    let v: serde_json::Value = depythonize(value.bind(py)).map_err(|e| {
        PyValueError::new_err(format!("context_value: value not JSON-serializable: {e}"))
    })?;
    let op = normalize_operator(&operator)?;
    to_py(
        py,
        serde_json::json!({ "type": "ContextValue", "key": key, "operator": op, "value": v }),
    )
}
```

</details>

### <span id="fn.task_success"></span><code style="font-size: 0.85em">task_​success</code>

```python
def task_success(task_name: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::task_success`](../rust/cloacina-python/trigger_rules/#fn.task_success)

<details>
<summary>Source</summary>

```rust
pub fn task_success(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskSuccess", "task_name": task_name }),
    )
}
```

</details>

### <span id="fn.task_failed"></span><code style="font-size: 0.85em">task_​failed</code>

```python
def task_failed(task_name: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::task_failed`](../rust/cloacina-python/trigger_rules/#fn.task_failed)

<details>
<summary>Source</summary>

```rust
pub fn task_failed(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskFailed", "task_name": task_name }),
    )
}
```

</details>

### <span id="fn.task_skipped"></span><code style="font-size: 0.85em">task_​skipped</code>

```python
def task_skipped(task_name: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::task_skipped`](../rust/cloacina-python/trigger_rules/#fn.task_skipped)

<details>
<summary>Source</summary>

```rust
pub fn task_skipped(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskSkipped", "task_name": task_name }),
    )
}
```

</details>

### <span id="fn.all_of"></span><code style="font-size: 0.85em">all_​of</code>

```python
def all_of(conditions: List[PyObject]) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::all_of`](../rust/cloacina-python/trigger_rules/#fn.all_of)

Run only if ALL conditions are met.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conditions` | `List[PyObject]` |  |

<details>
<summary>Source</summary>

```rust
pub fn all_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "All", conditions)
}
```

</details>

### <span id="fn.any_of"></span><code style="font-size: 0.85em">any_​of</code>

```python
def any_of(conditions: List[PyObject]) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::any_of`](../rust/cloacina-python/trigger_rules/#fn.any_of)

Run if ANY condition is met.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conditions` | `List[PyObject]` |  |

<details>
<summary>Source</summary>

```rust
pub fn any_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "Any", conditions)
}
```

</details>

### <span id="fn.none_of"></span><code style="font-size: 0.85em">none_​of</code>

```python
def none_of(conditions: List[PyObject]) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::none_of`](../rust/cloacina-python/trigger_rules/#fn.none_of)

Run only if NONE of the conditions are met.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conditions` | `List[PyObject]` |  |

<details>
<summary>Source</summary>

```rust
pub fn none_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "None", conditions)
}
```

</details>

### <span id="fn.always"></span><code style="font-size: 0.85em">always</code>

```python
def always() -> PyObject
```

> **Rust Implementation**: [`cloacina-python::trigger_rules::always`](../rust/cloacina-python/trigger_rules/#fn.always)

The default rule — always run.

<details>
<summary>Source</summary>

```rust
pub fn always(py: Python) -> PyResult<PyObject> {
    to_py(py, serde_json::json!({ "type": "Always" }))
}
```

</details>

### <span id="fn.trigger"></span><code style="font-size: 0.85em">trigger</code>

```python
def trigger(
    name: Optional[str],
    on: Optional[str],
    poll_interval: Optional[str],
    cron: Optional[str],
    timezone: Optional[str],
    allow_concurrent: bool,
) -> TriggerDecorator
```

> **Rust Implementation**: [`cloacina-python::trigger::trigger`](../rust/cloacina-python/trigger/#fn.trigger)

`@cloaca.trigger(...)` decorator factory.

Mirrors the Rust `#[trigger]` macro (CLOACI-T-0688). Two modes:
- **Custom poll** — `poll_interval` + a function body that returns a
  `TriggerResult`.
- **Cron** — `cron` (+ optional `timezone`); the framework schedules the
  `on` workflow on that cron expression (the function body is unused). The
  reconciler routes `cron_expression`-bearing triggers to the cron scheduler.

`on` names the workflow the trigger fires; it is **required for cron**
triggers. `cron` and `poll_interval` are mutually exclusive.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `Optional[str]` |  |
| `on` | `Optional[str]` |  |
| `poll_interval` | `Optional[str]` |  |
| `cron` | `Optional[str]` |  |
| `timezone` | `Optional[str]` |  |
| `allow_concurrent` | `bool` |  |

<details>
<summary>Source</summary>

```rust
pub fn trigger(
    name: Option<String>,
    on: Option<String>,
    poll_interval: Option<String>,
    cron: Option<String>,
    timezone: Option<String>,
    allow_concurrent: bool,
) -> PyResult<TriggerDecorator> {
    // Mirror the Rust macro's validation.
    if cron.is_some() && poll_interval.is_some() {
        return Err(PyValueError::new_err(
            "@cloaca.trigger cannot set both 'cron' and 'poll_interval' — pick one",
        ));
    }
    if cron.is_some() && on.is_none() {
        return Err(PyValueError::new_err(
            "@cloaca.trigger(cron=…) requires 'on' (the workflow the cron schedule fires)",
        ));
    }

    // Poll interval still defaults to 30s for the poll path; ignored for cron.
    let interval_str = poll_interval.unwrap_or_else(|| "30s".to_string());
    let interval = parse_duration_str(&interval_str).map_err(|e| {
        PyValueError::new_err(format!("Invalid poll_interval '{}': {}", interval_str, e))
    })?;

    Ok(TriggerDecorator {
        name,
        poll_interval: interval,
        allow_concurrent,
        workflow_name: on.unwrap_or_default(),
        cron_expression: cron,
        timezone,
    })
}
```

</details>

### <span id="fn.reactor"></span><code style="font-size: 0.85em">reactor</code>

```python
def reactor(name: str, accumulators: List[str], mode: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::reactor::reactor`](../rust/cloacina-python/reactor/#fn.reactor)

`@cloaca.reactor(name=..., accumulators=[...], mode="when_any"|"when_all")`.

Returns a decorator that, when applied to a class, validates the inputs,
sets `NAME`/`ACCUMULATORS`/`REACTION_MODE` on the class, and registers a
`ReactorRegistration` into the active scoped Runtime.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `accumulators` | `List[str]` |  |
| `mode` | `str` |  |

<details>
<summary>Source</summary>

```rust
pub fn reactor(
    py: Python<'_>,
    name: String,
    accumulators: Vec<String>,
    mode: String,
) -> PyResult<PyObject> {
    if name.is_empty() {
        return Err(PyValueError::new_err(
            "@cloaca.reactor: 'name' cannot be empty",
        ));
    }
    if accumulators.is_empty() {
        return Err(PyValueError::new_err(
            "@cloaca.reactor: 'accumulators' list cannot be empty",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for acc in &accumulators {
        if !seen.insert(acc.clone()) {
            return Err(PyValueError::new_err(format!(
                "@cloaca.reactor: accumulator '{}' listed more than once",
                acc
            )));
        }
    }
    let reaction_mode = match mode.as_str() {
        "when_any" => ReactionMode::WhenAny,
        "when_all" => ReactionMode::WhenAll,
        other => {
            return Err(PyValueError::new_err(format!(
                "@cloaca.reactor: unknown mode '{}', expected 'when_any' or 'when_all'",
                other
            )));
        }
    };

    // Capture state for the decorator closure.
    let name_clone = name.clone();
    let accumulators_clone = accumulators.clone();

    let decorator = pyo3::types::PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>|
              -> PyResult<PyObject> {
            let py = args.py();
            let cls_obj = args.get_item(0)?;

            // Require a class (type). Reject instances / functions to give a clear error.
            let cls = cls_obj.downcast::<PyType>().map_err(|_| {
                PyTypeError::new_err(
                    "@cloaca.reactor must be applied to a class (e.g. `class Rx: pass`)",
                )
            })?;

            // Set class attributes — these mirror Rust's `Reactor` trait associated consts.
            cls.setattr("NAME", PyString::new(py, &name_clone))?;
            cls.setattr("ACCUMULATORS", PyList::new(py, &accumulators_clone)?)?;
            cls.setattr("REACTION_MODE", PyString::new(py, reaction_mode.as_str()))?;

            // Register into the active scoped Runtime.
            let rt = current_runtime().ok_or_else(|| {
                PyValueError::new_err(
                    "@cloaca.reactor: no scoped Runtime installed — \
                     decorator must run inside a process or loader thread \
                     that has installed a ScopedRuntime",
                )
            })?;

            let reg_name = name_clone.clone();
            let reg_accs = accumulators_clone.clone();
            let reg_mode = reaction_mode;
            rt.register_reactor(reg_name.clone(), move || {
                // CLOACI-T-0839: carry the module's authored accumulator kinds
                // (`@cloaca.state_accumulator(capacity=N)` etc.) on the
                // registration, so runtime-walking load paths (packaged Python
                // via the reconciler) spawn the right accumulator runtime
                // instead of defaulting to passthrough. Resolved lazily — by
                // the time anyone calls `get_reactor`, all module decorators
                // have run regardless of declaration order.
                let accumulator_specs = crate::computation_graph::get_registered_accumulators()
                    .into_iter()
                    .filter(|areg| reg_accs.contains(&areg.name))
                    .map(|areg| {
                        let mut config = areg.config.clone();
                        // state capacity rides the config map (the same key
                        // `state_capacity_from_config` reads host-side).
                        if areg.accumulator_type == "state" {
                            config
                                .entry("capacity".to_string())
                                .or_insert_with(|| areg.capacity.to_string());
                        }
                        cloacina_computation_graph::AccumulatorSpec {
                            name: areg.name,
                            accumulator_type: areg.accumulator_type,
                            config,
                        }
                    })
                    .collect();
                ReactorRegistration {
                    name: reg_name.clone(),
                    accumulator_names: reg_accs.clone(),
                    accumulator_specs,
                    reaction_mode: reg_mode,
                    // CLOACI-T-0830: the Python reactor path doesn't (yet) author
                    // WASM reactor constructors — native dirty-flag firing only.
                    constructor: None,
                }
            });

            tracing::debug!(
                "Registered Python reactor: name={} accumulators={:?} mode={}",
                name_clone,
                accumulators_clone,
                reaction_mode.as_str()
            );

            Ok(cls.clone().unbind().into())
        },
    )?;

    Ok(decorator.into())
}
```

</details>

### <span id="fn.register_workflow_constructor"></span><code style="font-size: 0.85em">register_​workflow_​constructor</code>

```python
def register_workflow_constructor(name: str, constructor: PyObject) -> None
```

> **Rust Implementation**: [`cloacina-python::workflow::py_register_workflow`](../rust/cloacina-python/workflow/#fn.py_register_workflow)

Register a workflow constructor function on the thread-local Runtime scope. Exported to Python as `register_workflow_constructor` for backwards compatibility; the Rust symbol name is `py_register_workflow` so it does not collide with the (now deleted) `cloacina::register_workflow_constructor`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `constructor` | `PyObject` |  |

<details>
<summary>Source</summary>

```rust
pub fn py_register_workflow(name: String, constructor: PyObject) -> PyResult<()> {
    Python::with_gil(|py| {
        let workflow_obj = constructor.call0(py).map_err(|e| {
            PyValueError::new_err(format!("Failed to call workflow constructor: {}", e))
        })?;

        let py_workflow: PyWorkflow = workflow_obj.extract(py).map_err(|e| {
            PyValueError::new_err(format!(
                "Failed to extract workflow from constructor: {}",
                e
            ))
        })?;

        let workflow = py_workflow.inner.clone();
        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err("register_workflow_constructor called outside a Runtime scope")
        })?;
        rt.register_workflow(name, move || workflow.clone());

        Ok(())
    })
}
```

</details>

### <span id="fn.workflow_params"></span><code style="font-size: 0.85em">workflow_​params</code>

```python
def workflow_params(
    _kwargs: Optional[Bound<'_, pyo3::types::PyDict>],
) -> PyWorkflowParamsDecorator
```

> **Rust Implementation**: [`cloacina-python::workflow::workflow_params`](../rust/cloacina-python/workflow/#fn.workflow_params)

`@cloaca.workflow_params(name=type, name=(type, default), ...)` — declares a packaged Python workflow's typed input params (CLOACI-I-0128 / T-0760).

Parity with Rust's `#[workflow(params(...))]`. The authoritative extraction
happens at build time (the compiler parses this decorator from source into
JSON-Schema-typed `InputSlot`s); at runtime it is a no-op pass-through so the
decorated function/workflow is unaffected. Supported scalar types: `str`,
`int`, `float`, `bool`, `list`, `dict`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_kwargs` | `Optional[Bound<'_, pyo3::types::PyDict>]` |  |

<details>
<summary>Source</summary>

```rust
pub fn workflow_params(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyWorkflowParamsDecorator {
    PyWorkflowParamsDecorator
}
```

</details>

### <span id="fn.workflow_secrets"></span><code style="font-size: 0.85em">workflow_​secrets</code>

```python
def workflow_secrets(
    _args: Bound<'_, pyo3::types::PyTuple>,
) -> PyWorkflowSecretsDecorator
```

> **Rust Implementation**: [`cloacina-python::workflow::workflow_secrets`](../rust/cloacina-python/workflow/#fn.workflow_secrets)

`@cloaca.workflow_secrets("db_prod", "api_token", ...)` — declares the SECRETS a packaged Python workflow requires (CLOACI-I-0133 / T-0859), the parity of Rust's `#[workflow(secrets(...))]`. Each name becomes a required encrypted `InputSlot` (`encrypted: true`) carried in the package manifest alongside the declared params; a secret is bound per-instance via a `{"$secret": "name"}` reference and resolved encrypted at fire time (its value never enters the durable `Context`). The authoritative extraction happens at build time (the compiler parses this decorator from source); at runtime it is a no-op pass-through.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_args` | `Bound<'_, pyo3::types::PyTuple>` |  |

<details>
<summary>Source</summary>

```rust
pub fn workflow_secrets(_args: Bound<'_, pyo3::types::PyTuple>) -> PyWorkflowSecretsDecorator {
    PyWorkflowSecretsDecorator
}
```

</details>

### <span id="fn.boundary_schema"></span><code style="font-size: 0.85em">boundary_​schema</code>

```python
def boundary_schema(
    _kwargs: Optional[Bound<'_, pyo3::types::PyDict>],
) -> PyBoundarySchemaDecorator
```

> **Rust Implementation**: [`cloacina-python::workflow::boundary_schema`](../rust/cloacina-python/workflow/#fn.boundary_schema)

`@cloaca.boundary_schema(field=type, ...)` — declares the typed shape of a Python accumulator's boundary (CLOACI-T-0770), the parity of deriving `schemars::JsonSchema` on a Rust boundary type. The compiler parses this from source at build time into the accumulator's declared input-interface slot (so inject/fire-with render typed forms); at runtime it's a no-op pass-through. Applied to the accumulator function:

```python
@cloaca.boundary_schema(bid=float, ask=float)
@cloaca.passthrough_accumulator
def py_alpha(event): ...
```

Supported scalar types: `str`, `int`, `float`, `bool`, `list`, `dict`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_kwargs` | `Optional[Bound<'_, pyo3::types::PyDict>]` |  |

<details>
<summary>Source</summary>

```rust
pub fn boundary_schema(
    _kwargs: Option<Bound<'_, pyo3::types::PyDict>>,
) -> PyBoundarySchemaDecorator {
    PyBoundarySchemaDecorator
}
```

</details>

### <span id="fn.node"></span><code style="font-size: 0.85em">node</code>

```python
def node(func: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::node`](../rust/cloacina-python/computation_graph/#fn.node)

The `@cloaca.node` decorator. Registers a function as a node in the current ComputationGraphBuilder context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `func` | `PyObject` |  |

<details>
<summary>Source</summary>

```rust
pub fn node(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let _ctx = current_graph_context().ok_or_else(|| {
        PyValueError::new_err(
            "@cloaca.node must be used inside a ComputationGraphBuilder context manager",
        )
    })?;

    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    register_node(func_name, func.clone_ref(py));

    // Return the function unchanged (transparent decorator)
    Ok(func)
}
```

</details>

### <span id="fn.passthrough_accumulator_decorator"></span><code style="font-size: 0.85em">passthrough_​accumulator_​decorator</code>

```python
def passthrough_accumulator_decorator(func: PyObject) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::passthrough_accumulator_decorator`](../rust/cloacina-python/computation_graph/#fn.passthrough_accumulator_decorator)

The `@cloaca.passthrough_accumulator` decorator. Registers a function as a passthrough accumulator (Event → Output, no buffering).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `func` | `PyObject` |  |

<details>
<summary>Source</summary>

```rust
pub fn passthrough_accumulator_decorator(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    let reg = PyAccumulatorRegistration {
        name: func_name.clone(),
        accumulator_type: "passthrough".to_string(),
        config: HashMap::new(),
        capacity: 0,
    };
    register_accumulator(func_name, func.clone_ref(py), reg);
    Ok(func)
}
```

</details>

### <span id="fn.stream_accumulator_decorator"></span><code style="font-size: 0.85em">stream_​accumulator_​decorator</code>

```python
def stream_accumulator_decorator(
    r#type: str,
    topic: str,
    group: Optional[str],
) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::stream_accumulator_decorator`](../rust/cloacina-python/computation_graph/#fn.stream_accumulator_decorator)

Factory for `@cloaca.stream_accumulator(type=..., topic=...)`. Returns a decorator function that registers the target with stream config.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `r#type` | `str` |  |
| `topic` | `str` |  |
| `group` | `Optional[str]` |  |

<details>
<summary>Source</summary>

```rust
pub fn stream_accumulator_decorator(
    py: Python<'_>,
    r#type: String,
    topic: String,
    group: Option<String>,
) -> PyResult<PyObject> {
    let config_type = r#type;
    let config_topic = topic;
    let config_group = group;

    // Return a decorator function
    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("type".to_string(), config_type.clone());
            config.insert("topic".to_string(), config_topic.clone());
            if let Some(ref g) = config_group {
                config.insert("group".to_string(), g.clone());
            }

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "stream".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>

### <span id="fn.polling_accumulator_decorator"></span><code style="font-size: 0.85em">polling_​accumulator_​decorator</code>

```python
def polling_accumulator_decorator(interval: str) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::polling_accumulator_decorator`](../rust/cloacina-python/computation_graph/#fn.polling_accumulator_decorator)

Factory for `@cloaca.polling_accumulator(interval=...)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `interval` | `str` |  |

<details>
<summary>Source</summary>

```rust
pub fn polling_accumulator_decorator(py: Python<'_>, interval: String) -> PyResult<PyObject> {
    let config_interval = interval;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("interval".to_string(), config_interval.clone());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "polling".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>

### <span id="fn.batch_accumulator_decorator"></span><code style="font-size: 0.85em">batch_​accumulator_​decorator</code>

```python
def batch_accumulator_decorator(
    flush_interval: str,
    max_buffer_size: Optional[int],
) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::batch_accumulator_decorator`](../rust/cloacina-python/computation_graph/#fn.batch_accumulator_decorator)

Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `flush_interval` | `str` |  |
| `max_buffer_size` | `Optional[int]` |  |

<details>
<summary>Source</summary>

```rust
pub fn batch_accumulator_decorator(
    py: Python<'_>,
    flush_interval: String,
    max_buffer_size: Option<usize>,
) -> PyResult<PyObject> {
    let config_interval = flush_interval;
    let config_max = max_buffer_size;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("flush_interval".to_string(), config_interval.clone());
            if let Some(max) = config_max {
                config.insert("max_buffer_size".to_string(), max.to_string());
            }

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "batch".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>

### <span id="fn.state_accumulator_decorator"></span><code style="font-size: 0.85em">state_​accumulator_​decorator</code>

```python
def state_accumulator_decorator(capacity: int) -> PyObject
```

> **Rust Implementation**: [`cloacina-python::computation_graph::state_accumulator_decorator`](../rust/cloacina-python/computation_graph/#fn.state_accumulator_decorator)

Factory for `@cloaca.state_accumulator(capacity=N)`.

Registers a function as a *state* accumulator: a bounded `VecDeque` that
receives values, persists to the DAL on every write, and emits the full
list back as the boundary (enabling cyclic feedback patterns). Mirrors
Rust's `#[state_accumulator(capacity=…)]`.

Capacity semantics (see `StateAccumulator` in cloacina core):
- `> 0`: bounded — evicts oldest when at capacity
- `< 0`: unbounded — grows without limit
- `0`:  write-only sink — no history emitted back

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `capacity` | `int` |  |

<details>
<summary>Source</summary>

```rust
pub fn state_accumulator_decorator(py: Python<'_>, capacity: i32) -> PyResult<PyObject> {
    let config_capacity = capacity;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            // Mirror capacity into config too, so it survives the
            // String-keyed config map used at the FFI/override boundary.
            let mut config = HashMap::new();
            config.insert("capacity".to_string(), config_capacity.to_string());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "state".to_string(),
                config,
                capacity: config_capacity,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>

### <span id="fn.py_var"></span><code style="font-size: 0.85em">py_​var</code>

```python
def py_var(name: str) -> str
```

> **Rust Implementation**: [`cloacina-python::loader::py_var`](../rust/cloacina-python/loader/#fn.py_var)

Python binding: `cloaca.var(name)` — resolve a `CLOACINA_VAR_{NAME}` env var.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |

<details>
<summary>Source</summary>

```rust
pub(crate) fn py_var(name: &str) -> PyResult<String> {
    cloacina::var(name).map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
}
```

</details>

### <span id="fn.py_var_or"></span><code style="font-size: 0.85em">py_​var_​or</code>

```python
def py_var_or(name: str, default: str) -> str
```

> **Rust Implementation**: [`cloacina-python::loader::py_var_or`](../rust/cloacina-python/loader/#fn.py_var_or)

Python binding: `cloaca.var_or(name, default)` — resolve with a fallback.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `str` |  |
| `default` | `str` |  |

<details>
<summary>Source</summary>

```rust
pub(crate) fn py_var_or(name: &str, default: &str) -> String {
    cloacina::var_or(name, default)
}
```

</details>
