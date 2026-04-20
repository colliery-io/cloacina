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
//!   [`TaskDecorator`] (`@task`), and [`PythonTaskWrapper`] (implements [`cloacina::Task`])
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
pub use workflow::{py_register_workflow, PyWorkflow, PyWorkflowBuilder};
pub use workflow_context::PyWorkflowContext;

// Re-exports: trigger bindings
pub use trigger::{
    drain_python_triggers, trigger as trigger_decorator, PyTriggerResult, PythonTriggerDef,
    PythonTriggerWrapper, TriggerDecorator,
};

// Re-exports: loader
pub use loader::{
    ensure_cloaca_module, import_and_register_python_workflow, import_python_computation_graph,
    PythonLoaderError,
};

// Python API wrapper types (PyDefaultRunner, PyDatabaseAdmin, etc.)
pub mod bindings;

// Unpacks `.cloacina` archives for the Python loader. Previously lived in
// cloacina core under `registry/loader/python_loader`; moved here in
// CLOACI-T-0529 phase B because it's Python-specific.
pub mod package_loader;

// Implementation of `cloacina::python_runtime::PythonRuntime`. The server
// calls `install()` at startup to register this crate's impl into the
// cloacina-core dispatch slot.
mod runtime_impl;
pub use runtime_impl::{install, CloacinaPythonRuntime};

// Thread-local "current Runtime" slot used by decorator/loader paths to
// register into a scoped Runtime rather than cloacina's process-globals.
pub mod runtime_scope;
pub use runtime_scope::{current_runtime, ScopedRuntime};

// PyO3 module entry point for the `cloaca` Python wheel. Maturin points
// at this crate to build the standalone pip-installable wheel. Moved
// here from cloacina core in CLOACI-T-0529.
use pyo3::prelude::*;

#[pymodule]
fn cloaca(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<context::PyContext>()?;

    m.add_function(wrap_pyfunction!(task::task, m)?)?;
    m.add_class::<task::PyTaskHandle>()?;

    m.add_function(wrap_pyfunction!(trigger::trigger, m)?)?;
    m.add_class::<bindings::trigger::PyTriggerResult>()?;

    m.add_class::<workflow::PyWorkflowBuilder>()?;
    m.add_class::<workflow::PyWorkflow>()?;
    m.add_function(wrap_pyfunction!(workflow::py_register_workflow, m)?)?;

    m.add_class::<bindings::runner::PyDefaultRunner>()?;
    m.add_class::<bindings::runner::PyWorkflowResult>()?;
    m.add_class::<bindings::context::PyDefaultRunnerConfig>()?;

    m.add_class::<namespace::PyTaskNamespace>()?;
    m.add_class::<workflow_context::PyWorkflowContext>()?;
    m.add_class::<bindings::value_objects::PyRetryPolicy>()?;
    m.add_class::<bindings::value_objects::PyRetryPolicyBuilder>()?;
    m.add_class::<bindings::value_objects::PyBackoffStrategy>()?;
    m.add_class::<bindings::value_objects::PyRetryCondition>()?;

    m.add_class::<computation_graph::PyComputationGraphBuilder>()?;
    m.add_function(wrap_pyfunction!(computation_graph::node, m)?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::passthrough_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::stream_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::polling_accumulator_decorator,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(
        computation_graph::batch_accumulator_decorator,
        m
    )?)?;

    #[cfg(feature = "postgres")]
    {
        m.add_class::<bindings::admin::PyDatabaseAdmin>()?;
        m.add_class::<bindings::admin::PyTenantConfig>()?;
        m.add_class::<bindings::admin::PyTenantCredentials>()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::ffi::c_str;

    #[test]
    fn test_python_workflow_via_with_gil() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            // Install a scoped Runtime so @task registers into it rather
            // than the process-global registry (CLOACI-T-0509 step 2).
            let rt = std::sync::Arc::new(cloacina::Runtime::empty());
            let _scope =
                runtime_scope::ScopedRuntime::new(rt.clone()).expect("ScopedRuntime install");

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

            // Verify the task was registered in the scoped Runtime
            let ns =
                cloacina::TaskNamespace::new("public", "embedded", "test_py_workflow", "greet");
            let task_instance = rt
                .get_task(&ns)
                .expect("Python task should be registered in the scoped Runtime");
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
            // Computation graph decorators
            assert!(cloaca_mod.hasattr("passthrough_accumulator").unwrap());
            assert!(cloaca_mod.hasattr("stream_accumulator").unwrap());
            assert!(cloaca_mod.hasattr("polling_accumulator").unwrap());
            assert!(cloaca_mod.hasattr("batch_accumulator").unwrap());
            assert!(cloaca_mod.hasattr("node").unwrap());
            assert!(cloaca_mod.hasattr("ComputationGraphBuilder").unwrap());
            // Variable registry
            assert!(cloaca_mod.hasattr("var").unwrap());
            assert!(cloaca_mod.hasattr("var_or").unwrap());
        });
    }

    #[test]
    fn test_cloaca_var_and_var_or_from_python() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            loader::ensure_cloaca_module(py).unwrap();
            let cloaca = py.import("cloaca").unwrap();
            let locals = pyo3::types::PyDict::new(py);
            locals.set_item("cloaca", cloaca).unwrap();

            // var_or returns default when env var is not set
            let result: String = py
                .eval(
                    pyo3::ffi::c_str!("cloaca.var_or('UNIT_TEST_MISSING_VAR', 'fallback')"),
                    None,
                    Some(&locals),
                )
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(result, "fallback");

            // var raises KeyError when env var is not set
            let err = py.eval(
                pyo3::ffi::c_str!("cloaca.var('UNIT_TEST_MISSING_VAR')"),
                None,
                Some(&locals),
            );
            assert!(err.is_err(), "var() should raise KeyError for missing var");

            // Set env var and verify var() returns it
            std::env::set_var("CLOACINA_VAR_UNIT_TEST_PY_VAR", "hello_from_rust");
            let result: String = py
                .eval(
                    pyo3::ffi::c_str!("cloaca.var('UNIT_TEST_PY_VAR')"),
                    None,
                    Some(&locals),
                )
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(result, "hello_from_rust");
            std::env::remove_var("CLOACINA_VAR_UNIT_TEST_PY_VAR");

            // var_or returns value when env var IS set
            std::env::set_var("CLOACINA_VAR_UNIT_TEST_PY_OR", "real_value");
            let result: String = py
                .eval(
                    pyo3::ffi::c_str!("cloaca.var_or('UNIT_TEST_PY_OR', 'ignored')"),
                    None,
                    Some(&locals),
                )
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(result, "real_value");
            std::env::remove_var("CLOACINA_VAR_UNIT_TEST_PY_OR");
        });
    }

    #[test]
    fn test_cloaca_cg_decorators_are_callable() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            loader::ensure_cloaca_module(py).unwrap();

            // Verify decorators are callable (not just present as attributes)
            let cloaca = py.import("cloaca").unwrap();
            assert!(
                cloaca
                    .getattr("passthrough_accumulator")
                    .unwrap()
                    .is_callable(),
                "passthrough_accumulator should be callable"
            );
            assert!(
                cloaca.getattr("node").unwrap().is_callable(),
                "node should be callable"
            );
            assert!(
                cloaca.getattr("var").unwrap().is_callable(),
                "var should be callable"
            );
            assert!(
                cloaca.getattr("var_or").unwrap().is_callable(),
                "var_or should be callable"
            );

            // ComputationGraphBuilder should be a class (instantiable)
            let cgb = cloaca.getattr("ComputationGraphBuilder").unwrap();
            assert!(
                cgb.is_callable(),
                "ComputationGraphBuilder should be a class"
            );
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
