---
title: "cloacina-python.trigger_rules"
description: "Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde representation (execution_planner::trigger_rules), so `@cloaca.task( trigger_rules=...)` can gate a task and have it land in the real `Skipped` state — the same evaluator the Rust DSL feeds."
sidebar:
  order: 14
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde representation (execution_planner::trigger_rules), so `@cloaca.task( trigger_rules=...)` can gate a task and have it land in the real `Skipped` state — the same evaluator the Rust DSL feeds.

Conditions: `context_value`, `task_success`, `task_failed`, `task_skipped`.
Rules: `all_of`, `any_of`, `none_of`, `always`. A bare condition passed to
`trigger_rules=` is treated as `all_of(that_condition)`.

## Functions

### <span id="fn.context_value"></span>`context_value`

```python
def context_value(key: str, operator: str, value: PyObject) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::context_value](../../rust/cloacina-python/trigger_rules/#fn.context_value)

A context-value condition: `context_value("key", "Equals", value)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `key` | `str` |  |
| `operator` | `str` |  |
| `value` | `PyObject` |  |


<details>
<summary>Source</summary>

```python
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



### <span id="fn.task_success"></span>`task_success`

```python
def task_success(task_name: str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::task_success](../../rust/cloacina-python/trigger_rules/#fn.task_success)

<details>
<summary>Source</summary>

```python
pub fn task_success(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskSuccess", "task_name": task_name }),
    )
}
```

</details>



### <span id="fn.task_failed"></span>`task_failed`

```python
def task_failed(task_name: str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::task_failed](../../rust/cloacina-python/trigger_rules/#fn.task_failed)

<details>
<summary>Source</summary>

```python
pub fn task_failed(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskFailed", "task_name": task_name }),
    )
}
```

</details>



### <span id="fn.task_skipped"></span>`task_skipped`

```python
def task_skipped(task_name: str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::task_skipped](../../rust/cloacina-python/trigger_rules/#fn.task_skipped)

<details>
<summary>Source</summary>

```python
pub fn task_skipped(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskSkipped", "task_name": task_name }),
    )
}
```

</details>



### <span id="fn.all_of"></span>`all_of`

```python
def all_of(conditions: Vec < PyObject >) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::all_of](../../rust/cloacina-python/trigger_rules/#fn.all_of)

Run only if ALL conditions are met.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conditions` | `Vec < PyObject >` |  |


<details>
<summary>Source</summary>

```python
pub fn all_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "All", conditions)
}
```

</details>



### <span id="fn.any_of"></span>`any_of`

```python
def any_of(conditions: Vec < PyObject >) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::any_of](../../rust/cloacina-python/trigger_rules/#fn.any_of)

Run if ANY condition is met.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conditions` | `Vec < PyObject >` |  |


<details>
<summary>Source</summary>

```python
pub fn any_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "Any", conditions)
}
```

</details>



### <span id="fn.none_of"></span>`none_of`

```python
def none_of(conditions: Vec < PyObject >) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::none_of](../../rust/cloacina-python/trigger_rules/#fn.none_of)

Run only if NONE of the conditions are met.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `conditions` | `Vec < PyObject >` |  |


<details>
<summary>Source</summary>

```python
pub fn none_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "None", conditions)
}
```

</details>



### <span id="fn.always"></span>`always`

```python
def always() -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::trigger_rules::always](../../rust/cloacina-python/trigger_rules/#fn.always)

The default rule — always run.

<details>
<summary>Source</summary>

```python
pub fn always(py: Python) -> PyResult<PyObject> {
    to_py(py, serde_json::json!({ "type": "Always" }))
}
```

</details>



