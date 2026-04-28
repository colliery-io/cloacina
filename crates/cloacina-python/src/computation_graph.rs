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
//! Mirrors the WorkflowBuilder + @task pattern. Two flavors:
//!
//! 1. **Reactor-triggered** — the graph is bound to a `@cloaca.reactor` class.
//!    Entry nodes consume the reactor's accumulator payloads.
//!
//!    ```python
//!    @cloaca.reactor(name="market_maker",
//!                    accumulators=["alpha", "beta"],
//!                    mode="when_any")
//!    class MarketMaker: pass
//!
//!    with cloaca.ComputationGraphBuilder("market_maker",
//!        reactor=MarketMaker,
//!        graph={...}) as builder: ...
//!    ```
//!
//! 2. **Trigger-less** — the graph is invoked by a workflow task via
//!    `@cloaca.task(invokes=...)`. Entry nodes take only the task context;
//!    no accumulator/cache inputs are permitted.
//!
//!    ```python
//!    with cloaca.ComputationGraphBuilder("score_transactions",
//!        graph={"score": {}}) as score_graph: ...
//!    ```
//!
//! Pre-split (bundled) form:
//! ```python
//! with cloaca.ComputationGraphBuilder("market_maker",
//!     react={"mode": "when_any", "accumulators": ["alpha", "beta"]},
//!     graph={
//!         "decision": {"inputs": ["alpha", "beta"], "routes": {
//!             "Signal": "signal_handler",
//!             "NoAction": "audit_logger",
//!         }},
//!         "signal_handler": {},
//!         "audit_logger": {},
//!     }
//! ) as builder:
//!
//!     @cloaca.node
//!     def decision(alpha, beta):
//!         if alpha["value"] + beta["estimate"] > 10:
//!             return ("Signal", {"output": alpha["value"] + beta["estimate"]})
//!         return ("NoAction", {"reason": "below threshold"})
//!
//!     @cloaca.node
//!     def signal_handler(signal):
//!         return {"published": True}
//!
//!     @cloaca.node
//!     def audit_logger(reason):
//!         return {"logged": True}
//! ```

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use pyo3::exceptions::{PyAttributeError, PyKeyError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyCFunction, PyDict, PyList, PyString, PyTuple};

use cloacina::computation_graph::types::{GraphError, GraphResult};

// ---------------------------------------------------------------------------
// Global node registry (scoped by active builder context)
// ---------------------------------------------------------------------------

static NODE_REGISTRY: Lazy<Mutex<HashMap<String, PyObject>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static ACTIVE_GRAPH_CONTEXT: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

fn push_graph_context(name: String) {
    let mut ctx = ACTIVE_GRAPH_CONTEXT.lock().unwrap();
    *ctx = Some(name);
}

fn pop_graph_context() {
    let mut ctx = ACTIVE_GRAPH_CONTEXT.lock().unwrap();
    *ctx = None;
}

fn current_graph_context() -> Option<String> {
    ACTIVE_GRAPH_CONTEXT.lock().unwrap().clone()
}

fn register_node(name: String, func: PyObject) {
    NODE_REGISTRY.lock().unwrap().insert(name, func);
}

fn drain_nodes() -> HashMap<String, PyObject> {
    let mut registry = NODE_REGISTRY.lock().unwrap();
    std::mem::take(&mut *registry)
}

// ---------------------------------------------------------------------------
// Global accumulator registry
// ---------------------------------------------------------------------------

/// Metadata for a registered Python accumulator.
#[derive(Debug, Clone)]
pub struct PyAccumulatorRegistration {
    pub name: String,
    pub accumulator_type: String, // "passthrough", "stream", "polling", "batch"
    pub config: HashMap<String, String>,
}

static ACCUMULATOR_REGISTRY: Lazy<Mutex<HashMap<String, (PyObject, PyAccumulatorRegistration)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn register_accumulator(name: String, func: PyObject, reg: PyAccumulatorRegistration) {
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .insert(name, (func, reg));
}

/// Get all registered accumulators (for testing/inspection).
pub fn get_registered_accumulators() -> Vec<PyAccumulatorRegistration> {
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .values()
        .map(|(_, reg)| reg.clone())
        .collect()
}

/// Drain all registered accumulators (used by builder on __exit__).
pub fn drain_accumulators() -> HashMap<String, (PyObject, PyAccumulatorRegistration)> {
    let mut registry = ACCUMULATOR_REGISTRY.lock().unwrap();
    std::mem::take(&mut *registry)
}

// ---------------------------------------------------------------------------
// @cloaca.passthrough_accumulator decorator
// ---------------------------------------------------------------------------

/// The `@cloaca.passthrough_accumulator` decorator.
/// Registers a function as a passthrough accumulator (Event → Output, no buffering).
#[pyfunction]
#[pyo3(name = "passthrough_accumulator")]
pub fn passthrough_accumulator_decorator(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    let reg = PyAccumulatorRegistration {
        name: func_name.clone(),
        accumulator_type: "passthrough".to_string(),
        config: HashMap::new(),
    };
    register_accumulator(func_name, func.clone_ref(py), reg);
    Ok(func)
}

// ---------------------------------------------------------------------------
// @cloaca.stream_accumulator(type=..., topic=...) decorator
// ---------------------------------------------------------------------------

/// Factory for `@cloaca.stream_accumulator(type=..., topic=...)`.
/// Returns a decorator function that registers the target with stream config.
#[pyfunction]
#[pyo3(name = "stream_accumulator", signature = (*, r#type, topic, group=None))]
pub fn stream_accumulator_decorator(
    py: Python<'_>,
    r#type: String,
    topic: String,
    group: Option<String>,
) -> PyResult<PyObject> {
    let config_type = r#type;
    let config_topic = topic;
    let config_group = group;

    // Return a decorator function
    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("type".to_string(), config_type.clone());
            config.insert("topic".to_string(), config_topic.clone());
            if let Some(ref g) = config_group {
                config.insert("group".to_string(), g.clone());
            }

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "stream".to_string(),
                config,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}

// ---------------------------------------------------------------------------
// @cloaca.polling_accumulator(interval=...) decorator
// ---------------------------------------------------------------------------

/// Factory for `@cloaca.polling_accumulator(interval=...)`.
#[pyfunction]
#[pyo3(name = "polling_accumulator", signature = (*, interval))]
pub fn polling_accumulator_decorator(py: Python<'_>, interval: String) -> PyResult<PyObject> {
    let config_interval = interval;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("interval".to_string(), config_interval.clone());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "polling".to_string(),
                config,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}

// ---------------------------------------------------------------------------
// @cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...) decorator
// ---------------------------------------------------------------------------

/// Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.
#[pyfunction]
#[pyo3(name = "batch_accumulator", signature = (*, flush_interval, max_buffer_size=None))]
pub fn batch_accumulator_decorator(
    py: Python<'_>,
    flush_interval: String,
    max_buffer_size: Option<usize>,
) -> PyResult<PyObject> {
    let config_interval = flush_interval;
    let config_max = max_buffer_size;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("flush_interval".to_string(), config_interval.clone());
            if let Some(max) = config_max {
                config.insert("max_buffer_size".to_string(), max.to_string());
            }

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "batch".to_string(),
                config,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}

// ---------------------------------------------------------------------------
// Topology types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PyNodeDecl {
    name: String,
    cache_inputs: Vec<String>,
    edge: PyEdgeDecl,
}

#[derive(Debug, Clone)]
enum PyEdgeDecl {
    Linear { target: String },
    Routing { variants: Vec<(String, String)> },
    Terminal,
}

// ---------------------------------------------------------------------------
// @cloaca.node decorator
// ---------------------------------------------------------------------------

/// The `@cloaca.node` decorator. Registers a function as a node in the
/// current ComputationGraphBuilder context.
#[pyfunction]
pub fn node(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let ctx = current_graph_context().ok_or_else(|| {
        PyValueError::new_err(
            "@cloaca.node must be used inside a ComputationGraphBuilder context manager",
        )
    })?;

    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    register_node(func_name, func.clone_ref(py));

    // Return the function unchanged (transparent decorator)
    Ok(func)
}

// ---------------------------------------------------------------------------
// ComputationGraphBuilder context manager
// ---------------------------------------------------------------------------

#[pyclass(name = "ComputationGraphBuilder")]
pub struct PyComputationGraphBuilder {
    name: String,
    /// Reactor binding for the split form; `None` means trigger-less.
    reactor_binding: Option<ReactorBinding>,
    nodes_decl: Vec<PyNodeDecl>,
}

#[derive(Debug, Clone)]
struct ReactorBinding {
    name: String,
    accumulators: Vec<String>,
    /// `"when_any"` | `"when_all"`.
    mode: String,
}

#[pymethods]
impl PyComputationGraphBuilder {
    #[new]
    #[pyo3(signature = (name, *, graph, reactor = None, react = None))]
    pub fn new(
        _py: Python<'_>,
        name: &str,
        graph: &Bound<'_, PyDict>,
        reactor: Option<&Bound<'_, PyAny>>,
        react: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Hard-break migration: bundled `react={...}` form is gone — point users
        // at I-0101 with a clear error.
        if react.is_some() {
            return Err(PyValueError::new_err(
                "ComputationGraphBuilder no longer accepts the bundled `react={...}` kwarg. \
                 Declare a `@cloaca.reactor` class and pass `reactor=YourReactor`, \
                 or omit `reactor` entirely for a trigger-less graph invoked by \
                 `@cloaca.task(invokes=...)`. See initiative CLOACI-I-0101.",
            ));
        }

        let reactor_binding = match reactor {
            None => None,
            Some(r) => Some(extract_reactor_binding(r)?),
        };

        let nodes_decl = parse_graph_dict(graph)?;

        Ok(PyComputationGraphBuilder {
            name: name.to_string(),
            reactor_binding,
            nodes_decl,
        })
    }

    /// The graph's declared name. Mirrors Rust's `Graph::NAME` so this
    /// instance can serve as the handle referenced by `@cloaca.task(invokes=...)`.
    #[getter]
    #[allow(non_snake_case)]
    pub fn NAME(&self) -> String {
        self.name.clone()
    }

    /// Context manager entry — establish graph context for @node decorators
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_graph_context(slf.name.clone());
        slf
    }

    /// Context manager exit — validate nodes against topology, build executor
    pub fn __exit__(
        &self,
        py: Python,
        _exc_type: Option<&Bound<PyAny>>,
        _exc_value: Option<&Bound<PyAny>>,
        _traceback: Option<&Bound<PyAny>>,
    ) -> PyResult<bool> {
        pop_graph_context();

        let registered_nodes = drain_nodes();

        // Validate: every node in topology must have a registered function
        for decl in &self.nodes_decl {
            if !registered_nodes.contains_key(&decl.name) {
                return Err(PyAttributeError::new_err(format!(
                    "computation graph '{}' topology references node '{}' but no @cloaca.node function with that name was defined",
                    self.name, decl.name
                )));
            }
        }

        // Validate: every registered function must appear in topology
        for fn_name in registered_nodes.keys() {
            if !self.nodes_decl.iter().any(|d| d.name == *fn_name) {
                return Err(PyValueError::new_err(format!(
                    "function '{}' was decorated with @cloaca.node but does not appear in the graph topology",
                    fn_name
                )));
            }
        }

        // Trigger-less entry contract: in trigger-less form, no node may
        // declare cache `inputs=[...]` — those inputs only exist when a
        // reactor is feeding the graph. Trigger-less graphs receive their
        // input via the task `Context` passed by `@cloaca.task(invokes=...)`.
        if self.reactor_binding.is_none() {
            for decl in &self.nodes_decl {
                if !decl.cache_inputs.is_empty() {
                    return Err(PyValueError::new_err(format!(
                        "trigger-less computation graph '{}': node '{}' declares cache \
                         inputs={:?}, but trigger-less graphs have no accumulator inputs. \
                         Either remove the inputs (graph receives data through the task \
                         context) or bind a reactor with `reactor=YourReactor`.",
                        self.name, decl.name, decl.cache_inputs
                    )));
                }
            }
        } else if let Some(ref binding) = self.reactor_binding {
            // Split-form sanity: every cache input declared by a node must be
            // one of the reactor's accumulators. Catches typos at registration
            // time rather than producing silent `None`s at execution time.
            for decl in &self.nodes_decl {
                for input in &decl.cache_inputs {
                    if !binding.accumulators.contains(input) {
                        return Err(PyValueError::new_err(format!(
                            "computation graph '{}': node '{}' references cache input \
                             '{}' which is not in reactor '{}' accumulators {:?}",
                            self.name, decl.name, input, binding.name, binding.accumulators
                        )));
                    }
                }
            }
        }

        // Build the executor
        let node_map: HashMap<String, PyNodeDecl> = self
            .nodes_decl
            .iter()
            .cloned()
            .map(|n| (n.name.clone(), n))
            .collect();
        let execution_order = compute_execution_order(&self.nodes_decl);

        let (react_mode, accumulators, reactor_name) = match &self.reactor_binding {
            Some(b) => (b.mode.clone(), b.accumulators.clone(), Some(b.name.clone())),
            None => (String::new(), Vec::new(), None),
        };

        let executor = PythonGraphExecutor {
            name: self.name.clone(),
            node_functions: registered_nodes,
            node_map,
            execution_order,
            react_mode,
            accumulators,
            has_reactor: self.reactor_binding.is_some(),
            reactor_name,
        };

        // Register the executor globally (similar to workflow registration)
        register_graph_executor(self.name.clone(), executor, py)?;

        Ok(false)
    }

    pub fn __repr__(&self) -> String {
        format!(
            "ComputationGraphBuilder(name='{}', nodes={})",
            self.name,
            self.nodes_decl.len()
        )
    }

    /// Execute the computation graph with the given input cache.
    ///
    /// `inputs` is a dict mapping source names to their data dicts.
    /// Returns the terminal node's output dict.
    pub fn execute(&self, py: Python<'_>, inputs: &Bound<'_, PyDict>) -> PyResult<PyObject> {
        let executor = get_graph_executor(&self.name).ok_or_else(|| {
            PyValueError::new_err(format!(
                "graph '{}' not built yet — call execute after the 'with' block exits",
                self.name
            ))
        })?;

        // Convert Python dict inputs → HashMap<String, PyObject>
        let mut cache: HashMap<String, PyObject> = HashMap::new();
        for (key, value) in inputs.iter() {
            let key_str: String = key.extract()?;
            cache.insert(key_str, value.unbind());
        }

        // Execute synchronously (this is a simplified path for tutorials)
        let result = executor.execute_sync(py, &cache)?;
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Graph executor
// ---------------------------------------------------------------------------

/// Global registry of graph executors.
static GRAPH_EXECUTORS: Lazy<Mutex<HashMap<String, PythonGraphExecutor>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn register_graph_executor(
    name: String,
    executor: PythonGraphExecutor,
    _py: Python<'_>,
) -> PyResult<()> {
    GRAPH_EXECUTORS.lock().unwrap().insert(name, executor);
    Ok(())
}

/// Get a registered graph executor by name (for testing / reactor use).
pub fn get_graph_executor(name: &str) -> Option<PythonGraphExecutor> {
    GRAPH_EXECUTORS.lock().unwrap().get(name).cloned()
}

#[pyclass]
pub struct PythonGraphExecutor {
    pub name: String,
    node_functions: HashMap<String, PyObject>,
    node_map: HashMap<String, PyNodeDecl>,
    execution_order: Vec<String>,
    react_mode: String,
    accumulators: Vec<String>,
    /// `true` when the graph is bound to a reactor. Trigger-less graphs
    /// (`false`) cannot be turned into a `ComputationGraphDeclaration`.
    pub has_reactor: bool,
    /// Reactor name from the `@cloaca.reactor`-decorated class. Threaded
    /// into the resulting `ComputationGraphDeclaration.reactor_name` so
    /// two packages declaring the same reactor share a single runtime
    /// instance (T-0544 M5 cross-package fan-out).
    pub reactor_name: Option<String>,
}

// SAFETY: All PyObject access goes through Python::with_gil() inside spawn_blocking.
unsafe impl Send for PythonGraphExecutor {}
unsafe impl Sync for PythonGraphExecutor {}

impl Clone for PythonGraphExecutor {
    fn clone(&self) -> Self {
        Python::with_gil(|py| Self {
            name: self.name.clone(),
            node_functions: self
                .node_functions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect(),
            node_map: self.node_map.clone(),
            execution_order: self.execution_order.clone(),
            react_mode: self.react_mode.clone(),
            accumulators: self.accumulators.clone(),
            has_reactor: self.has_reactor,
            reactor_name: self.reactor_name.clone(),
        })
    }
}

impl PythonGraphExecutor {
    /// Execute the graph synchronously from Python with dict inputs.
    ///
    /// Used by `ComputationGraphBuilder.execute()` for tutorials.
    pub fn execute_sync(
        &self,
        py: Python<'_>,
        inputs: &HashMap<String, PyObject>,
    ) -> PyResult<PyObject> {
        // Convert PyObject inputs to serde_json::Value for the executor
        let mut cache_values: HashMap<String, serde_json::Value> = HashMap::new();
        for (name, obj) in inputs {
            let val = pythonize::depythonize::<serde_json::Value>(&obj.bind(py))?;
            cache_values.insert(name.clone(), val);
        }

        let result = execute_graph_sync(
            py,
            &self.node_functions,
            &self.execution_order,
            &self.node_map,
            &cache_values,
        );

        match result {
            Ok(outputs) => {
                // Terminal results are Box<dyn Any + Send> containing serde_json::Value
                if let Some(last) = outputs.last() {
                    if let Some(json_val) = last.downcast_ref::<serde_json::Value>() {
                        let py_obj = pythonize::pythonize(py, json_val).map_err(|e| {
                            PyValueError::new_err(format!("result conversion failed: {}", e))
                        })?;
                        Ok(py_obj.unbind())
                    } else {
                        Ok(py.None().into())
                    }
                } else {
                    Ok(py.None().into())
                }
            }
            Err(e) => Err(PyValueError::new_err(format!(
                "graph execution failed: {}",
                e
            ))),
        }
    }

    /// Execute the graph with the given input cache.
    pub async fn execute(
        &self,
        cache: &cloacina::computation_graph::types::InputCache,
    ) -> GraphResult {
        let executor = self.clone();

        // Deserialize cache inputs
        let mut cache_values: HashMap<String, serde_json::Value> = HashMap::new();
        for acc_name in &executor.accumulators {
            if let Some(Ok(val)) = cache.get::<serde_json::Value>(acc_name) {
                cache_values.insert(acc_name.clone(), val);
            }
        }

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                execute_graph_sync(
                    py,
                    &executor.node_functions,
                    &executor.execution_order,
                    &executor.node_map,
                    &cache_values,
                )
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

    /// Terminal node names in `execution_order`, in the order their outputs
    /// will be appended to the result vector. Used by `@cloaca.task(invokes=...)`
    /// to route outputs back into the task context under their terminal name.
    pub fn terminal_names(&self) -> Vec<String> {
        self.execution_order
            .iter()
            .filter(|n| {
                matches!(
                    self.node_map.get(*n).map(|d| &d.edge),
                    Some(PyEdgeDecl::Terminal)
                )
            })
            .cloned()
            .collect()
    }

    /// Execute a trigger-less graph with a Python `Context` object.
    ///
    /// Each "root" node — one with no upstream edges and no cache inputs —
    /// receives the context as its single argument. Downstream nodes get
    /// their predecessor's output as before. Returns `(terminal_name, value)`
    /// pairs in the order each terminal *actually fired* — so routing
    /// branches that were skipped do not appear in the output. The task
    /// wrapper routes each pair into the task context under its terminal
    /// node name.
    pub async fn execute_trigger_less(
        &self,
        ctx: PyObject,
    ) -> Result<Vec<(String, serde_json::Value)>, GraphError> {
        let executor = self.clone();
        match tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                execute_graph_sync_named_terminals(
                    py,
                    &executor.node_functions,
                    &executor.execution_order,
                    &executor.node_map,
                    &ctx,
                )
            })
        })
        .await
        {
            Ok(Ok(pairs)) => Ok(pairs),
            Ok(Err(e)) => Err(e),
            Err(join_err) => Err(GraphError::NodeExecution(format!(
                "trigger-less graph execution panicked: {}",
                join_err
            ))),
        }
    }
}

/// Build a [`ComputationGraphDeclaration`] from a registered Python graph executor.
///
/// This bridges the Python CG world (decorators + `ComputationGraphBuilder`) into
/// the Rust `ComputationGraphScheduler` by wrapping the Python executor in a `CompiledGraphFn`.
pub fn build_python_graph_declaration(
    graph_name: &str,
    tenant_id: Option<String>,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
) -> Option<cloacina::computation_graph::scheduler::ComputationGraphDeclaration> {
    use cloacina::computation_graph::packaging_bridge::{
        PassthroughAccumulatorFactory, StreamBackendAccumulatorFactory,
    };
    use cloacina::computation_graph::reactor::{InputStrategy, ReactionCriteria};
    use cloacina::computation_graph::scheduler::{
        AccumulatorDeclaration, ComputationGraphDeclaration, ReactorDeclaration,
    };
    use cloacina_computation_graph::InputCache;
    use std::sync::Arc;

    let executor = get_graph_executor(graph_name)?;

    // Trigger-less graphs aren't reactor-driven and therefore have no
    // ComputationGraphDeclaration to publish. Callers that need the graph
    // (e.g. a `@cloaca.task(invokes=...)` body) look it up through
    // `get_graph_executor` directly.
    if !executor.has_reactor {
        return None;
    }

    let criteria = match executor.react_mode.as_str() {
        "when_all" => ReactionCriteria::WhenAll,
        _ => ReactionCriteria::WhenAny,
    };

    let accumulator_names = executor.accumulators.clone();
    let reactor_name = executor.reactor_name.clone();

    // Build CompiledGraphFn from the Python executor
    let graph_fn: cloacina_computation_graph::CompiledGraphFn =
        Arc::new(move |cache: InputCache| {
            let exec = executor.clone();
            Box::pin(async move { exec.execute(&cache).await })
        });

    // Build accumulator declarations — use package.toml overrides when available
    let accumulators = accumulator_names
        .iter()
        .map(|name| {
            let factory: Arc<dyn cloacina::computation_graph::scheduler::AccumulatorFactory> =
                if let Some(override_cfg) = accumulator_overrides.iter().find(|a| a.name == *name) {
                    match override_cfg.accumulator_type.as_str() {
                        "stream" => Arc::new(StreamBackendAccumulatorFactory::new(
                            override_cfg.config.clone(),
                        )),
                        _ => Arc::new(PassthroughAccumulatorFactory),
                    }
                } else {
                    Arc::new(PassthroughAccumulatorFactory)
                };
            AccumulatorDeclaration {
                name: name.clone(),
                factory,
            }
        })
        .collect();

    Some(ComputationGraphDeclaration {
        name: graph_name.to_string(),
        accumulators,
        reactor: ReactorDeclaration {
            criteria,
            strategy: InputStrategy::Latest,
            graph_fn,
        },
        tenant_id,
        // T-0544 M5: thread the reactor name from the `@cloaca.reactor`
        // class through to the scheduler so Python packages naming the
        // same reactor as a Rust (or Python) package share a runtime
        // instance via M2's idempotent registration path.
        reactor_name,
    })
}

// ---------------------------------------------------------------------------
// Graph execution (synchronous, inside GIL)
// ---------------------------------------------------------------------------

/// Sync helper used by both reactor-triggered and trigger-less paths.
/// `trigger_less_ctx`, when `Some`, is passed as the sole argument to every
/// "root" node (one with no incoming edges and no cache inputs). The
/// reactor-triggered path leaves this `None`.
fn execute_graph_sync(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    cache_values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<Box<dyn std::any::Any + Send>>, GraphError> {
    execute_graph_sync_inner(
        py,
        node_functions,
        execution_order,
        node_map,
        cache_values,
        None,
    )
}

fn execute_graph_sync_with_context(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    ctx: &PyObject,
) -> Result<Vec<Box<dyn std::any::Any + Send>>, GraphError> {
    let empty_cache: HashMap<String, serde_json::Value> = HashMap::new();
    execute_graph_sync_inner(
        py,
        node_functions,
        execution_order,
        node_map,
        &empty_cache,
        Some(ctx),
    )
}

/// Trigger-less variant that returns `(terminal_name, value)` pairs in the
/// order each terminal fired. Routing branches that aren't selected emit no
/// pair, so consumers (e.g. `@cloaca.task(invokes=...)`) can route outputs
/// to context keys without misalignment.
fn execute_graph_sync_named_terminals(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    ctx: &PyObject,
) -> Result<Vec<(String, serde_json::Value)>, GraphError> {
    let cache_values: HashMap<String, serde_json::Value> = HashMap::new();
    let trigger_less_ctx = Some(ctx);

    let mut pairs: Vec<(String, serde_json::Value)> = Vec::new();
    let mut node_results: HashMap<String, PyObject> = HashMap::new();

    let mut incoming: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for node in node_map.values() {
        match &node.edge {
            PyEdgeDecl::Linear { target } => {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push((node.name.clone(), None));
            }
            PyEdgeDecl::Routing { variants } => {
                for (variant_name, target) in variants {
                    incoming
                        .entry(target.clone())
                        .or_default()
                        .push((node.name.clone(), Some(variant_name.clone())));
                }
            }
            PyEdgeDecl::Terminal => {}
        }
    }

    let mut skipped_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for node_name in execution_order {
        if skipped_nodes.contains(node_name) {
            continue;
        }

        let node_decl = node_map.get(node_name).ok_or_else(|| {
            GraphError::Execution(format!("node '{}' not found in topology", node_name))
        })?;

        if let Some(sources) = incoming.get(node_name) {
            let mut should_skip = false;
            for (from_node, variant) in sources {
                if let Some(v) = variant {
                    let selected_key = format!("{}:__selected_variant", from_node);
                    if let Some(selected) = node_results.get(&selected_key) {
                        let selected_str: String = selected.extract(py).unwrap_or_default();
                        if selected_str != *v {
                            should_skip = true;
                        }
                    }
                }
            }
            if should_skip {
                skipped_nodes.insert(node_name.clone());
                continue;
            }
        }

        let args = build_node_args(
            py,
            node_name,
            node_decl,
            &cache_values,
            &node_results,
            &incoming,
            trigger_less_ctx,
        )?;

        let func = node_functions.get(node_name).ok_or_else(|| {
            GraphError::NodeExecution(format!("function '{}' not registered", node_name))
        })?;

        let result = func.call1(py, args).map_err(|e| {
            GraphError::NodeExecution(format!("node '{}' failed: {}", node_name, e))
        })?;

        match &node_decl.edge {
            PyEdgeDecl::Terminal => {
                let json_val: serde_json::Value =
                    pythonize::depythonize(result.bind(py)).map_err(|e| {
                        GraphError::Serialization(format!(
                            "terminal '{}' result conversion failed: {}",
                            node_name, e
                        ))
                    })?;
                pairs.push((node_name.clone(), json_val));
            }
            PyEdgeDecl::Linear { .. } => {
                node_results.insert(node_name.clone(), result);
            }
            PyEdgeDecl::Routing { .. } => {
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
                            "routing node '{}': first element must be a string",
                            node_name
                        ))
                    })?
                    .to_string();

                let variant_value = tuple
                    .get_item(1)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .unbind();

                node_results.insert(format!("{}:{}", node_name, variant_name), variant_value);

                let variant_py = PyString::new(py, &variant_name);
                node_results.insert(
                    format!("{}:__selected_variant", node_name),
                    variant_py.unbind().into(),
                );
            }
        }
    }

    Ok(pairs)
}

fn execute_graph_sync_inner(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    cache_values: &HashMap<String, serde_json::Value>,
    trigger_less_ctx: Option<&PyObject>,
) -> Result<Vec<Box<dyn std::any::Any + Send>>, GraphError> {
    let mut terminal_results: Vec<Box<dyn std::any::Any + Send>> = Vec::new();
    let mut node_results: HashMap<String, PyObject> = HashMap::new();

    // Build incoming edge map
    let mut incoming: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
    for node in node_map.values() {
        match &node.edge {
            PyEdgeDecl::Linear { target } => {
                incoming
                    .entry(target.clone())
                    .or_default()
                    .push((node.name.clone(), None));
            }
            PyEdgeDecl::Routing { variants } => {
                for (variant_name, target) in variants {
                    incoming
                        .entry(target.clone())
                        .or_default()
                        .push((node.name.clone(), Some(variant_name.clone())));
                }
            }
            PyEdgeDecl::Terminal => {}
        }
    }

    let mut skipped_nodes: std::collections::HashSet<String> = std::collections::HashSet::new();

    for node_name in execution_order {
        if skipped_nodes.contains(node_name) {
            continue;
        }

        let node_decl = node_map.get(node_name).ok_or_else(|| {
            GraphError::Execution(format!("node '{}' not found in topology", node_name))
        })?;

        // Check if this node depends on a routing variant that wasn't taken
        if let Some(sources) = incoming.get(node_name) {
            let mut should_skip = false;
            for (from_node, variant) in sources {
                if let Some(v) = variant {
                    let selected_key = format!("{}:__selected_variant", from_node);
                    if let Some(selected) = node_results.get(&selected_key) {
                        let selected_str: String = selected.extract(py).unwrap_or_default();
                        if selected_str != *v {
                            should_skip = true;
                        }
                    }
                }
            }
            if should_skip {
                skipped_nodes.insert(node_name.clone());
                continue;
            }
        }

        // Build arguments. In trigger-less mode, root nodes (no incoming
        // edges, no cache inputs) take the task `Context` as their sole arg.
        let args = build_node_args(
            py,
            node_name,
            node_decl,
            cache_values,
            &node_results,
            &incoming,
            trigger_less_ctx,
        )?;

        // Call the function
        let func = node_functions.get(node_name).ok_or_else(|| {
            GraphError::NodeExecution(format!("function '{}' not registered", node_name))
        })?;

        let result = func.call1(py, args).map_err(|e| {
            GraphError::NodeExecution(format!("node '{}' failed: {}", node_name, e))
        })?;

        // Handle edge type
        match &node_decl.edge {
            PyEdgeDecl::Terminal => {
                let json_val: serde_json::Value =
                    pythonize::depythonize(result.bind(py)).map_err(|e| {
                        GraphError::Serialization(format!(
                            "terminal '{}' result conversion failed: {}",
                            node_name, e
                        ))
                    })?;
                terminal_results.push(Box::new(json_val) as Box<dyn std::any::Any + Send>);
            }
            PyEdgeDecl::Linear { .. } => {
                node_results.insert(node_name.clone(), result);
            }
            PyEdgeDecl::Routing { .. } => {
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
                            "routing node '{}': first element must be a string",
                            node_name
                        ))
                    })?
                    .to_string();

                let variant_value = tuple
                    .get_item(1)
                    .map_err(|e| GraphError::NodeExecution(format!("tuple index error: {}", e)))?
                    .unbind();

                node_results.insert(format!("{}:{}", node_name, variant_name), variant_value);

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

fn build_node_args<'py>(
    py: Python<'py>,
    node_name: &str,
    node_decl: &PyNodeDecl,
    cache_values: &HashMap<String, serde_json::Value>,
    node_results: &HashMap<String, PyObject>,
    incoming: &HashMap<String, Vec<(String, Option<String>)>>,
    trigger_less_ctx: Option<&PyObject>,
) -> Result<Bound<'py, PyTuple>, GraphError> {
    let mut args: Vec<PyObject> = Vec::new();

    // Trigger-less root: a node with no incoming edges and no cache inputs
    // gets the task Context as its sole arg.
    let has_incoming = incoming
        .get(node_name)
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    if let Some(ctx) = trigger_less_ctx {
        if !has_incoming && node_decl.cache_inputs.is_empty() {
            args.push(ctx.clone_ref(py));
            return PyTuple::new(py, &args).map_err(|e| {
                GraphError::NodeExecution(format!("args tuple creation failed: {}", e))
            });
        }
    }

    // Cache inputs
    for input_name in &node_decl.cache_inputs {
        if let Some(val) = cache_values.get(input_name) {
            let py_val = pythonize::pythonize(py, val).map_err(|e| {
                GraphError::Deserialization(format!(
                    "cache input '{}' conversion failed: {}",
                    input_name, e
                ))
            })?;
            args.push(py_val.unbind());
        } else {
            args.push(py.None());
        }
    }

    // Upstream node outputs
    if let Some(sources) = incoming.get(node_name) {
        for (from_node, variant) in sources {
            let key = if let Some(v) = variant {
                format!("{}:{}", from_node, v)
            } else {
                from_node.clone()
            };
            if let Some(result) = node_results.get(&key) {
                args.push(result.clone_ref(py));
            }
        }
    }

    PyTuple::new(py, &args)
        .map_err(|e| GraphError::NodeExecution(format!("args tuple creation failed: {}", e)))
}

// ---------------------------------------------------------------------------
// Reactor handle extraction
// ---------------------------------------------------------------------------

/// Pull `NAME`, `ACCUMULATORS`, and `REACTION_MODE` off a `@cloaca.reactor`-
/// decorated class. Anything else (string, dict, instance) is rejected here so
/// the user gets a precise error rather than a downstream `AttributeError`.
fn extract_reactor_binding(obj: &Bound<'_, PyAny>) -> PyResult<ReactorBinding> {
    if !obj.hasattr("NAME")? || !obj.hasattr("ACCUMULATORS")? || !obj.hasattr("REACTION_MODE")? {
        return Err(PyTypeError::new_err(
            "ComputationGraphBuilder(reactor=...) expects a class decorated with \
             @cloaca.reactor (must expose NAME / ACCUMULATORS / REACTION_MODE)",
        ));
    }
    let name: String = obj.getattr("NAME")?.extract()?;
    let accumulators: Vec<String> = obj.getattr("ACCUMULATORS")?.extract()?;
    let mode: String = obj.getattr("REACTION_MODE")?.extract()?;
    Ok(ReactorBinding {
        name,
        accumulators,
        mode,
    })
}

// ---------------------------------------------------------------------------
// Topology parsing
// ---------------------------------------------------------------------------

fn parse_graph_dict(graph: &Bound<'_, PyDict>) -> PyResult<Vec<PyNodeDecl>> {
    let mut nodes = Vec::new();
    for (key, value) in graph.iter() {
        let name: String = key.extract()?;
        let node_dict = value
            .downcast::<PyDict>()
            .map_err(|_| PyTypeError::new_err(format!("graph['{}'] must be a dict", name)))?;

        let cache_inputs: Vec<String> = if let Some(inputs) = node_dict.get_item("inputs")? {
            inputs
                .downcast::<PyList>()
                .map_err(|_| {
                    PyTypeError::new_err(format!("graph['{}']['inputs'] must be a list", name))
                })?
                .iter()
                .map(|item| item.extract::<String>())
                .collect::<PyResult<_>>()?
        } else {
            Vec::new()
        };

        let edge = if let Some(routes) = node_dict.get_item("routes")? {
            let routes_dict = routes.downcast::<PyDict>().map_err(|_| {
                PyTypeError::new_err(format!("graph['{}']['routes'] must be a dict", name))
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
    Ok(nodes)
}

fn compute_execution_order(nodes: &[PyNodeDecl]) -> Vec<String> {
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    for node in nodes {
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
