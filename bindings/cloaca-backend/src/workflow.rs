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

use crate::task::{pop_workflow_context, push_workflow_context};
use crate::value_objects::PyWorkflowContext;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Python wrapper for WorkflowBuilder
#[pyclass(name = "WorkflowBuilder")]
pub struct PyWorkflowBuilder {
    inner: cloacina::WorkflowBuilder,
    context: PyWorkflowContext,
}

#[pymethods]
impl PyWorkflowBuilder {
    /// Create a new WorkflowBuilder with namespace context
    #[new]
    #[pyo3(signature = (name, *, tenant = None, package = None, workflow = None))]
    pub fn new(
        name: &str,
        tenant: Option<&str>,
        package: Option<&str>,
        workflow: Option<&str>,
    ) -> Self {
        let context = PyWorkflowContext::new(
            tenant.unwrap_or("public"),
            package.unwrap_or("embedded"),
            workflow.unwrap_or(name),
        );

        // Create workflow builder with correct tenant
        let (tenant_id, _package_name, _workflow_id) = context.as_components();
        let workflow_builder = cloacina::Workflow::builder(name).tenant(tenant_id);

        PyWorkflowBuilder {
            inner: workflow_builder,
            context,
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

            // Look up the task from the workflow context namespace
            let (tenant_id, package_name, workflow_id) = self.context.as_components();
            let task_namespace =
                cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &task_id);
            let guard = registry.read();

            let constructor = guard.get(&task_namespace).ok_or_else(|| {
                PyValueError::new_err(format!(
                    "Task '{}' not found in registry. Make sure it was decorated with @task.",
                    task_id
                ))
            })?;

            // Create the task instance
            let task_instance = constructor();

            // Add to workflow builder
            self.inner = self
                .inner
                .clone()
                .add_task(task_instance)
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

                                    // Look up the task from the workflow context namespace
                                    let (tenant_id, package_name, workflow_id) = self.context.as_components();
                                    let task_namespace = cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &func_name);
                                    let guard = registry.read();

                                    let constructor = guard.get(&task_namespace).ok_or_else(|| {
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
        let workflow = self
            .inner
            .clone()
            .build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build workflow: {}", e)))?;
        Ok(PyWorkflow { inner: workflow })
    }

    /// Context manager entry - establish workflow context for task decorators
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_workflow_context(slf.context.clone());
        slf
    }

    /// Context manager exit - clean up context and build workflow
    pub fn __exit__(
        &mut self,
        _py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        // Pop the workflow context
        pop_workflow_context();

        let (tenant_id, package_name, workflow_id) = self.context.as_components();

        // CRITICAL: Create a new workflow with the correct namespace context
        // following the same pattern as the Rust workflow! macro
        let mut workflow = cloacina::Workflow::new(workflow_id);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Collect all tasks registered with this workflow's namespace
        let registry = cloacina::task::global_task_registry();
        let guard = registry.read();

        // Find all tasks that belong to this workflow's namespace
        for (namespace, constructor) in guard.iter() {
            if namespace.tenant_id == tenant_id
                && namespace.package_name == package_name
                && namespace.workflow_id == workflow_id
            {
                let task_instance = constructor();
                workflow
                    .add_task(task_instance)
                    .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;
            }
        }

        // Drop the read guard before building
        drop(guard);

        // Validate and finalize the workflow
        workflow
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))?;
        let final_workflow = workflow.finalize();

        // Register it automatically
        let workflow_name = final_workflow.name().to_string();
        cloacina::workflow::register_workflow_constructor(workflow_name, move || {
            final_workflow.clone()
        });

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
        self.inner
            .metadata()
            .description
            .clone()
            .unwrap_or_default()
    }

    /// Get workflow version
    #[getter]
    pub fn version(&self) -> &str {
        &self.inner.metadata().version
    }

    /// Get topological sort of tasks
    pub fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner
            .topological_sort()
            .map(|namespaces| namespaces.into_iter().map(|ns| ns.to_string()).collect())
            .map_err(|e| PyValueError::new_err(format!("Failed to sort tasks: {}", e)))
    }

    /// Get execution levels (tasks that can run in parallel)
    pub fn get_execution_levels(&self) -> PyResult<Vec<Vec<String>>> {
        self.inner
            .get_execution_levels()
            .map(|levels| {
                levels
                    .into_iter()
                    .map(|level| level.into_iter().map(|ns| ns.to_string()).collect())
                    .collect()
            })
            .map_err(|e| PyValueError::new_err(format!("Failed to get execution levels: {}", e)))
    }

    /// Get root tasks (no dependencies)
    pub fn get_roots(&self) -> Vec<String> {
        self.inner
            .get_roots()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }

    /// Get leaf tasks (no dependents)
    pub fn get_leaves(&self) -> Vec<String> {
        self.inner
            .get_leaves()
            .into_iter()
            .map(|ns| ns.to_string())
            .collect()
    }

    // Note: Removed can_run_parallel - the runner handles parallelism automatically

    /// Validate the workflow
    pub fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))
    }

    /// String representation
    pub fn __repr__(&self) -> String {
        format!(
            "Workflow(name='{}', tasks={})",
            self.inner.name(),
            self.inner.get_task_ids().len()
        )
    }
}

/// Register a workflow constructor function
#[pyfunction]
#[allow(dead_code)] // Used by PyO3 module exports
pub fn register_workflow_constructor(name: String, constructor: PyObject) -> PyResult<()> {
    // Pre-evaluate the constructor immediately while we have the GIL
    Python::with_gil(|py| {
        // Call the Python constructor function immediately
        let workflow_obj = constructor.call0(py).map_err(|e| {
            PyValueError::new_err(format!("Failed to call workflow constructor: {}", e))
        })?;

        // Extract the PyWorkflow wrapper
        let py_workflow: PyWorkflow = workflow_obj.extract(py).map_err(|e| {
            PyValueError::new_err(format!(
                "Failed to extract workflow from constructor: {}",
                e
            ))
        })?;

        // Store the pre-built workflow
        let workflow = py_workflow.inner.clone();
        cloacina::workflow::register_workflow_constructor(name, move || workflow.clone());

        Ok(())
    })
}

// Removed workflow decorator - using context manager pattern instead
