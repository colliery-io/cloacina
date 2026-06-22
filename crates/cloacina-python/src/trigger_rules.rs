/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Python trigger-rule authoring (CLOACI-T-0763): Python parity for Rust's
//! `#[task(trigger_rules = ...)]`. These builder pyfunctions return plain
//! JSON-shaped dicts matching the core `TriggerRule` / `TriggerCondition` serde
//! representation (execution_planner::trigger_rules), so `@cloaca.task(
//! trigger_rules=...)` can gate a task and have it land in the real `Skipped`
//! state — the same evaluator the Rust DSL feeds.
//!
//! Conditions: `context_value`, `task_success`, `task_failed`, `task_skipped`.
//! Rules: `all_of`, `any_of`, `none_of`, `always`. A bare condition passed to
//! `trigger_rules=` is treated as `all_of(that_condition)`.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pythonize::{depythonize, pythonize};

/// Condition `type` tags (vs. rule tags) — used to wrap a bare condition.
const CONDITION_TYPES: [&str; 4] = ["TaskSuccess", "TaskFailed", "TaskSkipped", "ContextValue"];

/// Normalize a user-supplied operator to the core `ValueOperator` serde name.
/// Accepts the canonical PascalCase plus friendly aliases (case-insensitive).
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

fn to_py(py: Python, value: serde_json::Value) -> PyResult<PyObject> {
    Ok(pythonize(py, &value)
        .map_err(|e| PyValueError::new_err(e.to_string()))?
        .into())
}

/// Parse + validate a `trigger_rules=` argument into the canonical TriggerRule
/// JSON. A bare condition (e.g. a `context_value(...)`) is wrapped as `All`.
/// Errors clearly if the value isn't a valid trigger rule.
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

// ---- condition builders ----

/// A context-value condition: `context_value("key", "Equals", value)`.
#[pyfunction]
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

#[pyfunction]
pub fn task_success(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskSuccess", "task_name": task_name }),
    )
}

#[pyfunction]
pub fn task_failed(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskFailed", "task_name": task_name }),
    )
}

#[pyfunction]
pub fn task_skipped(py: Python, task_name: String) -> PyResult<PyObject> {
    to_py(
        py,
        serde_json::json!({ "type": "TaskSkipped", "task_name": task_name }),
    )
}

// ---- rule combinators ----

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

/// Run only if ALL conditions are met.
#[pyfunction]
#[pyo3(signature = (*conditions))]
pub fn all_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "All", conditions)
}

/// Run if ANY condition is met.
#[pyfunction]
#[pyo3(signature = (*conditions))]
pub fn any_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "Any", conditions)
}

/// Run only if NONE of the conditions are met.
#[pyfunction]
#[pyo3(signature = (*conditions))]
pub fn none_of(py: Python, conditions: Vec<PyObject>) -> PyResult<PyObject> {
    combine(py, "None", conditions)
}

/// The default rule — always run.
#[pyfunction]
pub fn always(py: Python) -> PyResult<PyObject> {
    to_py(py, serde_json::json!({ "type": "Always" }))
}
