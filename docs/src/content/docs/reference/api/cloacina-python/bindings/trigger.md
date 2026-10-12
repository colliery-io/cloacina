---
title: "cloacina-python.bindings.trigger"
description: "Python trigger support for event-driven workflow execution."
sidebar:
  order: 4
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


Python trigger support for event-driven workflow execution.

This module provides Python bindings for defining triggers that poll
user-defined conditions and fire workflows when those conditions are met.

## Classes

### <span id="class.TriggerResult"></span>`class TriggerResult`

> **Rust Implementation**: [cloacina-python::bindings::trigger::PyTriggerResult](../../../rust/cloacina-python/bindings/trigger/#struct.PyTriggerResult)

Python TriggerResult class - represents the result of a trigger poll.

Use `TriggerResult.skip()` when the condition is not met.
Use `TriggerResult.fire(context=None)` when the condition is met.

#### Methods

##### <span id="TriggerResult.skip"></span>`skip`

```python
def skip() -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::trigger::PyTriggerResult::skip](../../../rust/cloacina-python/bindings/trigger/#PyTriggerResult.skip)

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



##### <span id="TriggerResult.fire"></span>`fire`

```python
def fire(context: Option < & PyContext >) -> Self
```

> **Rust Implementation**: [cloacina-python::bindings::trigger::PyTriggerResult::fire](../../../rust/cloacina-python/bindings/trigger/#PyTriggerResult.fire)

Create a Fire result - condition met, trigger the workflow.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `Option < & PyContext >` | Optional context to pass to the workflow |


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



##### <span id="TriggerResult.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::bindings::trigger::PyTriggerResult::__repr__](../../../rust/cloacina-python/bindings/trigger/#PyTriggerResult.__repr__)

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



##### <span id="TriggerResult.is_fire_result"></span>`is_fire_result`

```python
def is_fire_result() -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::trigger::PyTriggerResult::is_fire_result](../../../rust/cloacina-python/bindings/trigger/#PyTriggerResult.is_fire_result)

Check if this is a Fire result

<details>
<summary>Source</summary>

```python
    fn is_fire_result(&self) -> bool {
        self.is_fire
    }
```

</details>



##### <span id="TriggerResult.is_skip_result"></span>`is_skip_result`

```python
def is_skip_result() -> bool
```

> **Rust Implementation**: [cloacina-python::bindings::trigger::PyTriggerResult::is_skip_result](../../../rust/cloacina-python/bindings/trigger/#PyTriggerResult.is_skip_result)

Check if this is a Skip result

<details>
<summary>Source</summary>

```python
    fn is_skip_result(&self) -> bool {
        !self.is_fire
    }
```

</details>





### <span id="class.TriggerDecorator"></span>`class TriggerDecorator`

> **Rust Implementation**: [cloacina-python::trigger::TriggerDecorator](../../../rust/cloacina-python/trigger/#struct.TriggerDecorator)

Decorator class that holds trigger configuration

#### Methods

##### <span id="TriggerDecorator.__call__"></span>`__call__`

```python
def __call__(func: PyObject) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger::TriggerDecorator::__call__](../../../rust/cloacina-python/trigger/#TriggerDecorator.__call__)

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

        // Register trigger constructor in the scoped Runtime
        let rt = crate::runtime_scope::current_runtime().ok_or_else(|| {
            PyValueError::new_err("@trigger decorator called outside a Runtime scope")
        })?;
        rt.register_trigger(trigger_name.clone(), move || {
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

### <span id="fn.trigger"></span>`trigger`

```python
def trigger(
    workflow: str,
    name: Option < String >,
    poll_interval:  str,
    allow_concurrent: bool,
) -> PyResult < TriggerDecorator >
```

> **Rust Implementation**: [cloacina-python::trigger::trigger](../../../rust/cloacina-python/trigger/#fn.trigger)

Python @trigger decorator function

This function is exposed to Python as a decorator that registers
Python functions as triggers in the Cloacina trigger scheduler.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `workflow` | `str` |  |
| `name` | `Option < String >` |  |
| `poll_interval` | ` str` |  |
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



