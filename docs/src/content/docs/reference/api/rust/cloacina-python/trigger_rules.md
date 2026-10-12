---
title: "cloacina-python::trigger_rules"
description: "Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde representation (execution_planner::trigger_rules), so `@cloaca.task( trigger_rules=...)` can gate a task and have it land in the real `Skipped` state — the same evaluator the Rust DSL feeds."
sidebar:
  order: 42
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-orange-low); color: var(--sl-color-orange-high);">Rust</span>


Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde representation (execution_planner::trigger_rules), so `@cloaca.task( trigger_rules=...)` can gate a task and have it land in the real `Skipped` state — the same evaluator the Rust DSL feeds.

Conditions: `context_value`, `task_success`, `task_failed`, `task_skipped`.
Rules: `all_of`, `any_of`, `none_of`, `always`. A bare condition passed to
`trigger_rules=` is treated as `all_of(that_condition)`.

## Functions

### <span id="fn.normalize_operator"></span>`fn normalize_operator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn normalize_operator (op : & str) -> PyResult < & 'static str >
```

Normalize a user-supplied operator to the core `ValueOperator` serde name. Accepts the canonical PascalCase plus friendly aliases (case-insensitive).

<details>
<summary>Source</summary>

```rust
fn normalize_operator(op: &str) -> PyResult<&'static str> {
    Ok(match op.to_lowercase().replace('_', "").as_str() {
        "equals" | "==" | "eq" => "Equals",
        "notequals" | "!=" | "ne" => "NotEquals",
        "greaterthan" | ">" | "gt" => "GreaterThan",
        "lessthan" | "<" | "lt" => "LessThan",
        "contains" => "Contains",
        "notcontains" => "NotContains",
        "exists" => "Exists",
        "notexists" => "NotExists",
        other => {
            return Err(PyValueError::new_err(format!(
                "trigger rule: unknown operator '{other}'. Use one of: Equals, NotEquals, \
                 GreaterThan, LessThan, Contains, NotContains, Exists, NotExists.",
            )))
        }
    })
}
```

</details>



### <span id="fn.to_py"></span>`fn to_py`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn to_py (py : Python , value : serde_json :: Value) -> PyResult < PyObject >
```

<details>
<summary>Source</summary>

```rust
fn to_py(py: Python, value: serde_json::Value) -> PyResult<PyObject> {
    Ok(pythonize(py, &value)
        .map_err(|e| PyValueError::new_err(e.to_string()))?
        .into())
}
```

</details>



### <span id="fn.parse_trigger_rules"></span>`fn parse_trigger_rules`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-green-low); color: var(--sl-color-green-high);">pub</span>


```rust
fn parse_trigger_rules (py : Python , obj : & PyObject) -> PyResult < serde_json :: Value >
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



### <span id="fn.context_value"></span>`fn context_value`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.context_value](../../../cloacina-python/trigger_rules/#fn.context_value)

```rust
fn context_value (py : Python , key : String , operator : String , value : PyObject ,) -> PyResult < PyObject >
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



### <span id="fn.task_success"></span>`fn task_success`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.task_success](../../../cloacina-python/trigger_rules/#fn.task_success)

```rust
fn task_success (py : Python , task_name : String) -> PyResult < PyObject >
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



### <span id="fn.task_failed"></span>`fn task_failed`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.task_failed](../../../cloacina-python/trigger_rules/#fn.task_failed)

```rust
fn task_failed (py : Python , task_name : String) -> PyResult < PyObject >
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



### <span id="fn.task_skipped"></span>`fn task_skipped`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.task_skipped](../../../cloacina-python/trigger_rules/#fn.task_skipped)

```rust
fn task_skipped (py : Python , task_name : String) -> PyResult < PyObject >
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



### <span id="fn.combine"></span>`fn combine`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-gray-6); color: var(--sl-color-gray-2);">private</span>


```rust
fn combine (py : Python , kind : & str , conditions : Vec < PyObject >) -> PyResult < PyObject >
```

<details>
<summary>Source</summary>

```rust
fn combine(py: Python, kind: &str, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    let conds: Vec<serde_json::Value> = conditions
        .iter()
        .map(|c| {
            depythonize(c.bind(py))
                .map_err(|e| PyValueError::new_err(format!("trigger rule condition: {e}")))
        })
        .collect::<PyResult<_>>()?;
    to_py(py, serde_json::json!({ "type": kind, "conditions": conds }))
}
```

</details>



### <span id="fn.all_of"></span>`fn all_of`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.all_of](../../../cloacina-python/trigger_rules/#fn.all_of)

```rust
fn all_of (py : Python , conditions : Vec < PyObject >) -> PyResult < PyObject >
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



### <span id="fn.any_of"></span>`fn any_of`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.any_of](../../../cloacina-python/trigger_rules/#fn.any_of)

```rust
fn any_of (py : Python , conditions : Vec < PyObject >) -> PyResult < PyObject >
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



### <span id="fn.none_of"></span>`fn none_of`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.none_of](../../../cloacina-python/trigger_rules/#fn.none_of)

```rust
fn none_of (py : Python , conditions : Vec < PyObject >) -> PyResult < PyObject >
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



### <span id="fn.always"></span>`fn always`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


> **Python API**: [cloacina-python.trigger_rules.always](../../../cloacina-python/trigger_rules/#fn.always)

```rust
fn always (py : Python) -> PyResult < PyObject >
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



