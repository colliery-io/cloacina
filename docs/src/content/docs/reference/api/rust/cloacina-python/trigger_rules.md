---
title: "cloacina-python::trigger_rules"
description: "Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde representation (execution_planner::trigger_rules), so `@cloaca.task( trigger_rules=...)` can gate a task and have it land in the real `Skipped` state — the same evaluator the Rust DSL feeds."
editUrl: false
sidebar:
  order: 26
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>

Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde representation (execution_planner::trigger_rules), so `@cloaca.task( trigger_rules=...)` can gate a task and have it land in the real `Skipped` state — the same evaluator the Rust DSL feeds.

Conditions: `context_value`, `task_success`, `task_failed`, `task_skipped`.
Rules: `all_of`, `any_of`, `none_of`, `always`. A bare condition passed to
`trigger_rules=` is treated as `all_of(that_condition)`.

## Functions

### <span id="fn.parse_trigger_rules"></span><code style="font-size: 0.85em">fn parse_​trigger_​rules</code>

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>

```rust
fn parse_trigger_rules(
    py: Python,
    obj: &PyObject,
) -> PyResult<serde_json::Value>
```

Parse + validate a `trigger_rules=` argument into the canonical TriggerRule JSON. A bare condition (e.g. a `context_value(...)`) is wrapped as `All`. Errors clearly if the value isn't a valid trigger rule.

<details>
<summary>Source</summary>

```rust
pub fn parse_trigger_rules(py: Python, obj: &PyObject) -> PyResult<serde_json::Value> {
    let mut value: serde_json::Value = depythonize(obj.bind(py)).map_err(|e| {
        PyValueError::new_err(format!(
            "@cloaca.task(trigger_rules=...): not JSON-serializable: {e}"
        ))
    })?;

    // Allow a bare condition for the common single-condition case → wrap as All.
    if let Some(t) = value.get("type").and_then(|v| v.as_str()) {
        if CONDITION_TYPES.contains(&t) {
            value = serde_json::json!({ "type": "All", "conditions": [value] });
        }
    }

    // Validate against the real core type so bad shapes fail at decoration time.
    serde_json::from_value::<cloacina::execution_planner::TriggerRule>(value.clone()).map_err(
        |e| {
            PyValueError::new_err(format!(
                "@cloaca.task(trigger_rules=...): invalid trigger rule: {e}. Build rules with \
                 cloaca.context_value / task_success / task_failed / task_skipped, optionally \
                 inside cloaca.all_of / any_of / none_of.",
            ))
        },
    )?;
    Ok(value)
}
```

</details>

### <span id="fn.context_value"></span><code style="font-size: 0.85em">fn context_​value</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.context_value`](../../../cloaca/#fn.context_value)

```rust
fn context_value(
    py: Python,
    key: String,
    operator: String,
    value: PyObject,
) -> PyResult<PyObject>
```

A context-value condition: `context_value("key", "Equals", value)`.

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

### <span id="fn.task_success"></span><code style="font-size: 0.85em">fn task_​success</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.task_success`](../../../cloaca/#fn.task_success)

```rust
fn task_success(py: Python, task_name: String) -> PyResult<PyObject>
```

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

### <span id="fn.task_failed"></span><code style="font-size: 0.85em">fn task_​failed</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.task_failed`](../../../cloaca/#fn.task_failed)

```rust
fn task_failed(py: Python, task_name: String) -> PyResult<PyObject>
```

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

### <span id="fn.task_skipped"></span><code style="font-size: 0.85em">fn task_​skipped</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.task_skipped`](../../../cloaca/#fn.task_skipped)

```rust
fn task_skipped(py: Python, task_name: String) -> PyResult<PyObject>
```

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

### <span id="fn.all_of"></span><code style="font-size: 0.85em">fn all_​of</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.all_of`](../../../cloaca/#fn.all_of)

```rust
fn all_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject>
```

Run only if ALL conditions are met.

<details>
<summary>Source</summary>

```rust
pub fn all_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "All", conditions)
}
```

</details>

### <span id="fn.any_of"></span><code style="font-size: 0.85em">fn any_​of</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.any_of`](../../../cloaca/#fn.any_of)

```rust
fn any_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject>
```

Run if ANY condition is met.

<details>
<summary>Source</summary>

```rust
pub fn any_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "Any", conditions)
}
```

</details>

### <span id="fn.none_of"></span><code style="font-size: 0.85em">fn none_​of</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.none_of`](../../../cloaca/#fn.none_of)

```rust
fn none_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject>
```

Run only if NONE of the conditions are met.

<details>
<summary>Source</summary>

```rust
pub fn none_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "None", conditions)
}
```

</details>

### <span id="fn.always"></span><code style="font-size: 0.85em">fn always</code>

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>

> **Python API**: [`cloaca.always`](../../../cloaca/#fn.always)

```rust
fn always(py: Python) -> PyResult<PyObject>
```

The default rule — always run.

<details>
<summary>Source</summary>

```rust
pub fn always(py: Python) -> PyResult<PyObject> {
    to_py(py, serde_json::json!({ "type": "Always" }))
}
```

</details>
