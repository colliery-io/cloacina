---
title: "cloacina-python::trigger"
description: "Python trigger bindings via PyO3."
sidebar:
  order: 41
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python trigger bindings via PyO3.

Provides:
- `@cloaca.trigger` decorator for defining custom Python triggers
- `TriggerResult` Python class for returning poll results
- `PythonTriggerWrapper` implementing the Rust `Trigger` trait

## Structs

### <span id="struct.PythonTriggerDef"></span>`struct PythonTriggerDef`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


A collected Python trigger definition.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |
| `python_function` | `PyObject` |  |
| `workflow_name` | `String` | Workflow this trigger fires, from `@cloaca.trigger(on=…)`. Mirrors the
Rust `#[trigger(on = "…")]` binding (CLOACI-T-0688). Empty for legacy
poll triggers that bind via `#[workflow(triggers=[…])]` subscription. |
| `cron_expression` | `Option < String >` | Cron expression from `@cloaca.trigger(cron=…)`. When set, the reconciler
routes this to the cron scheduler instead of the poll registry. |
| `timezone` | `Option < String >` | Timezone for the cron schedule (`@cloaca.trigger(timezone=…)`). |



### <span id="struct.TriggerDecorator"></span>`class TriggerDecorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger.TriggerDecorator](../../../cloacina-python/trigger/#class.TriggerDecorator)

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
| `workflow_name` | `String` |  |
| `cron_expression` | `Option < String >` |  |
| `timezone` | `Option < String >` |  |

#### Methods

##### <span id="TriggerDecorator.__call__"></span>`__call__`

```rust
fn __call__ (& self , py : Python , func : PyObject) -> PyResult < PyObject >
```

> **Python API**: [cloacina-python.trigger.TriggerDecorator.__call__](../../../cloacina-python/trigger/#TriggerDecorator.__call__)

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





### <span id="struct.PythonTriggerWrapper"></span>`struct PythonTriggerWrapper`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


Rust wrapper that implements the `Trigger` trait by calling a Python function.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `poll_interval` | `Duration` |  |
| `allow_concurrent` | `bool` |  |
| `python_function` | `PyObject` |  |
| `workflow_name` | `String` |  |
| `cron_expression` | `Option < String >` |  |

#### Methods

##### <span id="PythonTriggerWrapper.new"></span>`new`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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
            workflow_name: def.workflow_name.clone(),
            cron_expression: def.cron_expression.clone(),
        }
    }
```

</details>





## Functions

### <span id="fn.drain_python_triggers"></span>`fn drain_python_triggers`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


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



### <span id="fn.trigger"></span>`fn trigger`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger.trigger](../../../cloacina-python/trigger/#fn.trigger)

```rust
fn trigger (name : Option < String > , on : Option < String > , poll_interval : Option < String > , cron : Option < String > , timezone : Option < String > , allow_concurrent : bool ,) -> PyResult < TriggerDecorator >
```

`@cloaca.trigger(...)` decorator factory.

Mirrors the Rust `#[trigger]` macro (CLOACI-T-0688). Two modes:
- **Custom poll** — `poll_interval` + a function body that returns a
`TriggerResult`.
- **Cron** — `cron` (+ optional `timezone`); the framework schedules the
`on` workflow on that cron expression (the function body is unused). The
reconciler routes `cron_expression`-bearing triggers to the cron scheduler.
`on` names the workflow the trigger fires; it is **required for cron**
triggers. `cron` and `poll_interval` are mutually exclusive.

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



