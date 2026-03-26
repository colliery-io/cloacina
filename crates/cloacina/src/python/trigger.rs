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

use crate::packaging::manifest_v2::parse_duration_str;
use crate::trigger::{Trigger, TriggerError, TriggerResult as RustTriggerResult};

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
}

/// Collect all registered Python triggers and clear the registry.
pub fn drain_python_triggers() -> Vec<PythonTriggerDef> {
    let mut registry = PYTHON_TRIGGER_REGISTRY.lock();
    std::mem::take(&mut *registry)
}

/// Python-side trigger result returned from poll functions.
///
/// Usage from Python:
/// ```python
/// from cloaca import TriggerResult
///
/// @cloaca.trigger(name="my_trigger", poll_interval="10s")
/// def my_trigger():
///     if some_condition():
///         return TriggerResult(should_fire=True, context={"key": "value"})
///     return TriggerResult(should_fire=False)
/// ```
#[pyclass(name = "TriggerResult")]
pub struct PyTriggerResult {
    #[pyo3(get, set)]
    pub should_fire: bool,
    #[pyo3(get, set)]
    pub context: Option<PyObject>,
}

#[pymethods]
impl PyTriggerResult {
    #[new]
    #[pyo3(signature = (should_fire = false, context = None))]
    fn new(should_fire: bool, context: Option<PyObject>) -> Self {
        Self {
            should_fire,
            context,
        }
    }

    fn __repr__(&self) -> String {
        if self.should_fire {
            "TriggerResult(should_fire=True)".to_string()
        } else {
            "TriggerResult(should_fire=False)".to_string()
        }
    }
}

/// Decorator for defining Python triggers.
///
/// ```python
/// @cloaca.trigger(name="check_inbox", poll_interval="30s")
/// def check_inbox():
///     # Return TriggerResult(should_fire=True, context={...}) to fire
///     return TriggerResult(should_fire=False)
/// ```
#[pyclass(name = "TriggerDecorator")]
pub struct TriggerDecorator {
    name: Option<String>,
    poll_interval: Duration,
    allow_concurrent: bool,
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
        };

        PYTHON_TRIGGER_REGISTRY.lock().push(def);

        tracing::debug!("Registered Python trigger: {}", trigger_name);

        // Return the original function (decorator is transparent)
        Ok(func)
    }
}

/// `@cloaca.trigger(...)` decorator factory.
#[pyfunction]
#[pyo3(signature = (*, name = None, poll_interval = "30s".to_string(), allow_concurrent = false))]
pub fn trigger(
    name: Option<String>,
    poll_interval: String,
    allow_concurrent: bool,
) -> PyResult<TriggerDecorator> {
    let interval = parse_duration_str(&poll_interval).map_err(|e| {
        PyValueError::new_err(format!("Invalid poll_interval '{}': {}", poll_interval, e))
    })?;

    Ok(TriggerDecorator {
        name,
        poll_interval: interval,
        allow_concurrent,
    })
}

/// Rust wrapper that implements the `Trigger` trait by calling a Python function.
pub struct PythonTriggerWrapper {
    name: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    python_function: PyObject,
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

    async fn poll(&self) -> Result<RustTriggerResult, TriggerError> {
        let function = Python::with_gil(|py| self.python_function.clone_ref(py));
        let trigger_name = self.name.clone();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<RustTriggerResult, TriggerError> {
                let call_result = function.call0(py).map_err(|e| TriggerError::PollError {
                    message: format!("Python trigger '{}' raised exception: {}", trigger_name, e),
                })?;

                // Accept either a TriggerResult object (with should_fire/context attrs) or a plain bool
                let bound = call_result.bind(py);
                if bound.hasattr("should_fire").unwrap_or(false) {
                    let should_fire: bool = bound
                        .getattr("should_fire")
                        .and_then(|v| v.extract())
                        .unwrap_or(false);

                    if should_fire {
                        let ctx = if let Ok(py_ctx) = bound.getattr("context") {
                            if py_ctx.is_none() {
                                None
                            } else {
                                let dict_value: serde_json::Value = pythonize::depythonize(&py_ctx)
                                    .map_err(|e| TriggerError::PollError {
                                        message: format!(
                                            "Failed to serialize trigger context: {}",
                                            e
                                        ),
                                    })?;
                                let mut context = crate::Context::new();
                                if let serde_json::Value::Object(map) = dict_value {
                                    for (k, v) in map {
                                        context.insert(k, v).map_err(|e| {
                                            TriggerError::PollError {
                                                message: format!("Context insert error: {}", e),
                                            }
                                        })?;
                                    }
                                }
                                Some(context)
                            }
                        } else {
                            None
                        };
                        Ok(RustTriggerResult::Fire(ctx))
                    } else {
                        Ok(RustTriggerResult::Skip)
                    }
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

            let decorator =
                trigger(Some("test_poll".to_string()), "5s".to_string(), false).unwrap();

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

            let decorator = trigger(None, "10s".to_string(), false).unwrap();

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

    #[test]
    fn test_py_trigger_result_creation() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|_py| {
            let result = PyTriggerResult::new(true, None);
            assert!(result.should_fire);
            assert!(result.context.is_none());

            let result = PyTriggerResult::new(false, None);
            assert!(!result.should_fire);
        });
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
            }
        });

        let wrapper = PythonTriggerWrapper::new(&def);
        let result = wrapper.poll().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("boom"));
    }
}
