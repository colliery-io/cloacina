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

//! Python trigger bindings via PyO3.
//!
//! Provides:
//! - `@cloaca.trigger` decorator for defining custom Python triggers
//! - `TriggerResult` Python class for returning poll results
//! - `PythonTriggerWrapper` implementing the Rust `Trigger` trait

use async_trait::async_trait;
use parking_lot::Mutex;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::fmt;
use std::time::Duration;

use cloacina::packaging::manifest_schema::parse_duration_str;
// T-0552: Trigger trait + TriggerError relocated to cloacina-workflow.
use cloacina::Trigger;
use cloacina_workflow::{TriggerError, TriggerResult as RustTriggerResult};

/// Global registry of Python trigger definitions collected during module import.
/// These are picked up by the reconciler when loading Python packages with
/// `type: "python"` triggers in their manifest.
static PYTHON_TRIGGER_REGISTRY: Mutex<Vec<PythonTriggerDef>> = Mutex::new(Vec::new());

/// A collected Python trigger definition.
pub struct PythonTriggerDef {
    pub name: String,
    pub poll_interval: Duration,
    pub allow_concurrent: bool,
    pub python_function: PyObject,
    /// Workflow this trigger fires, from `@cloaca.trigger(on=…)`. Mirrors the
    /// Rust `#[trigger(on = "…")]` binding (CLOACI-T-0688). Empty for legacy
    /// poll triggers that bind via `#[workflow(triggers=[…])]` subscription.
    pub workflow_name: String,
    /// Cron expression from `@cloaca.trigger(cron=…)`. When set, the reconciler
    /// routes this to the cron scheduler instead of the poll registry.
    pub cron_expression: Option<String>,
    /// Timezone for the cron schedule (`@cloaca.trigger(timezone=…)`).
    pub timezone: Option<String>,
}

/// Collect all registered Python triggers and clear the registry.
pub fn drain_python_triggers() -> Vec<PythonTriggerDef> {
    let mut registry = PYTHON_TRIGGER_REGISTRY.lock();
    std::mem::take(&mut *registry)
}

/// Decorator for defining Python triggers.
///
/// ```python
/// @cloaca.trigger(name="check_inbox", poll_interval="30s")
/// def check_inbox():
///     # Return TriggerResult.fire(context) to fire, .skip() otherwise
///     return TriggerResult.skip()
/// ```
#[pyclass(name = "TriggerDecorator")]
pub struct TriggerDecorator {
    name: Option<String>,
    poll_interval: Duration,
    allow_concurrent: bool,
    workflow_name: String,
    cron_expression: Option<String>,
    timezone: Option<String>,
}

#[pymethods]
impl TriggerDecorator {
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
}

/// `@cloaca.trigger(...)` decorator factory.
///
/// Mirrors the Rust `#[trigger]` macro (CLOACI-T-0688). Two modes:
/// - **Custom poll** — `poll_interval` + a function body that returns a
///   `TriggerResult`.
/// - **Cron** — `cron` (+ optional `timezone`); the framework schedules the
///   `on` workflow on that cron expression (the function body is unused). The
///   reconciler routes `cron_expression`-bearing triggers to the cron scheduler.
///
/// `on` names the workflow the trigger fires; it is **required for cron**
/// triggers. `cron` and `poll_interval` are mutually exclusive.
#[pyfunction]
#[pyo3(signature = (*, name = None, on = None, poll_interval = None, cron = None, timezone = None, allow_concurrent = false))]
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

/// Rust wrapper that implements the `Trigger` trait by calling a Python function.
pub struct PythonTriggerWrapper {
    name: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    python_function: PyObject,
    workflow_name: String,
    cron_expression: Option<String>,
}

// SAFETY: PythonTriggerWrapper holds a PyObject which is not Send/Sync.
// This is safe because ALL access to the PyObject goes through
// tokio::task::spawn_blocking + Python::with_gil(), ensuring the GIL is held.
unsafe impl Send for PythonTriggerWrapper {}
unsafe impl Sync for PythonTriggerWrapper {}

impl PythonTriggerWrapper {
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
}

impl fmt::Debug for PythonTriggerWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PythonTriggerWrapper")
            .field("name", &self.name)
            .field("poll_interval", &self.poll_interval)
            .finish()
    }
}

#[async_trait]
impl Trigger for PythonTriggerWrapper {
    fn name(&self) -> &str {
        &self.name
    }

    fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    fn allow_concurrent(&self) -> bool {
        self.allow_concurrent
    }

    fn workflow_name(&self) -> &str {
        &self.workflow_name
    }

    fn cron_expression(&self) -> Option<String> {
        self.cron_expression.clone()
    }

    async fn poll(&self) -> Result<RustTriggerResult, TriggerError> {
        let function = Python::with_gil(|py| self.python_function.clone_ref(py));
        let trigger_name = self.name.clone();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<RustTriggerResult, TriggerError> {
                let call_result = function.call0(py).map_err(|e| TriggerError::PollError {
                    message: format!("Python trigger '{}' raised exception: {}", trigger_name, e),
                })?;

                // Accept either a TriggerResult object (skip/fire API) or a plain bool.
                // T-0557 Bug 5: the older `should_fire`/`context` attribute shape
                // was removed in favor of `TriggerResult.skip()` / `.fire(ctx)`.
                let bound = call_result.bind(py);
                if let Ok(py_result_bound) =
                    bound.downcast::<crate::bindings::trigger::PyTriggerResult>()
                {
                    let py_result = py_result_bound.borrow();
                    Ok(py_result.clone_into_rust())
                } else if let Ok(should_fire) = call_result.extract::<bool>(py) {
                    // Shorthand: return True/False
                    if should_fire {
                        Ok(RustTriggerResult::Fire(None))
                    } else {
                        Ok(RustTriggerResult::Skip)
                    }
                } else {
                    Err(TriggerError::PollError {
                        message: format!(
                            "Python trigger must return TriggerResult or bool, got {:?}",
                            call_result.bind(py).get_type().name()
                        ),
                    })
                }
            })
        })
        .await
        .map_err(|e| TriggerError::PollError {
            message: format!("Python trigger task panicked: {}", e),
        })??;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::ffi::c_str;

    #[test]
    fn test_trigger_decorator_registers() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            // Clear any previous state
            drain_python_triggers();

            let decorator = trigger(
                Some("test_poll".to_string()),
                None,
                Some("5s".to_string()),
                None,
                None,
                false,
            )
            .unwrap();

            let func = py.eval(c_str!("lambda: None"), None, None).unwrap();
            decorator.__call__(py, func.into()).unwrap();

            let triggers = drain_python_triggers();
            assert_eq!(triggers.len(), 1);
            assert_eq!(triggers[0].name, "test_poll");
            assert_eq!(triggers[0].poll_interval, Duration::from_secs(5));
            assert!(!triggers[0].allow_concurrent);
        });
    }

    #[test]
    fn test_trigger_decorator_uses_function_name() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            drain_python_triggers();

            let decorator =
                trigger(None, None, Some("10s".to_string()), None, None, false).unwrap();

            // Create a named function
            py.run(c_str!("def check_status(): pass"), None, None)
                .unwrap();
            let func = py.eval(c_str!("check_status"), None, None).unwrap();
            decorator.__call__(py, func.into()).unwrap();

            let triggers = drain_python_triggers();
            assert_eq!(triggers.len(), 1);
            assert_eq!(triggers[0].name, "check_status");
        });
    }

    // CLOACI-T-0688: @cloaca.trigger(on=…, cron=…) mirrors the Rust
    // #[trigger(on=…, cron=…)] cron form. The wrapper must expose
    // workflow_name + cron_expression so build_view_python emits them and the
    // reconciler routes the trigger to the cron scheduler.
    #[test]
    fn test_cron_trigger_decorator_carries_cron_and_workflow() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            drain_python_triggers();

            let decorator = trigger(
                Some("nightly".to_string()),
                Some("reports_workflow".to_string()),
                None,
                Some("0 9 * * *".to_string()),
                Some("UTC".to_string()),
                false,
            )
            .unwrap();

            let func = py.eval(c_str!("lambda: None"), None, None).unwrap();
            decorator.__call__(py, func.into()).unwrap();

            let triggers = drain_python_triggers();
            assert_eq!(triggers.len(), 1);
            let def = &triggers[0];
            assert_eq!(def.name, "nightly");
            assert_eq!(def.workflow_name, "reports_workflow");
            assert_eq!(def.cron_expression.as_deref(), Some("0 9 * * *"));
            assert_eq!(def.timezone.as_deref(), Some("UTC"));

            // The Trigger-trait surface the host reads in build_view_python.
            let wrapper = PythonTriggerWrapper::new(def);
            assert_eq!(wrapper.workflow_name(), "reports_workflow");
            assert_eq!(wrapper.cron_expression().as_deref(), Some("0 9 * * *"));
        });
    }

    #[test]
    fn test_cron_trigger_validation_mirrors_rust() {
        // cron requires `on`
        assert!(trigger(None, None, None, Some("0 9 * * *".to_string()), None, false).is_err());
        // cron and poll_interval are mutually exclusive
        assert!(trigger(
            None,
            Some("wf".to_string()),
            Some("30s".to_string()),
            Some("0 9 * * *".to_string()),
            None,
            false
        )
        .is_err());
    }

    #[tokio::test]
    async fn test_python_trigger_wrapper_skip() {
        pyo3::prepare_freethreaded_python();

        let def = Python::with_gil(|py| {
            let func = py.eval(c_str!("lambda: False"), None, None).unwrap();
            PythonTriggerDef {
                name: "skip_trigger".to_string(),
                poll_interval: Duration::from_secs(1),
                allow_concurrent: false,
                python_function: func.into(),
                workflow_name: String::new(),
                cron_expression: None,
                timezone: None,
            }
        });

        let wrapper = PythonTriggerWrapper::new(&def);
        assert_eq!(wrapper.name(), "skip_trigger");

        let result = wrapper.poll().await.unwrap();
        assert!(!result.should_fire());
    }

    #[tokio::test]
    async fn test_python_trigger_wrapper_fire() {
        pyo3::prepare_freethreaded_python();

        let def = Python::with_gil(|py| {
            let func = py.eval(c_str!("lambda: True"), None, None).unwrap();
            PythonTriggerDef {
                name: "fire_trigger".to_string(),
                poll_interval: Duration::from_secs(1),
                allow_concurrent: false,
                python_function: func.into(),
                workflow_name: String::new(),
                cron_expression: None,
                timezone: None,
            }
        });

        let wrapper = PythonTriggerWrapper::new(&def);
        let result = wrapper.poll().await.unwrap();
        assert!(result.should_fire());
    }

    #[tokio::test]
    async fn test_python_trigger_wrapper_exception_handled() {
        pyo3::prepare_freethreaded_python();

        let def = Python::with_gil(|py| {
            py.run(
                c_str!("def bad_trigger():\n    raise ValueError('boom')"),
                None,
                None,
            )
            .unwrap();
            let func = py.eval(c_str!("bad_trigger"), None, None).unwrap();
            PythonTriggerDef {
                name: "bad".to_string(),
                poll_interval: Duration::from_secs(1),
                allow_concurrent: false,
                python_function: func.into(),
                workflow_name: String::new(),
                cron_expression: None,
                timezone: None,
            }
        });

        let wrapper = PythonTriggerWrapper::new(&def);
        let result = wrapper.poll().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("boom"));
    }
}
