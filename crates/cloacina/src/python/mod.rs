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

// Computation graph bindings
pub mod computation_graph;
#[cfg(test)]
mod computation_graph_tests;

// Abstract executor interface (no PyO3 dependency)
pub mod executor;

// Concrete PyO3 bindings
pub mod context;
pub mod loader;
pub mod namespace;
pub mod task;
pub mod trigger;
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

// Re-exports: trigger bindings
pub use trigger::{
    drain_python_triggers, trigger as trigger_decorator, PyTriggerResult, PythonTriggerDef,
    PythonTriggerWrapper, TriggerDecorator,
};

// Re-exports: loader
pub use loader::{ensure_cloaca_module, import_and_register_python_workflow, PythonLoaderError};

// Python API wrapper types (PyDefaultRunner, PyDatabaseAdmin, etc.)
pub mod bindings;

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::ffi::c_str;
    use pyo3::prelude::*;

    #[test]
    fn test_python_workflow_via_with_gil() {
        pyo3::prepare_freethreaded_python();
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

    #[test]
    fn test_ensure_cloaca_module_registers_in_sys_modules() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            loader::ensure_cloaca_module(py).unwrap();

            let sys = py.import("sys").unwrap();
            let modules = sys.getattr("modules").unwrap();
            assert!(
                modules.contains("cloaca").unwrap(),
                "cloaca should be registered in sys.modules"
            );

            // Verify the module is importable
            let cloaca_mod = py.import("cloaca").unwrap();
            assert!(cloaca_mod.hasattr("task").unwrap());
            assert!(cloaca_mod.hasattr("trigger").unwrap());
            assert!(cloaca_mod.hasattr("TriggerResult").unwrap());
            assert!(cloaca_mod.hasattr("WorkflowBuilder").unwrap());
            assert!(cloaca_mod.hasattr("Context").unwrap());
        });
    }

    #[test]
    fn test_validate_no_stdlib_shadowing_rejects_os_py() {
        let dir = tempfile::TempDir::new().unwrap();
        let workflow_dir = dir.path().join("workflow");
        let vendor_dir = dir.path().join("vendor");
        std::fs::create_dir_all(&workflow_dir).unwrap();
        std::fs::create_dir_all(&vendor_dir).unwrap();

        // Create an os.py in workflow/ — should be rejected
        std::fs::write(workflow_dir.join("os.py"), "# malicious").unwrap();

        let err = loader::validate_no_stdlib_shadowing(&workflow_dir, &vendor_dir);
        assert!(err.is_err(), "Should reject package with os.py");
        assert!(
            err.unwrap_err().to_string().contains("os"),
            "Error should mention 'os'"
        );
    }

    #[test]
    fn test_validate_no_stdlib_shadowing_allows_normal_packages() {
        let dir = tempfile::TempDir::new().unwrap();
        let workflow_dir = dir.path().join("workflow");
        let vendor_dir = dir.path().join("vendor");
        std::fs::create_dir_all(&workflow_dir).unwrap();
        std::fs::create_dir_all(&vendor_dir).unwrap();

        // Normal module names should be fine
        std::fs::write(workflow_dir.join("my_tasks.py"), "# fine").unwrap();
        std::fs::write(workflow_dir.join("data_pipeline.py"), "# fine").unwrap();

        let result = loader::validate_no_stdlib_shadowing(&workflow_dir, &vendor_dir);
        assert!(result.is_ok(), "Normal packages should pass validation");
    }
}
