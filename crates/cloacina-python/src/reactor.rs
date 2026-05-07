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

//! `@cloaca.reactor` class decorator — Python parity for Rust's `#[reactor]`.
//!
//! Mirrors the Rust attribute macro: the decorated class becomes a handle that
//! carries `NAME`, `ACCUMULATORS`, and `REACTION_MODE` class attributes, and a
//! `ReactorRegistration` is registered into the active scoped Runtime.
//!
//! ```python
//! @cloaca.reactor(
//!     name="risk_signals",
//!     accumulators=["alpha", "beta"],
//!     mode="when_any",
//! )
//! class RiskSignals:
//!     pass
//! ```
//!
//! The class is then referenced by the (forthcoming) `reactor=ReactorClass`
//! kwarg on `ComputationGraphBuilder` and by `@cloaca.task(invokes=...)`.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString, PyTuple, PyType};

use cloacina_computation_graph::{ReactionMode, ReactorRegistration};

use crate::runtime_scope::current_runtime;

// Re-export the dispatch helper that lives in cloacina (no pyo3 deps) so
// existing T-0545 M2 tests keep their import path. T-0545 M3a moved the
// implementation to `cloacina::computation_graph::packaging_bridge` so the
// reconciler can call it directly without crossing the FFI boundary.
pub use cloacina::computation_graph::packaging_bridge::dispatch_runtime_reactors_into_scheduler;

/// `@cloaca.reactor(name=..., accumulators=[...], mode="when_any"|"when_all")`.
///
/// Returns a decorator that, when applied to a class, validates the inputs,
/// sets `NAME`/`ACCUMULATORS`/`REACTION_MODE` on the class, and registers a
/// `ReactorRegistration` into the active scoped Runtime.
#[pyfunction]
#[pyo3(name = "reactor", signature = (*, name, accumulators, mode = "when_any".to_string()))]
pub fn reactor(
    py: Python<'_>,
    name: String,
    accumulators: Vec<String>,
    mode: String,
) -> PyResult<PyObject> {
    if name.is_empty() {
        return Err(PyValueError::new_err(
            "@cloaca.reactor: 'name' cannot be empty",
        ));
    }
    if accumulators.is_empty() {
        return Err(PyValueError::new_err(
            "@cloaca.reactor: 'accumulators' list cannot be empty",
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for acc in &accumulators {
        if !seen.insert(acc.clone()) {
            return Err(PyValueError::new_err(format!(
                "@cloaca.reactor: accumulator '{}' listed more than once",
                acc
            )));
        }
    }
    let reaction_mode = match mode.as_str() {
        "when_any" => ReactionMode::WhenAny,
        "when_all" => ReactionMode::WhenAll,
        other => {
            return Err(PyValueError::new_err(format!(
                "@cloaca.reactor: unknown mode '{}', expected 'when_any' or 'when_all'",
                other
            )));
        }
    };

    // Capture state for the decorator closure.
    let name_clone = name.clone();
    let accumulators_clone = accumulators.clone();

    let decorator = pyo3::types::PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, pyo3::types::PyDict>>|
              -> PyResult<PyObject> {
            let py = args.py();
            let cls_obj = args.get_item(0)?;

            // Require a class (type). Reject instances / functions to give a clear error.
            let cls = cls_obj.downcast::<PyType>().map_err(|_| {
                PyTypeError::new_err(
                    "@cloaca.reactor must be applied to a class (e.g. `class Rx: pass`)",
                )
            })?;

            // Set class attributes — these mirror Rust's `Reactor` trait associated consts.
            cls.setattr("NAME", PyString::new(py, &name_clone))?;
            cls.setattr("ACCUMULATORS", PyList::new(py, &accumulators_clone)?)?;
            cls.setattr("REACTION_MODE", PyString::new(py, reaction_mode.as_str()))?;

            // Register into the active scoped Runtime.
            let rt = current_runtime().ok_or_else(|| {
                PyValueError::new_err(
                    "@cloaca.reactor: no scoped Runtime installed — \
                     decorator must run inside a process or loader thread \
                     that has installed a ScopedRuntime",
                )
            })?;

            let reg_name = name_clone.clone();
            let reg_accs = accumulators_clone.clone();
            let reg_mode = reaction_mode;
            rt.register_reactor(reg_name.clone(), move || ReactorRegistration {
                name: reg_name.clone(),
                accumulator_names: reg_accs.clone(),
                reaction_mode: reg_mode,
            });

            tracing::debug!(
                "Registered Python reactor: name={} accumulators={:?} mode={}",
                name_clone,
                accumulators_clone,
                reaction_mode.as_str()
            );

            Ok(cls.clone().unbind().into())
        },
    )?;

    Ok(decorator.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_scope::ScopedRuntime;
    use pyo3::ffi::c_str;
    use std::sync::Arc;

    #[test]
    fn reactor_decorator_sets_class_attrs_and_registers() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let rt = Arc::new(cloacina::Runtime::empty());
            let _scope = ScopedRuntime::new(rt.clone()).unwrap();

            let cloaca_reactor = wrap_pyfunction!(reactor, py).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("name", "risk_signals").unwrap();
            kwargs
                .set_item("accumulators", vec!["alpha", "beta"])
                .unwrap();
            kwargs.set_item("mode", "when_any").unwrap();
            let decorator = cloaca_reactor.call((), Some(&kwargs)).unwrap();

            // Build a Python class and apply the decorator.
            py.run(c_str!("class Rx: pass"), None, None).unwrap();
            let cls = py.eval(c_str!("Rx"), None, None).unwrap();
            let decorated = decorator.call1((cls,)).unwrap();

            let name: String = decorated.getattr("NAME").unwrap().extract().unwrap();
            assert_eq!(name, "risk_signals");
            let accs: Vec<String> = decorated
                .getattr("ACCUMULATORS")
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(accs, vec!["alpha", "beta"]);
            let mode: String = decorated
                .getattr("REACTION_MODE")
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(mode, "when_any");

            // Registered in the runtime.
            let reg = rt.get_reactor("risk_signals").expect("reactor registered");
            assert_eq!(reg.name, "risk_signals");
            assert_eq!(reg.accumulator_names, vec!["alpha", "beta"]);
            assert_eq!(reg.reaction_mode, ReactionMode::WhenAny);
        });
    }

    #[test]
    fn reactor_decorator_rejects_empty_name() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let rt = Arc::new(cloacina::Runtime::empty());
            let _scope = ScopedRuntime::new(rt).unwrap();
            let cloaca_reactor = wrap_pyfunction!(reactor, py).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("name", "").unwrap();
            kwargs.set_item("accumulators", vec!["a"]).unwrap();
            let err = cloaca_reactor.call((), Some(&kwargs)).unwrap_err();
            assert!(err.to_string().contains("'name' cannot be empty"));
        });
    }

    #[test]
    fn reactor_decorator_rejects_empty_accumulators() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let rt = Arc::new(cloacina::Runtime::empty());
            let _scope = ScopedRuntime::new(rt).unwrap();
            let cloaca_reactor = wrap_pyfunction!(reactor, py).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("name", "rx").unwrap();
            kwargs
                .set_item("accumulators", Vec::<String>::new())
                .unwrap();
            let err = cloaca_reactor.call((), Some(&kwargs)).unwrap_err();
            assert!(err.to_string().contains("cannot be empty"));
        });
    }

    #[test]
    fn reactor_decorator_rejects_duplicate_accumulators() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let rt = Arc::new(cloacina::Runtime::empty());
            let _scope = ScopedRuntime::new(rt).unwrap();
            let cloaca_reactor = wrap_pyfunction!(reactor, py).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("name", "rx").unwrap();
            kwargs.set_item("accumulators", vec!["a", "a"]).unwrap();
            let err = cloaca_reactor.call((), Some(&kwargs)).unwrap_err();
            assert!(err.to_string().contains("listed more than once"));
        });
    }

    #[test]
    fn reactor_decorator_rejects_unknown_mode() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let rt = Arc::new(cloacina::Runtime::empty());
            let _scope = ScopedRuntime::new(rt).unwrap();
            let cloaca_reactor = wrap_pyfunction!(reactor, py).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("name", "rx").unwrap();
            kwargs.set_item("accumulators", vec!["a"]).unwrap();
            kwargs.set_item("mode", "when_sometimes").unwrap();
            let err = cloaca_reactor.call((), Some(&kwargs)).unwrap_err();
            assert!(err.to_string().contains("unknown mode"));
        });
    }

    #[test]
    fn reactor_decorator_rejects_non_class_target() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let rt = Arc::new(cloacina::Runtime::empty());
            let _scope = ScopedRuntime::new(rt).unwrap();
            let cloaca_reactor = wrap_pyfunction!(reactor, py).unwrap();
            let kwargs = pyo3::types::PyDict::new(py);
            kwargs.set_item("name", "rx").unwrap();
            kwargs.set_item("accumulators", vec!["a"]).unwrap();
            let decorator = cloaca_reactor.call((), Some(&kwargs)).unwrap();

            let func = py.eval(c_str!("lambda: None"), None, None).unwrap();
            let err = decorator.call1((func,)).unwrap_err();
            assert!(err.to_string().contains("must be applied to a class"));
        });
    }
}
