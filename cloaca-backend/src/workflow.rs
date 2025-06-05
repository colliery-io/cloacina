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
    pub fn add_task(&mut self, task_id: &str) -> PyResult<()> {
        // Verify the task exists in the registry
        let registry = cloacina::global_task_registry();
        let guard = registry.lock().map_err(|e| {
            PyValueError::new_err(format!("Failed to access task registry: {}", e))
        })?;
        
        if !guard.contains_key(task_id) {
            return Err(PyValueError::new_err(format!(
                "Task '{}' not found in registry. Make sure the task was decorated with @task.", 
                task_id
            )));
        }
        
        // Store the task ID for later building
        self.task_ids.push(task_id.to_string());
        
        Ok(())
    }

    /// Build the final workflow with automatic version calculation
    pub fn build(&self) -> PyResult<PyWorkflow> {
        // For now, create a simple empty workflow for testing
        // TODO: Implement full workflow building with task registry integration
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