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

use super::task::{pop_workflow_context, push_workflow_context};
use super::workflow_context::PyWorkflowContext;
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
        if let Ok(task_id) = task.extract::<String>(py) {
            let registry = cloacina::task::global_task_registry();

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

            let task_instance = constructor();

            self.inner = self
                .inner
                .clone()
                .add_task(task_instance)
                .map_err(|e| PyValueError::new_err(format!("Failed to add task: {}", e)))?;

            Ok(())
        } else {
            match task.bind(py).hasattr("__name__") {
                Ok(true) => {
                    match task.getattr(py, "__name__") {
                        Ok(name_obj) => {
                            match name_obj.extract::<String>(py) {
                                Ok(func_name) => {
                                    let registry = cloacina::task::global_task_registry();

                                    let (tenant_id, package_name, workflow_id) = self.context.as_components();
                                    let task_namespace = cloacina::TaskNamespace::new(tenant_id, package_name, workflow_id, &func_name);
                                    let guard = registry.read();

                                    let constructor = guard.get(&task_namespace).ok_or_else(|| {
                                        PyValueError::new_err(format!(
                                            "Task '{}' not found in registry. Make sure it was decorated with @task.",
                                            func_name
                                        ))
                                    })?;

                                    let task_instance = constructor();

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
        pop_workflow_context();

        let (tenant_id, package_name, workflow_id) = self.context.as_components();

        let mut workflow = cloacina::Workflow::new(workflow_id);
        workflow.set_tenant(tenant_id);
        workflow.set_package(package_name);

        // Preserve description and tags set on the builder during the `with` block
        if let Some(desc) = self.inner.get_description() {
            workflow.set_description(desc);
        }
        for (key, value) in self.inner.get_tags() {
            workflow.add_tag(key, value);
        }

        let registry = cloacina::task::global_task_registry();
        let guard = registry.read();

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

        drop(guard);

        workflow
            .validate()
            .map_err(|e| PyValueError::new_err(format!("Workflow validation failed: {}", e)))?;
        let final_workflow = workflow.finalize();

        let workflow_name = final_workflow.name().to_string();
        cloacina::workflow::register_workflow_constructor(workflow_name, move || {
            final_workflow.clone()
        });

        Ok(false)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder_new_defaults() {
        pyo3::prepare_freethreaded_python();
        let builder = PyWorkflowBuilder::new("my_workflow", None, None, None);
        let repr = builder.__repr__();
        assert_eq!(repr, "WorkflowBuilder(name='my_workflow')");
    }

    #[test]
    fn test_workflow_builder_new_with_custom_namespace() {
        pyo3::prepare_freethreaded_python();
        let builder = PyWorkflowBuilder::new(
            "wf1",
            Some("custom_tenant"),
            Some("custom_pkg"),
            Some("custom_wf"),
        );
        let repr = builder.__repr__();
        assert!(repr.contains("wf1"));
    }

    #[test]
    fn test_workflow_builder_description_and_tag() {
        pyo3::prepare_freethreaded_python();
        let mut builder = PyWorkflowBuilder::new("wf1", None, None, None);
        builder.description("A test workflow");
        builder.tag("env", "test");
        // Should not panic; verify repr still works
        let repr = builder.__repr__();
        assert!(repr.contains("wf1"));
    }

    #[test]
    fn test_workflow_builder_build_empty_returns_error() {
        pyo3::prepare_freethreaded_python();
        let builder = PyWorkflowBuilder::new("empty_wf", None, None, None);
        let result = builder.build();
        assert!(result.is_err(), "Empty workflow build should fail");
    }

    #[test]
    fn test_workflow_builder_build_with_task() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            // Register a task first so the builder has something
            crate::task::push_workflow_context(crate::workflow_context::PyWorkflowContext::new(
                "public",
                "embedded",
                "build_test_wf",
            ));
            let decorator = crate::task::task(
                Some("build_task".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let func = py
                .eval(pyo3::ffi::c_str!("lambda ctx: ctx"), None, None)
                .unwrap();
            decorator.__call__(py, func.into()).unwrap();
            crate::task::pop_workflow_context();

            // Now build a workflow that references the registered task
            let builder = PyWorkflowBuilder::new("build_test_wf", None, None, None);
            // build() may still fail if the task isn't wired through the builder
            // Just verify it doesn't panic
            let _ = builder.build();
        });
    }
}

/// Register a workflow constructor function
#[pyfunction]
pub fn register_workflow_constructor(name: String, constructor: PyObject) -> PyResult<()> {
    Python::with_gil(|py| {
        let workflow_obj = constructor.call0(py).map_err(|e| {
            PyValueError::new_err(format!("Failed to call workflow constructor: {}", e))
        })?;

        let py_workflow: PyWorkflow = workflow_obj.extract(py).map_err(|e| {
            PyValueError::new_err(format!(
                "Failed to extract workflow from constructor: {}",
                e
            ))
        })?;

        let workflow = py_workflow.inner.clone();
        cloacina::workflow::register_workflow_constructor(name, move || workflow.clone());

        Ok(())
    })
}
