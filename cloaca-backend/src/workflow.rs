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

use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

/// Python wrapper for WorkflowBuilder
#[pyclass(name = "WorkflowBuilder")]
pub struct PyWorkflowBuilder {
    inner: cloacina::WorkflowBuilder,
}

#[pymethods]
impl PyWorkflowBuilder {
    /// Create a new WorkflowBuilder
    #[new]
    pub fn new(name: &str) -> Self {
        PyWorkflowBuilder {
            inner: cloacina::Workflow::builder(name),
        }
    }

    /// Set the workflow description
    pub fn description(&mut self, description: &str) {
        self.inner = self.inner.clone().description(description);
    }

    /// Add a tag to the workflow
    pub fn tag(&mut self, key: &str, value: &str) {
        self.inner = self.inner.clone().tag(key, value);
    }

    /// Add a task to the workflow by ID or function reference
    pub fn add_task(&mut self, py: Python, task: PyObject) -> PyResult<()> {
        // Try to extract as string first
        if let Ok(task_id) = task.extract::<String>(py) {
            // It's a string task ID - look it up in the registry
            let registry = cloacina::task::global_task_registry();
            let guard = registry.read().map_err(|e| {
                PyValueError::new_err(format!("Failed to access task registry: {}", e))
            })?;
            
            let constructor = guard.get(&task_id).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Task '{}' not found in registry. Make sure it was decorated with @task.",
                    task_id
                ))
            })?;
            
            // Create the task instance
            let task_instance = constructor();
            
            // Add to workflow builder
            self.inner = self.inner.clone().add_task(task_instance)
                .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;
                
            Ok(())
        } else {
            // Try to get function name from the object
            match task.bind(py).hasattr("__name__") {
                Ok(true) => {
                    match task.getattr(py, "__name__") {
                        Ok(name_obj) => {
                            match name_obj.extract::<String>(py) {
                                Ok(func_name) => {
                                    // Look up by function name
                                    let registry = cloacina::task::global_task_registry();
                                    let guard = registry.read().map_err(|e| {
                                        PyValueError::new_err(format!("Failed to access task registry: {}", e))
                                    })?;
                                    
                                    let constructor = guard.get(&func_name).ok_or_else(|| {
                                        PyValueError::new_err(format!(
                                            "Task '{}' not found in registry. Make sure it was decorated with @task.",
                                            func_name
                                        ))
                                    })?;
                                    
                                    // Create the task instance
                                    let task_instance = constructor();
                                    
                                    // Add to workflow builder
                                    self.inner = self.inner.clone().add_task(task_instance)
                                        .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;
                                        
                                    Ok(())
                                },
                                Err(e) => {
                                    Err(PyValueError::new_err(format!(
                                        "Function has __name__ but it's not a string: {}",
                                        e
                                    )))
                                }
                            }
                        },
                        Err(e) => {
                            Err(PyValueError::new_err(format!(
                                "Failed to get __name__ from function: {}",
                                e
                            )))
                        }
                    }
                },
                Ok(false) => {
                    Err(PyValueError::new_err(
                        "Task must be either a string task ID or a function object with __name__ attribute"
                    ))
                },
                Err(e) => {
                    Err(PyValueError::new_err(format!(
                        "Failed to check if object has __name__ attribute: {}",
                        e
                    )))
                }
            }
        }
    }

    /// Build the workflow
    pub fn build(&self) -> PyResult<PyWorkflow> {
        let workflow = self.inner.clone().build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build workflow: {}", e)))?;
        Ok(PyWorkflow { inner: workflow })
    }

    /// Context manager entry
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    /// Context manager exit - automatically build and register workflow
    pub fn __exit__(
        &self,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        // Build the workflow
        let workflow = self.inner.clone().build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build workflow: {}", e)))?;
        
        // Register it automatically
        let workflow_name = workflow.name().to_string();
        
        cloacina::workflow::register_workflow_constructor(
            workflow_name,
            move || workflow.clone()
        );
        
        Ok(false) // Don't suppress exceptions
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!("WorkflowBuilder(name='{}')", self.inner.name())
    }
}

/// Python wrapper for Workflow
#[pyclass(name = "Workflow")]
#[derive(Clone)]
pub struct PyWorkflow {
    inner: cloacina::Workflow,
}

#[pymethods]
impl PyWorkflow {
    /// Get workflow name
    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get workflow description
    #[getter]
    pub fn description(&self) -> String {
        self.inner.metadata().description.clone().unwrap_or_default()
    }

    /// Get workflow version
    #[getter]
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }

    /// Get topological sort of tasks
    pub fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner.topological_sort()
            .map_err(|e| PyValueError::new_err(format!("Failed to sort tasks: {}", e)))
    }

    /// Get execution levels (tasks that can run in parallel)
    pub fn get_execution_levels(&self) -> PyResult<Vec<Vec<String>>> {
        self.inner.get_execution_levels()
            .map_err(|e| PyValueError::new_err(format!("Failed to get execution levels: {}", e)))
    }

    /// Get root tasks (no dependencies)
    pub fn get_roots(&self) -> Vec<String> {
        self.inner.get_roots()
    }

    /// Get leaf tasks (no dependents)
    pub fn get_leaves(&self) -> Vec<String> {
        self.inner.get_leaves()
    }

    /// Check if two tasks can run in parallel
    pub fn can_run_parallel(&self, task1: &str, task2: &str) -> bool {
        self.inner.can_run_parallel(task1, task2)
    }

    /// Validate the workflow
    pub fn validate(&self) -> PyResult<()> {
        self.inner.validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!("Workflow(name='{}', tasks={})", self.inner.name(), self.inner.get_task_ids().len())
    }
}

/// Register a workflow constructor function
#[pyfunction]
pub fn register_workflow_constructor(name: String, constructor: PyObject) -> PyResult<()> {
    let constructor_arc = Arc::new(constructor);
    
    cloacina::workflow::register_workflow_constructor(
        name.clone(),
        {
            let _name_clone = name.clone();
            let constructor_clone = constructor_arc.clone();
            move || {
                Python::with_gil(|py| {
                    // Call the Python constructor function
                    match constructor_clone.call0(py) {
                        Ok(workflow_obj) => {
                            // Extract the PyWorkflow wrapper
                            match workflow_obj.extract::<PyWorkflow>(py) {
                                Ok(py_workflow) => py_workflow.inner.clone(),
                                Err(e) => {
                                    eprintln!("Failed to extract workflow from constructor: {}", e);
                                    panic!("Workflow constructor must return a Workflow object");
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to call workflow constructor: {}", e);
                            panic!("Failed to call workflow constructor");
                        }
                    }
                })
            }
        }
    );
    
    Ok(())
}

// Removed workflow decorator - using context manager pattern instead