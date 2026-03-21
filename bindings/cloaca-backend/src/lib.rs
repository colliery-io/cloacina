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

use pyo3::prelude::*;

// Local modules that remain in cloaca-backend
mod admin;
mod context;
mod runner;
mod trigger;
mod value_objects;

// Re-imports from cloacina::python (implementations now live in core)
use cloacina::python::context::PyContext;
use cloacina::python::namespace::PyTaskNamespace;
use cloacina::python::task::{task as task_decorator, PyTaskHandle};
use cloacina::python::workflow::{register_workflow_constructor, PyWorkflow, PyWorkflowBuilder};
use cloacina::python::workflow_context::PyWorkflowContext;

// Local imports (still defined in cloaca-backend)
use admin::{PyDatabaseAdmin, PyTenantConfig, PyTenantCredentials};
use context::PyDefaultRunnerConfig;
use runner::{PyDefaultRunner, PyPipelineResult};
use trigger::PyTriggerResult;
use value_objects::{PyBackoffStrategy, PyRetryCondition, PyRetryPolicy, PyRetryPolicyBuilder};

/// A simple hello world class for testing
#[pyclass]
pub struct HelloClass {
    message: String,
}

#[pymethods]
impl HelloClass {
    #[new]
    pub fn new() -> Self {
        HelloClass {
            message: "Hello from HelloClass!".to_string(),
        }
    }

    pub fn get_message(&self) -> String {
        self.message.clone()
    }

    pub fn __repr__(&self) -> String {
        format!("HelloClass(message='{}')", self.message)
    }
}

/// A simple hello world function for testing
#[pyfunction]
#[allow(dead_code)]
fn hello_world() -> String {
    "Hello from Cloaca backend!".to_string()
}

/// A unified Python module supporting both PostgreSQL and SQLite backends.
#[pymodule]
fn cloaca(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Simple test functions
    m.add_function(wrap_pyfunction!(hello_world, m)?)?;

    // Test class
    m.add_class::<HelloClass>()?;

    // Context class (from cloacina::python)
    m.add_class::<PyContext>()?;

    // Configuration class (local)
    m.add_class::<PyDefaultRunnerConfig>()?;

    // Task decorator function and handle (from cloacina::python)
    m.add_function(wrap_pyfunction!(task_decorator, m)?)?;
    m.add_class::<PyTaskHandle>()?;

    // Trigger decorator and result class (local)
    m.add_function(wrap_pyfunction!(trigger::trigger, m)?)?;
    m.add_class::<PyTriggerResult>()?;

    // Workflow classes and functions (from cloacina::python)
    m.add_class::<PyWorkflowBuilder>()?;
    m.add_class::<PyWorkflow>()?;
    m.add_function(wrap_pyfunction!(register_workflow_constructor, m)?)?;

    // Runner classes (local)
    m.add_class::<PyDefaultRunner>()?;
    m.add_class::<PyPipelineResult>()?;

    // Value objects (mixed: namespace/context from cloacina::python, retry local)
    m.add_class::<PyTaskNamespace>()?;
    m.add_class::<PyWorkflowContext>()?;
    m.add_class::<PyRetryPolicy>()?;
    m.add_class::<PyRetryPolicyBuilder>()?;
    m.add_class::<PyBackoffStrategy>()?;
    m.add_class::<PyRetryCondition>()?;

    // Admin classes (local)
    m.add_class::<PyDatabaseAdmin>()?;
    m.add_class::<PyTenantConfig>()?;
    m.add_class::<PyTenantCredentials>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cloacina::python::task::{pop_workflow_context, push_workflow_context, task};
    use cloacina::python::workflow_context::PyWorkflowContext;
    use pyo3::ffi::c_str;

    #[test]
    fn test_task_registration() {
        Python::with_gil(|py| {
            // Push a default workflow context for task registration
            push_workflow_context(PyWorkflowContext::default());

            let task_decorator = task(
                Some("test_task".to_string()),
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

            let mock_func = py.eval(c_str!("lambda ctx: ctx"), None, None).unwrap();
            let result = task_decorator.__call__(py, mock_func.into());
            assert!(result.is_ok(), "Task registration should succeed");

            let registry = cloacina::task::global_task_registry();
            let guard = registry.read();
            let namespace =
                cloacina::TaskNamespace::new("public", "embedded", "default", "test_task");
            assert!(
                guard.get(&namespace).is_some(),
                "Task should be found in registry"
            );

            pop_workflow_context();
        });
    }

    #[test]
    fn test_workflow_add_task_lookup() {
        Python::with_gil(|py| {
            // Push context before registering tasks
            push_workflow_context(PyWorkflowContext::new(
                "public",
                "embedded",
                "lookup_test_workflow",
            ));

            let task_decorator = task(
                Some("lookup_test_task".to_string()),
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

            let mock_func = py.eval(c_str!("lambda ctx: ctx"), None, None).unwrap();
            task_decorator.__call__(py, mock_func.into()).unwrap();

            pop_workflow_context();

            let mut builder = PyWorkflowBuilder::new("lookup_test_workflow", None, None, None);
            let task_id = py.eval(c_str!("'lookup_test_task'"), None, None).unwrap();
            let result = builder.add_task(py, task_id.into());
            assert!(result.is_ok(), "Task should be added to workflow");
        });
    }

    #[test]
    fn test_namespace_investigation() {
        println!("=== Investigating namespace issue ===");

        let registry = cloacina::task::global_task_registry();

        let default_ns =
            cloacina::TaskNamespace::new("public", "embedded", "default", "investigation_task");
        cloacina::register_task_constructor(default_ns.clone(), {
            move || {
                use std::sync::Arc;
                struct TestTask;
                #[async_trait::async_trait]
                impl cloacina::Task for TestTask {
                    async fn execute(
                        &self,
                        context: cloacina::Context<serde_json::Value>,
                    ) -> Result<cloacina::Context<serde_json::Value>, cloacina::TaskError>
                    {
                        Ok(context)
                    }
                    fn id(&self) -> &str {
                        "investigation_task"
                    }
                    fn dependencies(&self) -> &[cloacina::TaskNamespace] {
                        &[]
                    }
                    fn retry_policy(&self) -> cloacina::retry::RetryPolicy {
                        cloacina::retry::RetryPolicy::default()
                    }
                }
                Arc::new(TestTask) as Arc<dyn cloacina::Task>
            }
        });

        {
            let guard = registry.read();
            println!("Registry check:");

            let default_ns =
                cloacina::TaskNamespace::new("public", "embedded", "default", "investigation_task");
            let workflow_ns = cloacina::TaskNamespace::new(
                "public",
                "embedded",
                "test_workflow",
                "investigation_task",
            );

            println!("Default namespace: {:?}", default_ns);
            println!("Workflow namespace: {:?}", workflow_ns);

            println!(
                "Default namespace exists: {}",
                guard.get(&default_ns).is_some()
            );
            println!(
                "Workflow namespace exists: {}",
                guard.get(&workflow_ns).is_some()
            );
        }
    }
}
