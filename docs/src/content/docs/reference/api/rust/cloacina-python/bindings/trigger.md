---
title: "cloacina-python::bindings::trigger"
description: "Python trigger support for event-driven workflow execution."
sidebar:
  order: 24
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python trigger support for event-driven workflow execution.

This module provides Python bindings for defining triggers that poll
user-defined conditions and fire workflows when those conditions are met.

## Structs

### <span id="struct.PyTriggerResult"></span>`class TriggerResult`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.bindings.trigger.TriggerResult](../../../../cloacina-python/bindings/trigger/#class.TriggerResult)

Python TriggerResult class - represents the result of a trigger poll.

Use `TriggerResult.skip()` when the condition is not met.
Use `TriggerResult.fire(context=None)` when the condition is met.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `is_fire` | `bool` |  |
| `data` | `Option < std :: collections :: HashMap < String , Value > >` |  |

#### Methods

##### <span id="PyTriggerResult.skip"></span>`skip`

```rust
fn skip () -> Self
```

> **Python API**: [cloacina-python.bindings.trigger.TriggerResult.skip](../../../../cloacina-python/bindings/trigger/#TriggerResult.skip)

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



##### <span id="PyTriggerResult.fire"></span>`fire`

```rust
fn fire (context : Option < & PyContext >) -> Self
```

> **Python API**: [cloacina-python.bindings.trigger.TriggerResult.fire](../../../../cloacina-python/bindings/trigger/#TriggerResult.fire)

Create a Fire result - condition met, trigger the workflow.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `context` | `-` | Optional context to pass to the workflow |


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



##### <span id="PyTriggerResult.__repr__"></span>`__repr__`

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloacina-python.bindings.trigger.TriggerResult.__repr__](../../../../cloacina-python/bindings/trigger/#TriggerResult.__repr__)

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



##### <span id="PyTriggerResult.is_fire_result"></span>`is_fire_result`

```rust
fn is_fire_result (& self) -> bool
```

> **Python API**: [cloacina-python.bindings.trigger.TriggerResult.is_fire_result](../../../../cloacina-python/bindings/trigger/#TriggerResult.is_fire_result)

Check if this is a Fire result

<details>
<summary>Source</summary>

```rust
    fn is_fire_result(&self) -> bool {
        self.is_fire
    }
```

</details>



##### <span id="PyTriggerResult.is_skip_result"></span>`is_skip_result`

```rust
fn is_skip_result (& self) -> bool
```

> **Python API**: [cloacina-python.bindings.trigger.TriggerResult.is_skip_result](../../../../cloacina-python/bindings/trigger/#TriggerResult.is_skip_result)

Check if this is a Skip result

<details>
<summary>Source</summary>

```rust
    fn is_skip_result(&self) -> bool {
        !self.is_fire
    }
```

</details>



##### <span id="PyTriggerResult.into_rust"></span>`into_rust`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn into_rust (self) -> TriggerResult
```

Convert to Rust TriggerResult by consuming self.

<details>
<summary>Source</summary>

```rust
    pub fn into_rust(self) -> TriggerResult {
        if !self.is_fire {
            TriggerResult::Skip
        } else {
            let ctx = self.data.map(|d| {
                let mut context = Context::new();
                for (key, value) in d {
                    context.insert(key, value).ok();
                }
                context
            });
            TriggerResult::Fire(ctx)
        }
    }
```

</details>



##### <span id="PyTriggerResult.clone_into_rust"></span>`clone_into_rust`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn clone_into_rust (& self) -> TriggerResult
```

Convert to Rust TriggerResult by cloning self (for &self callers such as the cross-API loader-side `PythonTriggerWrapper.poll`).

<details>
<summary>Source</summary>

```rust
    pub fn clone_into_rust(&self) -> TriggerResult {
        if !self.is_fire {
            TriggerResult::Skip
        } else {
            let ctx = self.data.clone().map(|d| {
                let mut context = Context::new();
                for (key, value) in d {
                    context.insert(key, value).ok();
                }
                context
            });
            TriggerResult::Fire(ctx)
        }
    }
```

</details>





### <span id="struct.PythonTriggerWrapper"></span>`struct PythonTriggerWrapper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Python trigger wrapper implementing Rust Trigger trait.

This struct allows Python functions to be registered and executed
as triggers within the Cloacina trigger scheduler.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `workflow_name` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |
| `python_function` | `PyObject` |  |

#### Methods

##### <span id="PythonTriggerWrapper.workflow_name"></span>`workflow_name`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn workflow_name (& self) -> & str
```

Get the workflow name this trigger is associated with

<details>
<summary>Source</summary>

```rust
    pub fn workflow_name(&self) -> &str {
        &self.workflow_name
    }
```

</details>





### <span id="struct.TriggerDecorator"></span>`class TriggerDecorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger.TriggerDecorator](../../../../cloacina-python/trigger/#class.TriggerDecorator)

Decorator class that holds trigger configuration

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `Option < String >` |  |
| `workflow` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |

#### Methods

##### <span id="TriggerDecorator.__call__"></span>`__call__`

```rust
fn __call__ (& self , py : Python , func : PyObject) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.trigger.TriggerDecorator.__call__](../../../../cloacina-python/trigger/#TriggerDecorator.__call__)

<details>
<summary>Source</summary>

```rust
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

### <span id="fn.parse_duration"></span>`fn parse_duration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn parse_duration (s : & str) -> Result < Duration , String >
```

Parse duration string like "5s", "100ms", "1m" into Duration

<details>
<summary>Source</summary>

```rust
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix("ms") {
        let num: u64 = stripped
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_millis(num))
    } else if let Some(stripped) = s.strip_suffix('s') {
        let num: u64 = stripped
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(num))
    } else if let Some(stripped) = s.strip_suffix('m') {
        let num: u64 = stripped
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(num * 60))
    } else {
        // Default to seconds if no suffix
        let num: u64 = s.parse().map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(num))
    }
}
```

</details>



### <span id="fn.trigger"></span>`fn trigger`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger.trigger](../../../../cloacina-python/trigger/#fn.trigger)

```rust
fn trigger (workflow : String , name : Option < String > , poll_interval : & str , allow_concurrent : bool ,) -> PyResult < TriggerDecorator >
```

Python @trigger decorator function

This function is exposed to Python as a decorator that registers
Python functions as triggers in the Cloacina trigger scheduler.

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

```rust
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



