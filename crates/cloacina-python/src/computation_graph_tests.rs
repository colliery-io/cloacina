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
//!
//! Tests use the builder + @node decorator pattern matching the existing
//! WorkflowBuilder + @task pattern.

#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use pyo3::prelude::*;
    use serial_test::serial;

    use crate::computation_graph;

    /// Helper: run a Python script that defines a computation graph using the
    /// builder + @node pattern, then return the registered executor.
    fn define_graph_and_get_executor(
        py: Python<'_>,
        graph_name: &str,
        python_code: &std::ffi::CStr,
    ) {
        // Make our node decorator and builder available to the Python code
        let globals = py.import("builtins").unwrap().dict();
        let locals = pyo3::types::PyDict::new(py);

        // Register our functions in locals so Python can call them
        locals
            .set_item(
                "node",
                pyo3::wrap_pyfunction!(computation_graph::node, py).unwrap(),
            )
            .unwrap();
        locals
            .set_item(
                "ComputationGraphBuilder",
                py.get_type::<computation_graph::PyComputationGraphBuilder>(),
            )
            .unwrap();

        py.run(python_code, Some(&globals), Some(&locals)).unwrap();
    }

    #[test]
    #[serial]
    fn test_linear_graph_via_builder() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            define_graph_and_get_executor(
                py,
                "linear_test",
                c_str!(
                    r#"
with ComputationGraphBuilder("linear_test",
    react={"mode": "when_any", "accumulators": ["alpha"]},
    graph={
        "entry": {"inputs": ["alpha"], "next": "output"},
        "output": {},
    }
) as builder:

    @node
    def entry(alpha):
        if alpha is None:
            return {"value": 0.0}
        return {"value": alpha["value"] * 2.0}

    @node
    def output(data):
        return {"result": data["value"] + 10.0, "done": True}
"#
                ),
            );

            // Verify executor was registered
            let executor = computation_graph::get_graph_executor("linear_test");
            assert!(executor.is_some(), "executor should be registered");
        });
    }

    #[test]
    #[serial]
    fn test_routing_graph_via_builder() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            define_graph_and_get_executor(
                py,
                "routing_test",
                c_str!(
                    r#"
with ComputationGraphBuilder("routing_test",
    react={"mode": "when_any", "accumulators": ["alpha", "beta"]},
    graph={
        "decision": {"inputs": ["alpha", "beta"], "routes": {
            "Signal": "signal_handler",
            "NoAction": "audit_logger",
        }},
        "signal_handler": {},
        "audit_logger": {},
    }
) as builder:

    @node
    def decision(alpha, beta):
        a = alpha["value"] if alpha else 0.0
        b = beta["estimate"] if beta else 0.0
        if a + b > 10.0:
            return ("Signal", {"output": a + b})
        return ("NoAction", {"reason": "below threshold"})

    @node
    def signal_handler(signal):
        return {"published": True, "value": signal["output"]}

    @node
    def audit_logger(reason):
        return {"logged": True}
"#
                ),
            );

            let executor = computation_graph::get_graph_executor("routing_test");
            assert!(executor.is_some(), "routing executor should be registered");
        });
    }

    #[test]
    #[serial]
    fn test_missing_node_errors() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = pyo3::types::PyDict::new(py);
            locals
                .set_item(
                    "node",
                    pyo3::wrap_pyfunction!(computation_graph::node, py).unwrap(),
                )
                .unwrap();
            locals
                .set_item(
                    "ComputationGraphBuilder",
                    py.get_type::<computation_graph::PyComputationGraphBuilder>(),
                )
                .unwrap();

            // Define graph topology referencing "output" but don't define the function
            let result = py.run(
                c_str!(
                    r#"
with ComputationGraphBuilder("missing_test",
    react={"mode": "when_any", "accumulators": ["alpha"]},
    graph={
        "entry": {"inputs": ["alpha"], "next": "output"},
        "output": {},
    }
) as builder:

    @node
    def entry(alpha):
        return {"value": 1.0}
    # Missing: @node def output(data): ...
"#
                ),
                Some(&globals),
                Some(&locals),
            );

            assert!(result.is_err(), "should error on missing 'output' node");
        });
    }

    #[test]
    #[serial]
    fn test_orphan_node_errors() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = pyo3::types::PyDict::new(py);
            locals
                .set_item(
                    "node",
                    pyo3::wrap_pyfunction!(computation_graph::node, py).unwrap(),
                )
                .unwrap();
            locals
                .set_item(
                    "ComputationGraphBuilder",
                    py.get_type::<computation_graph::PyComputationGraphBuilder>(),
                )
                .unwrap();

            // Define a node function not referenced in topology
            let result = py.run(
                c_str!(
                    r#"
with ComputationGraphBuilder("orphan_test",
    react={"mode": "when_any", "accumulators": ["alpha"]},
    graph={
        "entry": {"inputs": ["alpha"]},
    }
) as builder:

    @node
    def entry(alpha):
        return {"value": 1.0}

    @node
    def orphan_func(data):
        return {"should": "error"}
"#
                ),
                Some(&globals),
                Some(&locals),
            );

            assert!(result.is_err(), "should error on orphan node function");
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_linear_graph_executes() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = pyo3::types::PyDict::new(py);
            locals
                .set_item(
                    "node",
                    pyo3::wrap_pyfunction!(computation_graph::node, py).unwrap(),
                )
                .unwrap();
            locals
                .set_item(
                    "ComputationGraphBuilder",
                    py.get_type::<computation_graph::PyComputationGraphBuilder>(),
                )
                .unwrap();

            py.run(
                c_str!(
                    r#"
with ComputationGraphBuilder("exec_linear",
    react={"mode": "when_any", "accumulators": ["alpha"]},
    graph={
        "entry": {"inputs": ["alpha"], "next": "output"},
        "output": {},
    }
) as builder:

    @node
    def entry(alpha):
        if alpha is None:
            return {"value": 0.0}
        return {"value": alpha["value"] * 2.0}

    @node
    def output(data):
        return {"result": data["value"] + 10.0, "done": True}
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();
        });

        let executor = computation_graph::get_graph_executor("exec_linear").unwrap();

        let mut cache = cloacina::computation_graph::types::InputCache::new();
        cache.update(
            cloacina::computation_graph::types::SourceName::new("alpha"),
            cloacina::computation_graph::types::serialize(&serde_json::json!({"value": 5.0}))
                .unwrap(),
        );

        let result = executor.execute(&cache).await;
        match &result {
            cloacina::computation_graph::GraphResult::Error(e) => {
                panic!("execution failed: {:?}", e)
            }
            _ => {}
        }
        assert!(result.is_completed(), "linear graph should complete");
    }

    #[tokio::test]
    #[serial]
    async fn test_routing_graph_executes_signal_path() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = pyo3::types::PyDict::new(py);
            locals
                .set_item(
                    "node",
                    pyo3::wrap_pyfunction!(computation_graph::node, py).unwrap(),
                )
                .unwrap();
            locals
                .set_item(
                    "ComputationGraphBuilder",
                    py.get_type::<computation_graph::PyComputationGraphBuilder>(),
                )
                .unwrap();

            py.run(
                c_str!(
                    r#"
with ComputationGraphBuilder("exec_routing",
    react={"mode": "when_any", "accumulators": ["alpha", "beta"]},
    graph={
        "decision": {"inputs": ["alpha", "beta"], "routes": {
            "Signal": "signal_handler",
            "NoAction": "audit_logger",
        }},
        "signal_handler": {},
        "audit_logger": {},
    }
) as builder:

    @node
    def decision(alpha, beta):
        a = alpha["value"] if alpha else 0.0
        b = beta["estimate"] if beta else 0.0
        if a + b > 10.0:
            return ("Signal", {"output": a + b})
        return ("NoAction", {"reason": "below threshold"})

    @node
    def signal_handler(signal):
        return {"published": True, "value": signal["output"]}

    @node
    def audit_logger(reason):
        return {"logged": True}
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();
        });

        let executor = computation_graph::get_graph_executor("exec_routing").unwrap();

        // Signal path: 8 + 5 = 13 > 10
        let mut cache = cloacina::computation_graph::types::InputCache::new();
        cache.update(
            cloacina::computation_graph::types::SourceName::new("alpha"),
            cloacina::computation_graph::types::serialize(&serde_json::json!({"value": 8.0}))
                .unwrap(),
        );
        cache.update(
            cloacina::computation_graph::types::SourceName::new("beta"),
            cloacina::computation_graph::types::serialize(&serde_json::json!({"estimate": 5.0}))
                .unwrap(),
        );

        let result = executor.execute(&cache).await;
        match &result {
            cloacina::computation_graph::GraphResult::Error(e) => {
                panic!("routing execution failed: {:?}", e)
            }
            _ => {}
        }
        assert!(result.is_completed(), "routing graph should complete");
    }

    // =========================================================================
    // Accumulator decorator tests
    // =========================================================================

    /// Helper: set up Python environment with accumulator decorators available.
    fn setup_accumulator_env(py: Python<'_>) -> Bound<'_, pyo3::types::PyDict> {
        let locals = pyo3::types::PyDict::new(py);

        locals
            .set_item(
                "passthrough_accumulator",
                pyo3::wrap_pyfunction!(computation_graph::passthrough_accumulator_decorator, py)
                    .unwrap(),
            )
            .unwrap();
        locals
            .set_item(
                "stream_accumulator",
                pyo3::wrap_pyfunction!(computation_graph::stream_accumulator_decorator, py)
                    .unwrap(),
            )
            .unwrap();
        locals
            .set_item(
                "polling_accumulator",
                pyo3::wrap_pyfunction!(computation_graph::polling_accumulator_decorator, py)
                    .unwrap(),
            )
            .unwrap();
        locals
            .set_item(
                "batch_accumulator",
                pyo3::wrap_pyfunction!(computation_graph::batch_accumulator_decorator, py).unwrap(),
            )
            .unwrap();

        locals
    }

    #[test]
    #[serial]
    fn test_passthrough_accumulator_decorator() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = setup_accumulator_env(py);

            py.run(
                c_str!(
                    r#"
@passthrough_accumulator
def pricing(event):
    return {"price": event["mid_price"], "change": 0.0}

# Verify the function still works (transparent decorator)
result = pricing({"mid_price": 100.5})
assert result == {"price": 100.5, "change": 0.0}, f"unexpected result: {result}"
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            // Verify it was registered
            let accumulators = computation_graph::get_registered_accumulators();
            let pricing_acc = accumulators
                .iter()
                .find(|a| a.name == "pricing")
                .expect("pricing accumulator should be registered");
            assert_eq!(pricing_acc.accumulator_type, "passthrough");
            assert!(pricing_acc.config.is_empty());
        });
    }

    #[test]
    #[serial]
    fn test_stream_accumulator_decorator() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = setup_accumulator_env(py);

            py.run(
                c_str!(
                    r#"
@stream_accumulator(type="kafka", topic="market.orderbook")
def orderbook(event):
    return {"bid": event["best_bid"], "ask": event["best_ask"]}

result = orderbook({"best_bid": 100.0, "best_ask": 100.1})
assert result["bid"] == 100.0
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            let accumulators = computation_graph::get_registered_accumulators();
            let ob_acc = accumulators
                .iter()
                .find(|a| a.name == "orderbook")
                .expect("orderbook accumulator should be registered");
            assert_eq!(ob_acc.accumulator_type, "stream");
            assert_eq!(ob_acc.config.get("type").unwrap(), "kafka");
            assert_eq!(ob_acc.config.get("topic").unwrap(), "market.orderbook");
        });
    }

    #[test]
    #[serial]
    fn test_polling_accumulator_decorator() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = setup_accumulator_env(py);

            py.run(
                c_str!(
                    r#"
@polling_accumulator(interval="5s")
def config_source():
    return {"version": 1}
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            let accumulators = computation_graph::get_registered_accumulators();
            let cfg_acc = accumulators
                .iter()
                .find(|a| a.name == "config_source")
                .expect("config_source accumulator should be registered");
            assert_eq!(cfg_acc.accumulator_type, "polling");
            assert_eq!(cfg_acc.config.get("interval").unwrap(), "5s");
        });
    }

    #[test]
    #[serial]
    fn test_batch_accumulator_decorator() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let globals = py.import("builtins").unwrap().dict();
            let locals = setup_accumulator_env(py);

            py.run(
                c_str!(
                    r#"
@batch_accumulator(flush_interval="1s", max_buffer_size=100)
def aggregate_fills(events):
    total = sum(e["qty"] for e in events)
    return {"total_qty": total, "count": len(events)}

result = aggregate_fills([{"qty": 10}, {"qty": 20}])
assert result["total_qty"] == 30
assert result["count"] == 2
"#
                ),
                Some(&globals),
                Some(&locals),
            )
            .unwrap();

            let accumulators = computation_graph::get_registered_accumulators();
            let fill_acc = accumulators
                .iter()
                .find(|a| a.name == "aggregate_fills")
                .expect("aggregate_fills accumulator should be registered");
            assert_eq!(fill_acc.accumulator_type, "batch");
            assert_eq!(fill_acc.config.get("flush_interval").unwrap(), "1s");
            assert_eq!(fill_acc.config.get("max_buffer_size").unwrap(), "100");
        });
    }
}
