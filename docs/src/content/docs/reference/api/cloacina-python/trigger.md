---
title: "cloacina-python.trigger"
description: "Python trigger bindings via PyO3."
sidebar:
  order: 13
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


Python trigger bindings via PyO3.

Provides:
- `@cloaca.trigger` decorator for defining custom Python triggers
- `TriggerResult` Python class for returning poll results
- `PythonTriggerWrapper` implementing the Rust `Trigger` trait

## Classes

### <span id="class.TriggerDecorator"></span>`class TriggerDecorator`

> **Rust Implementation**: [cloacina-python::trigger::TriggerDecorator](../../rust/cloacina-python/trigger/#struct.TriggerDecorator)

Decorator for defining Python triggers.

```python
@cloaca.trigger(name="check_inbox", poll_interval="30s")
def check_inbox():
# Return TriggerResult.fire(context) to fire, .skip() otherwise
return TriggerResult.skip()
```

#### Methods

##### <span id="TriggerDecorator.__call__"></span>`__call__`

```python
def __call__(func: PyObject) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger::TriggerDecorator::__call__](../../rust/cloacina-python/trigger/#TriggerDecorator.__call__)

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





## Functions

### <span id="fn.trigger"></span>`trigger`

```python
def trigger(
    name: Option < String >,
    on: Option < String >,
    poll_interval: Option < String >,
    cron: Option < String >,
    timezone: Option < String >,
    allow_concurrent: bool,
) -> PyResult < TriggerDecorator >
```

> **Rust Implementation**: [cloacina-python::trigger::trigger](../../rust/cloacina-python/trigger/#fn.trigger)

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
| `name` | `Option < String >` |  |
| `on` | `Option < String >` |  |
| `poll_interval` | `Option < String >` |  |
| `cron` | `Option < String >` |  |
| `timezone` | `Option < String >` |  |
| `allow_concurrent` | `bool` |  |


<details>
<summary>Source</summary>

```python
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



