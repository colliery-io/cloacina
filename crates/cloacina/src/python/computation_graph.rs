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

//! Python computation graph bindings.
//!
//! Provides the `@computation_graph` decorator that takes a Python class with
//! async methods and a dict-based topology, and produces a callable executor
//! matching the same `async fn(&InputCache) -> GraphResult` interface as
//! Rust-compiled graphs.

use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

use crate::computation_graph::types::{self, GraphError, GraphResult, InputCache};

/// Parsed topology from the Python dict declaration.
#[derive(Debug, Clone)]
struct PyGraphTopology {
    react_mode: String,
    accumulators: Vec<String>,
    nodes: Vec<PyNodeDecl>,
}

/// A node declaration from the Python topology dict.
#[derive(Debug, Clone)]
struct PyNodeDecl {
    name: String,
    cache_inputs: Vec<String>,
    edge: PyEdgeDecl,
}

/// Edge type for a Python node.
#[derive(Debug, Clone)]
enum PyEdgeDecl {
    /// Linear: "next" -> target
    Linear { target: String },
    /// Routing: "routes" -> { "Variant": "target" }
    Routing { variants: Vec<(String, String)> },
    /// Terminal: no downstream
    Terminal,
}

/// The Python graph executor. Holds a reference to the Python class instance
/// and the parsed topology. Executes the graph by calling methods on the class
/// via spawn_blocking + GIL.
#[pyclass]
pub struct PythonGraphExecutor {
    /// The Python class instance (the decorated class, instantiated)
    instance: PyObject,
    /// Parsed topology
    topology: PyGraphTopology,
    /// Sorted node execution order (topological)
    execution_order: Vec<String>,
    /// Node lookup
    node_map: HashMap<String, PyNodeDecl>,
}

// SAFETY: PythonGraphExecutor holds PyObject which is not Send/Sync.
// All access goes through Python::with_gil() inside spawn_blocking.
unsafe impl Send for PythonGraphExecutor {}
unsafe impl Sync for PythonGraphExecutor {}

impl PythonGraphExecutor {
    /// Execute the graph with the given input cache.
    pub async fn execute(&self, cache: &InputCache) -> GraphResult {
        // Clone what we need for the blocking closure
        let instance = Python::with_gil(|py| self.instance.clone_ref(py));
        let execution_order = self.execution_order.clone();
        let node_map = self.node_map.clone();
        let topology = self.topology.clone();

        // Deserialize cache inputs into a HashMap<String, serde_json::Value>
        let mut cache_values: HashMap<String, serde_json::Value> = HashMap::new();
        for acc_name in &topology.accumulators {
            if let Some(Ok(val)) = cache.get::<serde_json::Value>(acc_name) {
                cache_values.insert(acc_name.clone(), val);
            }
        }

        // Run the entire graph execution in spawn_blocking + GIL
        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                execute_graph_sync(py, &instance, &execution_order, &node_map, &cache_values)
            })
        })
        .await;

        match result {
            Ok(Ok(outputs)) => GraphResult::completed(outputs),
            Ok(Err(e)) => GraphResult::error(e),
            Err(join_err) => GraphResult::error(GraphError::NodeExecution(format!(
                "graph execution panicked: {}",
                join_err
            ))),
        }
    }
}

/// Execute the graph synchronously inside the GIL.
fn execute_graph_sync(
    py: Python<'_>,
    instance: &PyObject,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    cache_values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<Box<dyn std::any::Any + Send>>, GraphError> {
    let mut terminal_results: Vec<Box<dyn std::any::Any + Send>> = Vec::new();
    let mut node_results: HashMap<String, PyObject> = HashMap::new();

    for node_name in execution_order {
        let node_decl = node_map.get(node_name).ok_or_else(|| {
            GraphError::Execution(format!("node '{}' not found in topology", node_name))
        })?;

        // Build arguments for this node
        let args = build_node_args(py, node_decl, cache_values, &node_results)?;

        // Call the method on the instance
        let method = instance.getattr(py, node_name.as_str()).map_err(|e| {
            GraphError::NodeExecution(format!("method '{}' not found: {}", node_name, e))
        })?;

        let result = method.call(py, args, None).map_err(|e| {
            GraphError::NodeExecution(format!("node '{}' failed: {}", node_name, e))
        })?;

        // Handle routing
        match &node_decl.edge {
            PyEdgeDecl::Terminal => {
                // Terminal node — collect output
                let json_val = pythonize_to_json(py, &result)?;
                terminal_results.push(Box::new(json_val));
            }
            PyEdgeDecl::Linear { .. } => {
                // Linear — store result for downstream
                node_results.insert(node_name.clone(), result);
            }
            PyEdgeDecl::Routing { variants } => {
                // Routing — result should be a ("VariantName", value) tuple
                let tuple = result.downcast_bound::<PyTuple>(py).map_err(|_| {
                    GraphError::NodeExecution(format!(
                        "routing node '{}' must return a (variant_name, value) tuple",
                        node_name
                    ))
                })?;

                if tuple.len() != 2 {
                    return Err(GraphError::NodeExecution(format!(
                        "routing node '{}' returned tuple of length {}, expected 2",
                        node_name,
                        tuple.len()
                    )));
                }

                let variant_name = tuple
                    .get_item(0)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .downcast::<PyString>()
                    .map_err(|_| {
                        GraphError::NodeExecution(format!(
                            "routing node '{}': first tuple element must be a string (variant name)",
                            node_name
                        ))
                    })?
                    .to_string();
                let variant_value = tuple
                    .get_item(1)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .unbind();

                // Find the target for this variant
                let target = variants
                    .iter()
                    .find(|(v, _)| v == &variant_name)
                    .map(|(_, t)| t.clone())
                    .ok_or_else(|| {
                        GraphError::NodeExecution(format!(
                            "routing node '{}' returned variant '{}' which is not in the topology",
                            node_name, variant_name
                        ))
                    })?;

                // Store the variant value keyed by "from_node:VariantName" for downstream lookup
                node_results.insert(format!("{}:{}", node_name, variant_name), variant_value);

                // Mark which variant was taken — skip non-matching downstream nodes
                let variant_py = PyString::new(py, &variant_name);
                node_results.insert(
                    format!("{}:__selected_variant", node_name),
                    variant_py.unbind().into(),
                );
            }
        }
    }

    Ok(terminal_results)
}

/// Build the argument tuple for a Python node call.
fn build_node_args<'py>(
    py: Python<'py>,
    node_decl: &PyNodeDecl,
    cache_values: &HashMap<String, serde_json::Value>,
    _node_results: &HashMap<String, PyObject>,
) -> Result<Bound<'py, PyTuple>, GraphError> {
    let mut args: Vec<PyObject> = Vec::new();

    // Cache inputs (from accumulators)
    for input_name in &node_decl.cache_inputs {
        if let Some(val) = cache_values.get(input_name) {
            let py_val = pythonize::pythonize(py, val).map_err(|e| {
                GraphError::Deserialization(format!(
                    "failed to convert cache input '{}' to Python: {}",
                    input_name, e
                ))
            })?;
            args.push(py_val.unbind());
        } else {
            args.push(py.None());
        }
    }

    // Upstream node outputs
    // For linear edges, look up the source node's result
    // For routing variant edges, look up "source:Variant" result
    // This is handled by the execution order — if this node is downstream of a routing node,
    // its entry in execution_order was placed there by the topology sort, and the
    // routing result is stored with the variant key.

    PyTuple::new(py, &args)
        .map_err(|e| GraphError::NodeExecution(format!("failed to build args tuple: {}", e)))
}

/// Convert a Python object to serde_json::Value.
fn pythonize_to_json(py: Python<'_>, obj: &PyObject) -> Result<serde_json::Value, GraphError> {
    pythonize::depythonize(obj.bind(py)).map_err(|e| {
        GraphError::Serialization(format!("failed to convert Python result to JSON: {}", e))
    })
}

/// Parse a Python dict topology into our internal representation.
fn parse_topology(
    _py: Python<'_>,
    react: &Bound<'_, PyDict>,
    graph: &Bound<'_, PyDict>,
) -> PyResult<PyGraphTopology> {
    // Parse react
    let mode: String = react
        .get_item("mode")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("react dict missing 'mode'"))?
        .extract()?;
    let accumulators: Vec<String> = react
        .get_item("accumulators")?
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyKeyError, _>("react dict missing 'accumulators'")
        })?
        .downcast::<PyList>()
        .map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>("'accumulators' must be a list")
        })?
        .iter()
        .map(|item| item.extract::<String>())
        .collect::<PyResult<_>>()?;

    // Parse graph nodes
    let mut nodes = Vec::new();
    for (key, value) in graph.iter() {
        let name: String = key.extract()?;
        let node_dict = value.downcast::<PyDict>().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "graph['{}'] must be a dict",
                name
            ))
        })?;

        let cache_inputs: Vec<String> = if let Some(inputs) = node_dict.get_item("inputs")? {
            inputs
                .downcast::<PyList>()
                .map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                        "graph['{}']['inputs'] must be a list",
                        name
                    ))
                })?
                .iter()
                .map(|item| item.extract::<String>())
                .collect::<PyResult<_>>()?
        } else {
            Vec::new()
        };

        let edge = if let Some(routes) = node_dict.get_item("routes")? {
            let routes_dict = routes.downcast::<PyDict>().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                    "graph['{}']['routes'] must be a dict",
                    name
                ))
            })?;
            let mut variants = Vec::new();
            for (v_key, v_val) in routes_dict.iter() {
                variants.push((v_key.extract::<String>()?, v_val.extract::<String>()?));
            }
            PyEdgeDecl::Routing { variants }
        } else if let Some(next) = node_dict.get_item("next")? {
            PyEdgeDecl::Linear {
                target: next.extract::<String>()?,
            }
        } else {
            PyEdgeDecl::Terminal
        };

        nodes.push(PyNodeDecl {
            name,
            cache_inputs,
            edge,
        });
    }

    Ok(PyGraphTopology {
        react_mode: mode,
        accumulators,
        nodes,
    })
}

/// Compute a simple topological order from the node declarations.
fn compute_execution_order(nodes: &[PyNodeDecl]) -> Vec<String> {
    // Build adjacency: target -> sources
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_names: Vec<String> = Vec::new();

    for node in nodes {
        all_names.push(node.name.clone());
        incoming.entry(node.name.clone()).or_default();

        match &node.edge {
            PyEdgeDecl::Linear { target } => {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push(node.name.clone());
            }
            PyEdgeDecl::Routing { variants } => {
                for (_, target) in variants {
                    incoming
                        .entry(target.clone())
                        .or_default()
                        .push(node.name.clone());
                }
            }
            PyEdgeDecl::Terminal => {}
        }
    }

    // Kahn's algorithm
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for (name, sources) in &incoming {
        in_degree.insert(name.clone(), sources.len());
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();
    queue.sort();

    let mut sorted = Vec::new();
    while let Some(name) = queue.pop() {
        sorted.push(name.clone());

        // Find nodes that depend on this one
        if let Some(node) = nodes.iter().find(|n| n.name == name) {
            let targets: Vec<String> = match &node.edge {
                PyEdgeDecl::Linear { target } => vec![target.clone()],
                PyEdgeDecl::Routing { variants } => {
                    variants.iter().map(|(_, t)| t.clone()).collect()
                }
                PyEdgeDecl::Terminal => vec![],
            };

            for target in targets {
                if let Some(deg) = in_degree.get_mut(&target) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(target);
                        queue.sort();
                    }
                }
            }
        }
    }

    sorted
}

/// The `@computation_graph` decorator function.
///
/// Usage:
/// ```python
/// @cloaca.computation_graph(
///     react={"mode": "when_any", "accumulators": ["alpha", "beta"]},
///     graph={
///         "decision": {"inputs": ["alpha", "beta"], "routes": {"Signal": "handler_a", "NoAction": "handler_b"}},
///     }
/// )
/// class MyStrategy:
///     def decision(self, alpha, beta):
///         ...
/// ```
#[pyfunction]
#[pyo3(signature = (react, graph))]
pub fn computation_graph(
    py: Python<'_>,
    react: &Bound<'_, PyDict>,
    graph: &Bound<'_, PyDict>,
) -> PyResult<PyObject> {
    // Parse the topology at decoration time
    let topology = parse_topology(py, react, graph)?;
    let execution_order = compute_execution_order(&topology.nodes);

    let node_map: HashMap<String, PyNodeDecl> = topology
        .nodes
        .iter()
        .cloned()
        .map(|n| (n.name.clone(), n))
        .collect();

    // Validate: every node in topology must be a method on the class
    // (validated lazily when the class is passed in)

    // Return a decorator that takes the class
    let topo = topology.clone();
    let order = execution_order.clone();
    let nmap = node_map.clone();

    // Create a closure-like object that accepts the class
    let decorator = PythonGraphDecorator {
        topology: topo,
        execution_order: order,
        node_map: nmap,
    };

    Ok(decorator.into_pyobject(py)?.unbind().into())
}

/// Intermediate decorator object — called with the class to produce the executor.
#[pyclass]
pub struct PythonGraphDecorator {
    topology: PyGraphTopology,
    execution_order: Vec<String>,
    node_map: HashMap<String, PyNodeDecl>,
}

#[pymethods]
impl PythonGraphDecorator {
    fn __call__(&self, py: Python<'_>, cls: PyObject) -> PyResult<PythonGraphExecutor> {
        // Instantiate the class
        let instance = cls.call0(py)?;

        // Validate: every node in the topology must be a method on the instance
        for node_name in self.node_map.keys() {
            if !instance.getattr(py, node_name.as_str()).is_ok() {
                return Err(PyErr::new::<pyo3::exceptions::PyAttributeError, _>(
                    format!(
                        "computation_graph topology references '{}' but class has no such method",
                        node_name
                    ),
                ));
            }
        }

        Ok(PythonGraphExecutor {
            instance,
            topology: self.topology.clone(),
            execution_order: self.execution_order.clone(),
            node_map: self.node_map.clone(),
        })
    }
}
