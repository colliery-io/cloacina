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

use async_trait::async_trait;
use chrono::Utc;
use cloacina::{Context, Task, TaskError};
use once_cell::sync::Lazy;
use pyo3::prelude::*;
use pyo3_asyncio_0_21 as pyo3_asyncio;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Global registry of Python tasks
static TASK_REGISTRY: Lazy<Arc<Mutex<HashMap<String, Arc<PythonTask>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// A Python callable wrapped as a Rust Task
#[derive(Clone)]
pub struct PythonTask {
    id: String,
    dependencies: Vec<String>,
    python_function: PyObject,
}

impl PythonTask {
    pub fn new(id: String, dependencies: Vec<String>, python_function: PyObject) -> Self {
        Self {
            id,
            dependencies,
            python_function,
        }
    }
}

#[async_trait]
impl Task for PythonTask {
    async fn execute(
        &self,
        context: Context<serde_json::Value>,
    ) -> Result<Context<serde_json::Value>, TaskError> {
        let python_function = self.python_function.clone();
        let task_id = self.id.clone();

        // Execute Python function synchronously for now
        // Full async support will be added in a future iteration
        let result = Python::with_gil(|py| -> Result<PyObject, TaskError> {
            // Convert Rust context to Python dict
            let py_context = crate::context::context_to_python(&context, py).map_err(|e| {
                TaskError::ExecutionFailed {
                    message: format!("Failed to convert context to Python: {}", e),
                    task_id: task_id.clone(),
                    timestamp: Utc::now(),
                }
            })?;

            // Call the Python function
            let result = python_function.call1(py, (py_context,)).map_err(|e| {
                TaskError::ExecutionFailed {
                    message: format!("Python function execution failed: {}", e),
                    task_id: task_id.clone(),
                    timestamp: Utc::now(),
                }
            })?;

            Ok(result)
        })?;

        // Process the result and update context
        let updated_context =
            Python::with_gil(|py| -> Result<Context<serde_json::Value>, TaskError> {
                let mut new_context = context.clone_data();

                // If the function returned a context/dict, merge it back
                if let Ok(dict) = result.downcast_bound::<pyo3::types::PyDict>(py) {
                    for (key, value) in dict {
                        let key_str =
                            key.extract::<String>()
                                .map_err(|e| TaskError::ExecutionFailed {
                                    message: format!("Invalid key type in returned context: {}", e),
                                    task_id: task_id.clone(),
                                    timestamp: Utc::now(),
                                })?;

                        let json_value = crate::context::python_to_json(&value).map_err(|e| {
                            TaskError::ExecutionFailed {
                                message: format!("Failed to convert Python value to JSON: {}", e),
                                task_id: task_id.clone(),
                                timestamp: Utc::now(),
                            }
                        })?;

                        new_context.insert(key_str, json_value).map_err(|e| {
                            TaskError::ExecutionFailed {
                                message: format!("Failed to insert into context: {}", e),
                                task_id: task_id.clone(),
                                timestamp: Utc::now(),
                            }
                        })?;
                    }
                }

                Ok(new_context)
            })?;

        Ok(updated_context)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    fn trigger_rules(&self) -> serde_json::Value {
        serde_json::json!({"type": "Always"})
    }
}

/// Register a Python task in the global registry
pub fn register_python_task(task: PythonTask) -> Result<(), String> {
    let id = task.id.clone();
    let mut registry = TASK_REGISTRY
        .lock()
        .map_err(|e| format!("Failed to acquire task registry lock: {}", e))?;

    if registry.contains_key(&id) {
        return Err(format!("Task with id '{}' already registered", id));
    }

    registry.insert(id.clone(), Arc::new(task));
    println!("Registered Python task: {}", id);
    Ok(())
}

/// Get a registered Python task by ID
pub fn get_python_task(id: &str) -> Option<Arc<PythonTask>> {
    let registry = TASK_REGISTRY.lock().ok()?;
    registry.get(id).cloned()
}

/// Get all registered Python tasks
pub fn get_all_python_tasks() -> Vec<Arc<PythonTask>> {
    let registry = TASK_REGISTRY.lock().unwrap();
    registry.values().cloned().collect()
}

/// Python decorator class for creating tasks
#[pyclass(name = "TaskDecorator")]
pub struct PyTaskDecorator {
    id: String,
    dependencies: Vec<String>,
}

#[pymethods]
impl PyTaskDecorator {
    fn __call__(&self, func: PyObject) -> PyResult<PyObject> {
        let task = PythonTask::new(self.id.clone(), self.dependencies.clone(), func.clone());

        register_python_task(task)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))?;

        // Return the original function so it can still be called normally
        Ok(func)
    }
}

/// Factory function to create Python tasks from Python
#[pyfunction]
#[pyo3(name = "task")]
pub fn task_decorator(id: String, dependencies: Option<Vec<String>>) -> PyResult<PyTaskDecorator> {
    Ok(PyTaskDecorator {
        id,
        dependencies: dependencies.unwrap_or_default(),
    })
}
