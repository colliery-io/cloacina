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

//! Python workflow package loader.
//!
//! Imports a Python workflow module via PyO3, triggering `@task` decorator
//! registration, then collects the registered tasks and builds the workflow.
//!
//! This is the bridge between extracted `.cloacina` packages and the
//! cloacina task execution engine.

use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::path::Path;

use super::task::{pop_workflow_context, push_workflow_context};
use super::workflow_context::PyWorkflowContext;
use crate::task::TaskNamespace;

/// Error type for Python package loading operations.
#[derive(Debug, thiserror::Error)]
pub enum PythonLoaderError {
    #[error("Python import failed: {0}")]
    ImportError(String),

    #[error("Workflow validation failed: {0}")]
    ValidationError(String),

    #[error("Task registration failed: {0}")]
    RegistrationError(String),

    #[error("Python runtime error: {0}")]
    RuntimeError(String),
}

impl From<PyErr> for PythonLoaderError {
    fn from(err: PyErr) -> Self {
        PythonLoaderError::RuntimeError(err.to_string())
    }
}

/// Ensure the `cloaca` Python module is available in the embedded interpreter.
///
/// User workflow code does `from cloaca import task, WorkflowBuilder`.
/// When running inside the server (no pip-installed cloaca wheel), we inject
/// a synthetic `cloaca` module that exports the PyO3 types from `cloacina::python`.
pub fn ensure_cloaca_module(py: Python) -> PyResult<()> {
    let sys_modules = py.import("sys")?.getattr("modules")?;

    // Already registered — nothing to do
    if sys_modules.contains("cloaca")? {
        return Ok(());
    }

    let module = PyModule::new(py, "cloaca")?;

    // Task decorator and handle
    module.add_function(wrap_pyfunction!(super::task::task, &module)?)?;
    module.add_class::<super::task::PyTaskHandle>()?;
    module.add_class::<super::task::TaskDecorator>()?;

    // Context
    module.add_class::<super::context::PyContext>()?;

    // Workflow
    module.add_class::<super::workflow::PyWorkflowBuilder>()?;
    module.add_class::<super::workflow::PyWorkflow>()?;
    module.add_function(wrap_pyfunction!(
        super::workflow::register_workflow_constructor,
        &module
    )?)?;

    // Value objects
    module.add_class::<super::workflow_context::PyWorkflowContext>()?;
    module.add_class::<super::namespace::PyTaskNamespace>()?;

    // Register in sys.modules so `import cloaca` works
    sys_modules.set_item("cloaca", &module)?;

    Ok(())
}

/// Import a Python workflow module and register its tasks.
///
/// This is the core function that bridges extracted Python packages to
/// the cloacina execution engine:
///
/// 1. Ensures the `cloaca` module is available in the interpreter
/// 2. Adds workflow and vendor directories to `sys.path`
/// 3. Pushes a workflow context (so `@task` decorators know the namespace)
/// 4. Imports the entry module (triggering decorator registration)
/// 5. Collects registered tasks, builds and validates the workflow
/// 6. Returns the list of registered task namespaces
///
/// # Arguments
///
/// * `workflow_dir` — Path to the extracted `workflow/` directory
/// * `vendor_dir` — Path to the extracted `vendor/` directory
/// * `entry_module` — Dotted module path (e.g., `"workflow.tasks"`)
/// * `package_name` — Package name from manifest
/// * `tenant_id` — Tenant for namespace isolation (default: `"public"`)
pub fn import_and_register_python_workflow(
    workflow_dir: &Path,
    vendor_dir: &Path,
    entry_module: &str,
    package_name: &str,
    tenant_id: &str,
) -> Result<Vec<TaskNamespace>, PythonLoaderError> {
    let workflow_dir = workflow_dir.to_path_buf();
    let vendor_dir = vendor_dir.to_path_buf();
    let entry_module = entry_module.to_string();
    let package_name = package_name.to_string();
    let tenant_id = tenant_id.to_string();

    // PyO3 operations must happen on a thread that can acquire the GIL
    let result = std::thread::spawn(move || -> Result<Vec<TaskNamespace>, PythonLoaderError> {
        Python::with_gil(|py| {
            // 1. Ensure cloaca module is available
            ensure_cloaca_module(py)?;

            // 2. Add paths to sys.path
            let sys = py.import("sys")?;
            let path = sys.getattr("path")?;
            path.call_method1(
                "insert",
                (
                    0i32,
                    workflow_dir
                        .to_str()
                        .ok_or(PythonLoaderError::RuntimeError(
                            "Invalid workflow_dir path".to_string(),
                        ))?,
                ),
            )?;
            if vendor_dir.exists() {
                path.call_method1(
                    "insert",
                    (
                        0i32,
                        vendor_dir
                            .to_str()
                            .ok_or(PythonLoaderError::RuntimeError(
                                "Invalid vendor_dir path".to_string(),
                            ))?,
                    ),
                )?;
            }

            // 3. Push workflow context for @task decorators
            let context =
                PyWorkflowContext::new(&tenant_id, &package_name, &entry_module);
            push_workflow_context(context.clone());

            // 4. Import entry module — @task decorators fire, tasks registered
            let import_result = py.import(entry_module.as_str());
            if let Err(e) = import_result {
                pop_workflow_context();
                return Err(PythonLoaderError::ImportError(format!(
                    "Failed to import '{}': {}",
                    entry_module, e
                )));
            }

            // 5. Pop context
            pop_workflow_context();

            // 6. Collect registered tasks and build workflow
            let (t, p, w) = context.as_components();

            let registry = crate::task::global_task_registry();
            let guard = registry.read();

            let mut namespaces = Vec::new();
            let mut workflow = crate::Workflow::new(w);
            workflow.set_tenant(t);
            workflow.set_package(p);

            for (namespace, constructor) in guard.iter() {
                if namespace.tenant_id == t
                    && namespace.package_name == p
                    && namespace.workflow_id == w
                {
                    namespaces.push(namespace.clone());
                    let task_instance = constructor();
                    workflow.add_task(task_instance).map_err(|e| {
                        PythonLoaderError::RegistrationError(format!(
                            "Failed to add task: {}",
                            e
                        ))
                    })?;
                }
            }
            drop(guard);

            if namespaces.is_empty() {
                return Err(PythonLoaderError::RegistrationError(format!(
                    "No tasks registered after importing '{}'. Ensure the module uses @cloaca.task decorators.",
                    entry_module
                )));
            }

            // 7. Validate and register workflow
            workflow.validate().map_err(|e| {
                PythonLoaderError::ValidationError(format!(
                    "Workflow validation failed: {}",
                    e
                ))
            })?;
            let final_workflow = workflow.finalize();

            let workflow_name = final_workflow.name().to_string();
            crate::workflow::register_workflow_constructor(workflow_name, move || {
                final_workflow.clone()
            });

            tracing::info!(
                "Python workflow imported: {} tasks registered for {}::{}::{}",
                namespaces.len(),
                t,
                p,
                w
            );

            Ok(namespaces)
        })
    })
    .join()
    .map_err(|_| PythonLoaderError::RuntimeError("Python import thread panicked".to_string()))??;

    Ok(result)
}
