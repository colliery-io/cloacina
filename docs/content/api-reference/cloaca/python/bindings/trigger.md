# cloaca.python.bindings.trigger <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


Python trigger support for event-driven workflow execution.

This module provides Python bindings for defining triggers that poll
user-defined conditions and fire workflows when those conditions are met.

## Classes

### `cloaca.python.bindings.trigger.TriggerResult`

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult](../../../rust/cloacina/python/trigger.md#class-triggerresult)

Python TriggerResult class - represents the result of a trigger poll.

Use `TriggerResult.skip()` when the condition is not met.
Use `TriggerResult.fire(context=None)` when the condition is met.

#### Methods

##### `skip`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">skip</span>() -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::skip](../../../rust/cloacina/python/trigger.md#skip)

Create a Skip result - condition not met, continue polling.

<details>
<summary>Source</summary>

```python
    fn skip() -> Self {
        PyTriggerResult {
            is_fire: false,
            data: None,
        }
    }
```

</details>



##### `fire`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">fire</span>(context: Optional[PyContext]) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::fire](../../../rust/cloacina/python/trigger.md#fire)

Create a Fire result - condition met, trigger the workflow.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `Optional[PyContext]` | Optional context to pass to the workflow |


<details>
<summary>Source</summary>

```python
    fn fire(context: Option<&PyContext>) -> Self {
        let data = context.map(|c| c.get_data_clone());
        PyTriggerResult {
            is_fire: true,
            data,
        }
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::__repr__](../../../rust/cloacina/python/trigger.md#__repr__)

<details>
<summary>Source</summary>

```python
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



##### `is_fire_result`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">is_fire_result</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::is_fire_result](../../../rust/cloacina/python/trigger.md#is_fire_result)

Check if this is a Fire result

<details>
<summary>Source</summary>

```python
    fn is_fire_result(&self) -> bool {
        self.is_fire
    }
```

</details>



##### `is_skip_result`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">is_skip_result</span>() -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::PyTriggerResult::is_skip_result](../../../rust/cloacina/python/trigger.md#is_skip_result)

Check if this is a Skip result

<details>
<summary>Source</summary>

```python
    fn is_skip_result(&self) -> bool {
        !self.is_fire
    }
```

</details>





### `cloaca.python.bindings.trigger.TriggerDecorator`

> **Rust Implementation**: [cloacina::python::trigger::TriggerDecorator](../../../rust/cloacina/python/trigger.md#class-triggerdecorator)

Decorator class that holds trigger configuration

#### Methods

##### `__call__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__call__</span>(func: Any) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::TriggerDecorator::__call__](../../../rust/cloacina/python/trigger.md#__call__)

<details>
<summary>Source</summary>

```python
    pub fn __call__(&self, py: Python, func: PyObject) -> PyResult<PyObject> {
        // Determine trigger name - use provided name or derive from function name
        let trigger_name = if let Some(name) = &self.name {
            name.clone()
        } else {
            func.getattr(py, "__name__")?.extract::<String>(py)?
        };

        // Store values for the closure
        let workflow_name = self.workflow.clone();
        let poll_interval = self.poll_interval;
        let allow_concurrent = self.allow_concurrent;
        let name_for_constructor = trigger_name.clone();

        // Create Arc'd function for sharing with constructor
        let shared_function = Arc::new(func.clone_ref(py));

        // Register trigger constructor in the global registry
        crate::trigger::register_trigger_constructor(trigger_name.clone(), move || {
            let function_clone = Python::with_gil(|py| (*shared_function).clone_ref(py));
            Arc::new(PythonTriggerWrapper {
                name: name_for_constructor.clone(),
                workflow_name: workflow_name.clone(),
                poll_interval,
                allow_concurrent,
                python_function: function_clone,
            }) as Arc<dyn Trigger>
        });

        tracing::info!(
            trigger_name = %trigger_name,
            workflow = %self.workflow,
            poll_interval_ms = %self.poll_interval.as_millis(),
            "Registered Python trigger"
        );

        // Return the original function
        Ok(func)
    }
```

</details>





## Functions

### `cloaca.python.bindings.trigger.trigger`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">trigger</span>(workflow: str, name: Optional[str], poll_interval: str, allow_concurrent: bool) -> <span style="color: var(--md-default-fg-color--light);">TriggerDecorator</span></code>
</div>

> **Rust Implementation**: [cloacina::python::trigger::trigger](../../../rust/cloacina/python/trigger.md#fn-trigger)

Python @trigger decorator function

This function is exposed to Python as a decorator that registers
Python functions as triggers in the Cloacina trigger scheduler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow` | `str` |  |
| `name` | `Optional[str]` |  |
| `poll_interval` | `str` |  |
| `allow_concurrent` | `bool` |  |


**Examples:**

```python
import cloaca
import random

@cloaca.trigger(
    workflow="my_workflow",
    poll_interval="5s",
    allow_concurrent=False
)
def my_trigger():
    # Check some condition
    if random.randint(1, 100) == 42:
        ctx = cloaca.Context({"triggered_at": "now"})
        return cloaca.TriggerResult.fire(ctx)
    return cloaca.TriggerResult.skip()
```

<details>
<summary>Source</summary>

```python
pub fn trigger(
    workflow: String,
    name: Option<String>,
    poll_interval: &str,
    allow_concurrent: bool,
) -> PyResult<TriggerDecorator> {
    let duration = parse_duration(poll_interval)
        .map_err(|e| PyValueError::new_err(format!("Invalid poll_interval: {}", e)))?;

    Ok(TriggerDecorator {
        name,
        workflow,
        poll_interval: duration,
        allow_concurrent,
    })
}
```

</details>
