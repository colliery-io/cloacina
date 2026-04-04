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

//! Tests for the Python computation graph bindings.

#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use pyo3::prelude::*;
    use pyo3::types::PyDict;

    use crate::python::computation_graph::computation_graph;

    #[test]
    fn test_linear_topology_parses() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_any", "accumulators": ["alpha"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(r#"{"entry": {"inputs": ["alpha"], "next": "output"}, "output": {}}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let result = computation_graph(py, &react_dict, &graph_dict);
            assert!(
                result.is_ok(),
                "linear topology parse failed: {:?}",
                result.err()
            );
        });
    }

    #[test]
    fn test_routing_topology_parses() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_any", "accumulators": ["alpha", "beta"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(
                        r#"{
                            "decision": {
                                "inputs": ["alpha", "beta"],
                                "routes": {"Signal": "handler_a", "NoAction": "handler_b"}
                            },
                            "handler_a": {},
                            "handler_b": {}
                        }"#
                    ),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let result = computation_graph(py, &react_dict, &graph_dict);
            assert!(
                result.is_ok(),
                "routing topology parse failed: {:?}",
                result.err()
            );
        });
    }

    #[test]
    fn test_when_all_mode_parses() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_all", "accumulators": ["a", "b", "c"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(r#"{"entry": {"inputs": ["a", "b", "c"]}}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let result = computation_graph(py, &react_dict, &graph_dict);
            assert!(result.is_ok(), "when_all parse failed: {:?}", result.err());
        });
    }

    #[test]
    fn test_missing_mode_errors() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(c_str!(r#"{"accumulators": ["alpha"]}"#), None, None)
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(c_str!(r#"{"entry": {"inputs": ["alpha"]}}"#), None, None)
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let result = computation_graph(py, &react_dict, &graph_dict);
            assert!(result.is_err(), "should error on missing mode");
        });
    }

    #[test]
    fn test_missing_accumulators_errors() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(c_str!(r#"{"mode": "when_any"}"#), None, None)
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(c_str!(r#"{"entry": {"inputs": ["alpha"]}}"#), None, None)
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let result = computation_graph(py, &react_dict, &graph_dict);
            assert!(result.is_err(), "should error on missing accumulators");
        });
    }

    #[test]
    fn test_decorator_applies_to_class_with_methods() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_any", "accumulators": ["alpha"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(r#"{"entry": {"inputs": ["alpha"], "next": "output"}, "output": {}}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let decorator_obj = computation_graph(py, &react_dict, &graph_dict).unwrap();

            // Define a Python class with the required methods
            let locals = PyDict::new(py);
            py.run(
                c_str!(
                    r#"
class Strategy:
    def entry(self, alpha):
        return {"value": 1.0}
    def output(self, data):
        return {"done": True}
"#
                ),
                None,
                Some(&locals),
            )
            .unwrap();

            let cls = locals.get_item("Strategy").unwrap().unwrap();

            // Call the decorator with the class — should succeed
            let result = decorator_obj.call1(py, (cls,));
            assert!(result.is_ok(), "decorator call failed: {:?}", result.err());
        });
    }

    #[test]
    fn test_decorator_rejects_class_missing_methods() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_any", "accumulators": ["alpha"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(r#"{"entry": {"inputs": ["alpha"], "next": "output"}, "output": {}}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let decorator_obj = computation_graph(py, &react_dict, &graph_dict).unwrap();

            // Define a class MISSING the "output" method
            let locals = PyDict::new(py);
            py.run(
                c_str!(
                    r#"
class BadStrategy:
    def entry(self, alpha):
        return {"value": 1.0}
"#
                ),
                None,
                Some(&locals),
            )
            .unwrap();

            let cls = locals.get_item("BadStrategy").unwrap().unwrap();

            // Call the decorator — should error
            let result = decorator_obj.call1(py, (cls,));
            assert!(
                result.is_err(),
                "should reject class missing 'output' method"
            );
        });
    }

    #[tokio::test]
    async fn test_python_linear_graph_executes() {
        pyo3::prepare_freethreaded_python();

        // Build the executor inside the GIL
        let executor = Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_any", "accumulators": ["alpha"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(r#"{"entry": {"inputs": ["alpha"], "next": "output"}, "output": {}}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let decorator_obj = computation_graph(py, &react_dict, &graph_dict).unwrap();

            // Define a Python class with real computation
            let locals = PyDict::new(py);
            py.run(
                c_str!(
                    r#"
class Strategy:
    def entry(self, alpha):
        if alpha is None:
            return {"value": 0.0}
        return {"value": alpha["value"] * 2.0}
    def output(self, data):
        return {"result": data["value"] + 10.0, "done": True}
"#
                ),
                None,
                Some(&locals),
            )
            .unwrap();

            let cls = locals.get_item("Strategy").unwrap().unwrap();
            // Call decorator to get executor, then extract the inner PythonGraphExecutor
            // We use the public build_executor_from_class helper
            let decorator_obj_bound = decorator_obj.bind(py);
            let executor_pyobj = decorator_obj_bound.call1((cls,)).unwrap();
            // Get the Rust struct out of the pyclass
            let executor_ref = executor_pyobj
                .downcast::<crate::python::computation_graph::PythonGraphExecutor>()
                .unwrap();
            // We need to take ownership — use a helper that clones the inner data
            crate::python::computation_graph::PythonGraphExecutor::clone_for_test(
                &executor_ref.borrow(),
                py,
            )
        });

        // Build an InputCache with test data
        let mut cache = crate::computation_graph::types::InputCache::new();
        cache.update(
            crate::computation_graph::types::SourceName::new("alpha"),
            crate::computation_graph::types::serialize(&serde_json::json!({"value": 5.0})).unwrap(),
        );

        // Execute the graph — this calls Python via spawn_blocking
        let result = executor.execute(&cache).await;
        match &result {
            crate::computation_graph::GraphResult::Error(e) => {
                panic!("graph execution failed: {:?}", e);
            }
            _ => {}
        }
        assert!(result.is_completed(), "graph should complete successfully");
    }

    #[tokio::test]
    async fn test_python_routing_graph_executes() {
        pyo3::prepare_freethreaded_python();

        let executor = Python::with_gil(|py| {
            let react_dict = py
                .eval(
                    c_str!(r#"{"mode": "when_any", "accumulators": ["alpha", "beta"]}"#),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let graph_dict = py
                .eval(
                    c_str!(
                        r#"{
                            "decision": {
                                "inputs": ["alpha", "beta"],
                                "routes": {"Signal": "signal_handler", "NoAction": "audit_logger"}
                            },
                            "signal_handler": {},
                            "audit_logger": {}
                        }"#
                    ),
                    None,
                    None,
                )
                .unwrap()
                .downcast_into::<PyDict>()
                .unwrap();

            let decorator_obj = computation_graph(py, &react_dict, &graph_dict).unwrap();

            let locals = PyDict::new(py);
            py.run(
                c_str!(
                    r#"
class RoutingStrategy:
    def decision(self, alpha, beta):
        a = alpha["value"] if alpha else 0.0
        b = beta["estimate"] if beta else 0.0
        if a + b > 10.0:
            return ("Signal", {"output": a + b})
        else:
            return ("NoAction", {"reason": "below threshold"})
    def signal_handler(self, signal):
        return {"published": True, "value": signal["output"]}
    def audit_logger(self, reason):
        return {"logged": True}
"#
                ),
                None,
                Some(&locals),
            )
            .unwrap();

            let cls = locals.get_item("RoutingStrategy").unwrap().unwrap();
            // Call decorator to get executor, then extract the inner PythonGraphExecutor
            // We use the public build_executor_from_class helper
            let decorator_obj_bound = decorator_obj.bind(py);
            let executor_pyobj = decorator_obj_bound.call1((cls,)).unwrap();
            // Get the Rust struct out of the pyclass
            let executor_ref = executor_pyobj
                .downcast::<crate::python::computation_graph::PythonGraphExecutor>()
                .unwrap();
            // We need to take ownership — use a helper that clones the inner data
            crate::python::computation_graph::PythonGraphExecutor::clone_for_test(
                &executor_ref.borrow(),
                py,
            )
        });

        // Test Signal path: 8 + 5 = 13 > 10
        let mut cache = crate::computation_graph::types::InputCache::new();
        cache.update(
            crate::computation_graph::types::SourceName::new("alpha"),
            crate::computation_graph::types::serialize(&serde_json::json!({"value": 8.0})).unwrap(),
        );
        cache.update(
            crate::computation_graph::types::SourceName::new("beta"),
            crate::computation_graph::types::serialize(&serde_json::json!({"estimate": 5.0}))
                .unwrap(),
        );

        let result = executor.execute(&cache).await;
        match &result {
            crate::computation_graph::GraphResult::Error(e) => {
                panic!("routing graph execution failed: {:?}", e);
            }
            _ => {}
        }
        assert!(
            result.is_completed(),
            "routing graph should complete on Signal path"
        );
    }
}
