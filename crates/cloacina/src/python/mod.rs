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

//! Python integration for Cloacina.
//!
//! This module provides:
//! - Abstract [`PythonTaskExecutor`] trait for executing Python tasks from packages
//! - Concrete PyO3 bindings: [`PyContext`], [`PyWorkflowBuilder`], [`PyTaskHandle`],
//!   [`TaskDecorator`] (`@task`), and [`PythonTaskWrapper`] (implements [`crate::Task`])
//!
//! The `@task` decorator machinery and `WorkflowBuilder` context manager are compiled
//! into the cloacina binary. The `cloaca` Python wheel re-exports these types via its
//! `#[pymodule]` definition.

// Abstract executor interface (no PyO3 dependency)
pub mod executor;

// Concrete PyO3 bindings
pub mod context;
pub mod loader;
pub mod namespace;
pub mod task;
pub mod workflow;
pub mod workflow_context;

// Re-exports: abstract executor
pub use executor::{PythonExecutionError, PythonTaskExecutor, PythonTaskResult};

// Re-exports: PyO3 bindings
pub use context::PyContext;
pub use namespace::PyTaskNamespace;
pub use task::{task as task_decorator, PyTaskHandle, PythonTaskWrapper, TaskDecorator};
pub use workflow::{
    register_workflow_constructor as py_register_workflow_constructor, PyWorkflow,
    PyWorkflowBuilder,
};
pub use workflow_context::PyWorkflowContext;

// Re-exports: loader
pub use loader::{ensure_cloaca_module, import_and_register_python_workflow, PythonLoaderError};

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::ffi::c_str;
    use pyo3::prelude::*;

    #[test]
    fn test_python_workflow_via_with_gil() {
        Python::with_gil(|py| {
            // Push a workflow context
            task::push_workflow_context(PyWorkflowContext::new(
                "public",
                "embedded",
                "test_py_workflow",
            ));

            // Create and register a task via the @task decorator
            let decorator = task::task(
                Some("greet".to_string()),
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

            let func = py.eval(c_str!("lambda ctx: ctx"), None, None).unwrap();
            decorator.__call__(py, func.into()).unwrap();

            // Pop context
            task::pop_workflow_context();

            // Verify the task was registered
            let registry = crate::task::global_task_registry();
            let guard = registry.read();
            let ns = crate::TaskNamespace::new("public", "embedded", "test_py_workflow", "greet");
            assert!(
                guard.get(&ns).is_some(),
                "Python task should be registered in the global registry"
            );

            // Verify the task instance implements Task trait correctly
            let constructor = guard.get(&ns).unwrap();
            let task_instance = constructor();
            assert_eq!(task_instance.id(), "greet");
            assert!(task_instance.dependencies().is_empty());
        });
    }
}
