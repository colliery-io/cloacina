# cloacina::python::trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Python trigger bindings via PyO3.

Provides:
- `@cloaca.trigger` decorator for defining custom Python triggers
- `TriggerResult` Python class for returning poll results
- `PythonTriggerWrapper` implementing the Rust `Trigger` trait

## Structs

### `cloacina::python::trigger::PythonTriggerDef`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


A collected Python trigger definition.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |
| `python_function` | `PyObject` |  |



### `cloacina::python::trigger::TriggerResult`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.trigger.TriggerResult](../../../cloaca/python/trigger.md#class-triggerresult)

Python-side trigger result returned from poll functions.

Usage from Python:
```python
from cloaca import TriggerResult
@cloaca.trigger(name="my_trigger", poll_interval="10s")
def my_trigger():
if some_condition():
return TriggerResult(should_fire=True, context={"key": "value"})
return TriggerResult(should_fire=False)
```

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `should_fire` | `bool` |  |
| `context` | `Option < PyObject >` |  |

#### Methods

##### `new`

```rust
fn new (should_fire : bool , context : Option < PyObject >) -> Self
```

> **Python API**: [cloaca.python.trigger.TriggerResult.new](../../../cloaca/python/trigger.md#new)

<details>
<summary>Source</summary>

```rust
    fn new(should_fire: bool, context: Option<PyObject>) -> Self {
        Self {
            should_fire,
            context,
        }
    }
```

</details>



##### `__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.trigger.TriggerResult.__repr__](../../../cloaca/python/trigger.md#__repr__)

<details>
<summary>Source</summary>

```rust
    fn __repr__(&self) -> String {
        if self.should_fire {
            "TriggerResult(should_fire=True)".to_string()
        } else {
            "TriggerResult(should_fire=False)".to_string()
        }
    }
```

</details>





### `cloacina::python::trigger::TriggerDecorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.trigger.TriggerDecorator](../../../cloaca/python/trigger.md#class-triggerdecorator)

Decorator for defining Python triggers.

```python
@cloaca.trigger(name="check_inbox", poll_interval="30s")
def check_inbox():

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `Option < String >` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |

#### Methods

##### `__call__`

```rust
fn __call__ (& self , py : Python , func : PyObject) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.trigger.TriggerDecorator.__call__](../../../cloaca/python/trigger.md#__call__)

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
        };

        PYTHON_TRIGGER_REGISTRY.lock().push(def);

        tracing::debug!("Registered Python trigger: {}", trigger_name);

        // Return the original function (decorator is transparent)
        Ok(func)
    }
```

</details>





### `cloacina::python::trigger::PythonTriggerWrapper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


Rust wrapper that implements the `Trigger` trait by calling a Python function.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |
| `python_function` | `PyObject` |  |

#### Methods

##### `new` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn new (def : & PythonTriggerDef) -> Self
```

<details>
<summary>Source</summary>

```rust
    pub fn new(def: &PythonTriggerDef) -> Self {
        let function = Python::with_gil(|py| def.python_function.clone_ref(py));
        Self {
            name: def.name.clone(),
            poll_interval: def.poll_interval,
            allow_concurrent: def.allow_concurrent,
            python_function: function,
        }
    }
```

</details>





## Functions

### `cloacina::python::trigger::drain_python_triggers`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn drain_python_triggers () -> Vec < PythonTriggerDef >
```

Collect all registered Python triggers and clear the registry.

<details>
<summary>Source</summary>

```rust
pub fn drain_python_triggers() -> Vec<PythonTriggerDef> {
    let mut registry = PYTHON_TRIGGER_REGISTRY.lock();
    std::mem::take(&mut *registry)
}
```

</details>



### `cloacina::python::trigger::trigger`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.trigger.trigger](../../../cloaca/python/trigger.md#trigger)

```rust
fn trigger (name : Option < String > , poll_interval : String , allow_concurrent : bool ,) -> PyResult < TriggerDecorator >
```

`@cloaca.trigger(...)` decorator factory.

<details>
<summary>Source</summary>

```rust
pub fn trigger(
    name: Option<String>,
    poll_interval: String,
    allow_concurrent: bool,
) -> PyResult<TriggerDecorator> {
    let interval = parse_duration_str(&poll_interval).map_err(|e| {
        PyValueError::new_err(format!("Invalid poll_interval '{}': {}", poll_interval, e))
    })?;

    Ok(TriggerDecorator {
        name,
        poll_interval: interval,
        allow_concurrent,
    })
}
```

</details>
