/*
 *  Copyright 2025 Colliery Software
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

//! Python trigger support for event-driven workflow execution.
//!
//! This module provides Python bindings for defining triggers that poll
//! user-defined conditions and fire workflows when those conditions are met.

use crate::context::PyContext;
use async_trait::async_trait;
use cloacina::trigger::{Trigger, TriggerError, TriggerResult};
use cloacina::Context;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Python TriggerResult class - represents the result of a trigger poll.
///
/// Use `TriggerResult.skip()` when the condition is not met.
/// Use `TriggerResult.fire(context=None)` when the condition is met.
#[pyclass(name = "TriggerResult")]
pub struct PyTriggerResult {
    is_fire: bool,
    data: Option<std::collections::HashMap<String, Value>>,
}

#[pymethods]
impl PyTriggerResult {
    /// Create a Skip result - condition not met, continue polling.
    #[staticmethod]
    fn skip() -> Self {
        PyTriggerResult {
            is_fire: false,
            data: None,
        }
    }

    /// Create a Fire result - condition met, trigger the workflow.
    ///
    /// # Arguments
    /// * `context` - Optional context to pass to the workflow
    #[staticmethod]
    #[pyo3(signature = (context=None))]
    fn fire(context: Option<&PyContext>) -> Self {
        let data = context.map(|c| c.get_data_clone());
        PyTriggerResult {
            is_fire: true,
            data,
        }
    }

    fn __repr__(&self) -> String {
        if !self.is_fire {
            "TriggerResult.Skip".to_string()
        } else if self.data.is_none() {
            "TriggerResult.Fire(None)".to_string()
        } else {
            "TriggerResult.Fire(<context>)".to_string()
        }
    }

    /// Check if this is a Fire result
    fn is_fire_result(&self) -> bool {
        self.is_fire
    }

    /// Check if this is a Skip result
    fn is_skip_result(&self) -> bool {
        !self.is_fire
    }
}

impl PyTriggerResult {
    /// Convert to Rust TriggerResult
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
}

/// Python trigger wrapper implementing Rust Trigger trait.
///
/// This struct allows Python functions to be registered and executed
/// as triggers within the Cloacina trigger scheduler.
pub struct PythonTriggerWrapper {
    name: String,
    workflow_name: String,
    poll_interval: Duration,
    allow_concurrent: bool,
    python_function: PyObject,
}

impl std::fmt::Debug for PythonTriggerWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythonTriggerWrapper")
            .field("name", &self.name)
            .field("workflow_name", &self.workflow_name)
            .field("poll_interval", &self.poll_interval)
            .field("allow_concurrent", &self.allow_concurrent)
            .field("python_function", &"<PyObject>")
            .finish()
    }
}

unsafe impl Send for PythonTriggerWrapper {}
unsafe impl Sync for PythonTriggerWrapper {}

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

    async fn poll(&self) -> Result<TriggerResult, TriggerError> {
        let function = Python::with_gil(|py| self.python_function.clone_ref(py));
        let trigger_name = self.name.clone();

        // Execute Python function in a blocking task
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Call Python poll function
                let result = function.call0(py).map_err(|e| TriggerError::PollError {
                    message: format!("Python trigger poll failed: {}", e),
                })?;

                // Downcast to PyTriggerResult
                let bound_result = result.bind(py);
                let py_result_bound = bound_result.downcast::<PyTriggerResult>().map_err(|e| {
                    TriggerError::PollError {
                        message: format!(
                            "Trigger '{}' must return TriggerResult, got: {}",
                            trigger_name, e
                        ),
                    }
                })?;

                // Borrow and convert to Rust TriggerResult
                let py_result = py_result_bound.borrow();
                let rust_result = if !py_result.is_fire {
                    TriggerResult::Skip
                } else {
                    let ctx = py_result.data.clone().map(|d| {
                        let mut context = Context::new();
                        for (key, value) in d {
                            context.insert(key, value).ok();
                        }
                        context
                    });
                    TriggerResult::Fire(ctx)
                };

                Ok(rust_result)
            })
        })
        .await
        .map_err(|e| TriggerError::PollError {
            message: format!("Trigger execution panicked: {}", e),
        })?
    }
}

impl PythonTriggerWrapper {
    /// Get the workflow name this trigger is associated with
    pub fn workflow_name(&self) -> &str {
        &self.workflow_name
    }
}

/// Parse duration string like "5s", "100ms", "1m" into Duration
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.ends_with("ms") {
        let num: u64 = s[..s.len() - 2]
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_millis(num))
    } else if s.ends_with('s') {
        let num: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(num))
    } else if s.ends_with('m') {
        let num: u64 = s[..s.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(num * 60))
    } else {
        // Default to seconds if no suffix
        let num: u64 = s.parse().map_err(|_| format!("Invalid duration: {}", s))?;
        Ok(Duration::from_secs(num))
    }
}

/// Decorator class that holds trigger configuration
#[pyclass]
pub struct TriggerDecorator {
    name: Option<String>,
    workflow: String,
    poll_interval: Duration,
    allow_concurrent: bool,
}

#[pymethods]
impl TriggerDecorator {
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

        // Register trigger constructor in the global registry
        cloacina::trigger::register_trigger_constructor(trigger_name.clone(), move || {
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
}

/// Python @trigger decorator function
///
/// This function is exposed to Python as a decorator that registers
/// Python functions as triggers in the Cloacina trigger scheduler.
///
/// # Examples
///
/// ```python
/// import cloaca
/// import random
///
/// @cloaca.trigger(
///     workflow="my_workflow",
///     poll_interval="5s",
///     allow_concurrent=False
/// )
/// def my_trigger():
///     # Check some condition
///     if random.randint(1, 100) == 42:
///         ctx = cloaca.Context({"triggered_at": "now"})
///         return cloaca.TriggerResult.fire(ctx)
///     return cloaca.TriggerResult.skip()
/// ```
#[pyfunction]
#[pyo3(signature = (
    workflow,
    *,
    name = None,
    poll_interval = "5s",
    allow_concurrent = false
))]
#[allow(dead_code)] // Used by PyO3 module exports
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("100ms").unwrap(), Duration::from_millis(100));
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("10").unwrap(), Duration::from_secs(10));
    }
}
