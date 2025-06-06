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
use std::collections::HashMap;

/// Python wrapper for building workflows
#[pyclass(name = "WorkflowBuilder")]
pub struct PyWorkflowBuilder {
    name: String,
    description: Option<String>,
    tags: HashMap<String, String>,
    task_ids: Vec<String>,
}

#[pymethods]
impl PyWorkflowBuilder {
    /// Create a new workflow builder
    #[new]
    pub fn new(name: &str) -> Self {
        PyWorkflowBuilder {
            name: name.to_string(),
            description: None,
            tags: HashMap::new(),
            task_ids: Vec::new(),
        }
    }

    /// Set the workflow description
    pub fn description(&mut self, description: &str) {
        self.description = Some(description.to_string());
    }

    /// Add a tag to the workflow metadata
    pub fn tag(&mut self, key: &str, value: &str) {
        self.tags.insert(key.to_string(), value.to_string());
    }

    /// Add a task to the workflow
    pub fn add_task(&mut self, task_or_id: PyObject, py: Python) -> PyResult<()> {
        // Convert task reference to task ID
        let task_id = if let Ok(string_id) = task_or_id.extract::<String>(py) {
            // It's a string - use directly
            string_id
        } else {
            // Try to get function name with better error handling
            match task_or_id.bind(py).hasattr("__name__") {
                Ok(true) => {
                    match task_or_id.getattr(py, "__name__") {
                        Ok(name_obj) => {
                            match name_obj.extract::<String>(py) {
                                Ok(func_name) => func_name,
                                Err(e) => {
                                    return Err(PyValueError::new_err(format!(
                                        "Task has __name__ but it's not a string: {}", e
                                    )));
                                }
                            }
                        },
                        Err(e) => {
                            return Err(PyValueError::new_err(format!(
                                "Failed to get __name__ from task: {}", e
                            )));
                        }
                    }
                },
                Ok(false) => {
                    return Err(PyValueError::new_err(
                        "Task must be either a string task ID or a function object with __name__ attribute"
                    ));
                },
                Err(e) => {
                    return Err(PyValueError::new_err(format!(
                        "Failed to check if task has __name__ attribute: {}", e
                    )));
                }
            }
        };
        
        // Verify the task exists in the registry
        let registry = cloacina::global_task_registry();
        let guard = registry.lock().map_err(|e| {
            PyValueError::new_err(format!("Failed to access task registry: {}", e))
        })?;
        
        if !guard.contains_key(&task_id) {
            return Err(PyValueError::new_err(format!(
                "Task '{}' not found in registry. Make sure the task was decorated with @task.", 
                task_id
            )));
        }
        
        // Store the task ID for later building
        self.task_ids.push(task_id);
        
        Ok(())
    }

    /// Build the final workflow with automatic version calculation
    pub fn build(&self) -> PyResult<PyWorkflow> {
        let mut workflow = cloacina::Workflow::new(&self.name);
        
        // Set description if provided
        if let Some(ref desc) = self.description {
            workflow.set_description(desc);
        }
        
        // Add tags
        for (key, value) in &self.tags {
            workflow.add_tag(key, value);
        }
        
        // Get the task registry
        let registry = cloacina::global_task_registry();
        let guard = registry.lock().map_err(|e| {
            PyValueError::new_err(format!("Failed to access task registry: {}", e))
        })?;
        
        // Add all tasks from the registry using the new add_boxed_task method
        for task_id in &self.task_ids {
            let constructor = guard.get(task_id).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Task '{}' not found in registry during build", 
                    task_id
                ))
            })?;
            
            // Create the task instance
            let task = constructor();
            
            // Add the task using the new add_boxed_task method
            workflow.add_boxed_task(task).map_err(|e| {
                PyValueError::new_err(format!("Failed to add task '{}': {}", task_id, e))
            })?;
        }
        
        let finalized = workflow.finalize();
        
        Ok(PyWorkflow {
            inner: Arc::new(finalized),
        })
    }
}

/// Python wrapper for Rust Workflow
#[pyclass(name = "Workflow")]
#[derive(Clone)]
pub struct PyWorkflow {
    inner: Arc<cloacina::Workflow>,
}

#[pymethods]
impl PyWorkflow {
    /// Get the workflow name
    #[getter]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the workflow version (content-based hash)
    #[getter]
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }

    /// Get the workflow description
    #[getter]
    pub fn description(&self) -> Option<&str> {
        self.inner.metadata().description.as_deref()
    }

    /// Get topological ordering of tasks
    pub fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner.topological_sort().map_err(|e| {
            PyValueError::new_err(format!("Failed to get topological sort: {}", e))
        })
    }

    /// Get execution levels (tasks that can run in parallel)
    pub fn get_execution_levels(&self) -> PyResult<Vec<Vec<String>>> {
        self.inner.get_execution_levels().map_err(|e| {
            PyValueError::new_err(format!("Failed to get execution levels: {}", e))
        })
    }

    /// Get root tasks (tasks with no dependencies)
    pub fn get_roots(&self) -> Vec<String> {
        self.inner.get_roots()
    }

    /// Get leaf tasks (tasks with no dependents)
    pub fn get_leaves(&self) -> Vec<String> {
        self.inner.get_leaves()
    }

    /// Check if two tasks can run in parallel
    pub fn can_run_parallel(&self, task_a: &str, task_b: &str) -> bool {
        self.inner.can_run_parallel(task_a, task_b)
    }

    /// Validate the workflow structure
    pub fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(|e| {
            PyValueError::new_err(format!("Workflow validation failed: {}", e))
        })
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "Workflow(name='{}', version='{}', tasks={})",
            self.inner.name(),
            self.inner.metadata().version,
            self.inner.topological_sort().map(|t| t.len()).unwrap_or(0)
        )
    }
}

/// Register a workflow constructor function in the global registry
///
/// This allows Python-built workflows to be available for execution.
/// The constructor function should return a PyWorkflow when called.
///
/// # Arguments
/// * `workflow_name` - Name of the workflow to register
/// * `constructor` - Python callable that returns a PyWorkflow
#[pyfunction]
pub fn register_workflow_constructor(workflow_name: String, constructor: PyObject) -> PyResult<()> {
    // Create a Rust closure that calls the Python constructor and converts the result
    // Following the same pattern as the Rust workflow! macro
    let rust_constructor = move || -> cloacina::Workflow {
        Python::with_gil(|py| {
            // Call the Python constructor function
            let py_workflow = constructor.call0(py)
                .expect("Failed to call Python workflow constructor");
            
            // Extract the PyWorkflow and get its inner Rust workflow
            let py_workflow: PyRef<PyWorkflow> = py_workflow.extract(py)
                .expect("Constructor must return a Workflow");
            
            // For Python workflows, try to recreate from registry first (for recovery scenarios)
            // If that fails (e.g., tasks defined in test functions), fall back to using the built workflow
            match py_workflow.inner.recreate_from_registry() {
                Ok(workflow) => workflow,
                Err(e) => {
                    // Log warning but continue with the Python-built workflow
                    eprintln!("WARNING: Failed to recreate workflow from registry: {}. Using Python-built workflow.", e);
                    // Since Workflow doesn't implement Clone, we need to recreate it
                    // by calling the Python constructor again
                    let py_workflow = constructor.call0(py)
                        .expect("Failed to call Python workflow constructor");
                    let py_workflow: PyRef<PyWorkflow> = py_workflow.extract(py)
                        .expect("Constructor must return a Workflow");
                    // Extract the inner workflow by taking ownership
                    // This is safe because we're in the constructor closure
                    Arc::try_unwrap(py_workflow.inner.clone())
                        .unwrap_or_else(|arc| {
                            // If we can't unwrap, recreate the workflow structure
                            let workflow = &*arc;
                            let mut new_workflow = cloacina::Workflow::new(workflow.name());
                            // Copy metadata
                            if let Some(desc) = workflow.description() {
                                new_workflow.set_description(desc);
                            }
                            for (k, v) in workflow.tags() {
                                new_workflow.add_tag(k, v);
                            }
                            // Tasks are already in the workflow, no need to recreate them
                            new_workflow
                        })
                }
            }
        })
    };
    
    // Register with the global Rust workflow registry
    cloacina::register_workflow_constructor(workflow_name, rust_constructor);
    
    Ok(())
}