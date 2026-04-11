# cloaca.python.trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


Python trigger bindings via PyO3.

Provides:
- `@cloaca.trigger` decorator for defining custom Python triggers
- `TriggerResult` Python class for returning poll results
- `PythonTriggerWrapper` implementing the Rust `Trigger` trait

## Classes

### `cloaca.python.trigger.TriggerResult`

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult](../../rust/cloacina/python/trigger.md#class-triggerresult)

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

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(should_fire: bool, context: Optional[Any]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::new](../../rust/cloacina/python/trigger.md#new)

<details>
<summary>Source</summary>

```python
    fn new(should_fire: bool, context: Option<PyObject>) -> Self {
        Self {
            should_fire,
            context,
        }
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::__repr__](../../rust/cloacina/python/trigger.md#__repr__)

<details>
<summary>Source</summary>

```python
    fn __repr__(&self) -> String {
        if self.should_fire {
            "TriggerResult(should_fire=True)".to_string()
        } else {
            "TriggerResult(should_fire=False)".to_string()
        }
    }
```

</details>





### `cloaca.python.trigger.TriggerDecorator`

> **Rust Implementation**: [cloacina::python::trigger::TriggerDecorator](../../rust/cloacina/python/trigger.md#class-triggerdecorator)

Decorator for defining Python triggers.

```python
@cloaca.trigger(name="check_inbox", poll_interval="30s")
def check_inbox():
# Return TriggerResult(should_fire=True, context={...}) to fire
return TriggerResult(should_fire=False)
```

#### Methods

##### `__call__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__call__</span>(func: Any) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::TriggerDecorator::__call__](../../rust/cloacina/python/trigger.md#__call__)

<details>
<summary>Source</summary>

```python
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





## Functions

### `cloaca.python.trigger.trigger`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">trigger</span>(name: Optional[str], poll_interval: str, allow_concurrent: bool) -> <span style="color: var(--md-default-fg-color--light);">TriggerDecorator</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::trigger](../../rust/cloacina/python/trigger.md#fn-trigger)

`@cloaca.trigger(...)` decorator factory.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `name` | `Optional[str]` |  |
| `poll_interval` | `str` |  |
| `allow_concurrent` | `bool` |  |


<details>
<summary>Source</summary>

```python
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
