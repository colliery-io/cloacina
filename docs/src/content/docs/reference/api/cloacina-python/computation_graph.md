---
title: "cloacina-python.computation_graph"
description: "Python computation graph bindings."
sidebar:
  order: 6
  hidden: true
---

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.05em 0.6em; font-family: var(--sl-font-mono); font-size: var(--sl-text-xs); line-height: 1.6; border-radius: var(--radius-pill, 1rem); vertical-align: middle; background: var(--sl-color-purple-low); color: var(--sl-color-purple-high);">Binding</span>


Python computation graph bindings.

Mirrors the WorkflowBuilder + @task pattern. Two flavors:
1. **Reactor-triggered** — the graph is bound to a `@cloaca.reactor` class.
Entry nodes consume the reactor's accumulator payloads.
```python
@cloaca.reactor(name="market_maker",
accumulators=["alpha", "beta"],
mode="when_any")
class MarketMaker: pass
with cloaca.ComputationGraphBuilder("market_maker",
reactor=MarketMaker,
graph={...}) as builder: ...
```
2. **Trigger-less** — the graph is invoked by a workflow task via
`@cloaca.task(invokes=...)`. Entry nodes take only the task context;
no accumulator/cache inputs are permitted.
```python
with cloaca.ComputationGraphBuilder("score_transactions",
graph={"score": {}}) as score_graph: ...
```
Pre-split (bundled) form:
```python
with cloaca.ComputationGraphBuilder("market_maker",
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
@cloaca.node
def decision(alpha, beta):
if alpha["value"] + beta["estimate"] > 10:
return ("Signal", {"output": alpha["value"] + beta["estimate"]})
return ("NoAction", {"reason": "below threshold"})
@cloaca.node
def signal_handler(signal):
return {"published": True}
@cloaca.node
def audit_logger(reason):
return {"logged": True}
```

## Classes

### <span id="class.ComputationGraphBuilder"></span>`class ComputationGraphBuilder`

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder](../../rust/cloacina-python/computation_graph/#struct.PyComputationGraphBuilder)

#### Methods

##### <span id="ComputationGraphBuilder.new"></span>`new`

```python
def new(
    _py: Python < '_ >,
    name:  str,
    graph:  Bound < '_ , PyDict >,
    reactor: Option < & Bound < '_ , PyAny > >,
    react: Option < & Bound < '_ , PyAny > >,
) -> PyResult < Self >
```

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder::new](../../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.new)

<details>
<summary>Source</summary>

```python
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
```

</details>



##### <span id="ComputationGraphBuilder.NAME"></span>`NAME`

```python
def NAME() -> str
```

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder::NAME](../../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.NAME)

The graph's declared name. Mirrors Rust's `Graph::NAME` so this instance can serve as the handle referenced by `@cloaca.task(invokes=...)`.

<details>
<summary>Source</summary>

```python
    pub fn NAME(&self) -> String {
        self.name.clone()
    }
```

</details>



##### <span id="ComputationGraphBuilder.__enter__"></span>`__enter__`

```python
def __enter__(slf: PyRef < Self >) -> PyRef < Self >
```

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder::__enter__](../../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.__enter__)

Context manager entry — establish graph context for @node decorators

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef < Self >` |  |


<details>
<summary>Source</summary>

```python
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_graph_context(slf.name.clone());
        slf
    }
```

</details>



##### <span id="ComputationGraphBuilder.__exit__"></span>`__exit__`

```python
def __exit__(
    _exc_type: Option < & Bound < PyAny > >,
    _exc_value: Option < & Bound < PyAny > >,
    _traceback: Option < & Bound < PyAny > >,
) -> PyResult < bool >
```

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder::__exit__](../../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.__exit__)

Context manager exit — validate nodes against topology, build executor

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_exc_type` | `Option < & Bound < PyAny > >` |  |
| `_exc_value` | `Option < & Bound < PyAny > >` |  |
| `_traceback` | `Option < & Bound < PyAny > >` |  |


<details>
<summary>Source</summary>

```python
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
```

</details>



##### <span id="ComputationGraphBuilder.__repr__"></span>`__repr__`

```python
def __repr__() -> str
```

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder::__repr__](../../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.__repr__)

<details>
<summary>Source</summary>

```python
    pub fn __repr__(&self) -> String {
        format!(
            "ComputationGraphBuilder(name='{}', nodes={})",
            self.name,
            self.nodes_decl.len()
        )
    }
```

</details>



##### <span id="ComputationGraphBuilder.execute"></span>`execute`

```python
def execute(inputs:  Bound < '_ , PyDict >) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::PyComputationGraphBuilder::execute](../../rust/cloacina-python/computation_graph/#PyComputationGraphBuilder.execute)

Execute the computation graph with the given input cache.

`inputs` is a dict mapping source names to their data dicts.
Returns the terminal node's output dict.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `inputs` | ` Bound < '_ , PyDict >` |  |


<details>
<summary>Source</summary>

```python
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
```

</details>





### <span id="class.PythonGraphExecutor"></span>`class PythonGraphExecutor`

> **Rust Implementation**: [cloacina-python::computation_graph::PythonGraphExecutor](../../rust/cloacina-python/computation_graph/#struct.PythonGraphExecutor)



## Functions

### <span id="fn.passthrough_accumulator_decorator"></span>`passthrough_accumulator_decorator`

```python
def passthrough_accumulator_decorator(func: PyObject) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::passthrough_accumulator_decorator](../../rust/cloacina-python/computation_graph/#fn.passthrough_accumulator_decorator)

The `@cloaca.passthrough_accumulator` decorator. Registers a function as a passthrough accumulator (Event → Output, no buffering).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `func` | `PyObject` |  |


<details>
<summary>Source</summary>

```python
pub fn passthrough_accumulator_decorator(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    let reg = PyAccumulatorRegistration {
        name: func_name.clone(),
        accumulator_type: "passthrough".to_string(),
        config: HashMap::new(),
        capacity: 0,
    };
    register_accumulator(func_name, func.clone_ref(py), reg);
    Ok(func)
}
```

</details>



### <span id="fn.stream_accumulator_decorator"></span>`stream_accumulator_decorator`

```python
def stream_accumulator_decorator(
    r#type: str,
    topic: str,
    group: Option < String >,
) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::stream_accumulator_decorator](../../rust/cloacina-python/computation_graph/#fn.stream_accumulator_decorator)

Factory for `@cloaca.stream_accumulator(type=..., topic=...)`. Returns a decorator function that registers the target with stream config.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `r#type` | `str` |  |
| `topic` | `str` |  |
| `group` | `Option < String >` |  |


<details>
<summary>Source</summary>

```python
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
            let _py = args.py();
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
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.polling_accumulator_decorator"></span>`polling_accumulator_decorator`

```python
def polling_accumulator_decorator(interval: str) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::polling_accumulator_decorator](../../rust/cloacina-python/computation_graph/#fn.polling_accumulator_decorator)

Factory for `@cloaca.polling_accumulator(interval=...)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `interval` | `str` |  |


<details>
<summary>Source</summary>

```python
pub fn polling_accumulator_decorator(py: Python<'_>, interval: String) -> PyResult<PyObject> {
    let config_interval = interval;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            let mut config = HashMap::new();
            config.insert("interval".to_string(), config_interval.clone());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "polling".to_string(),
                config,
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.batch_accumulator_decorator"></span>`batch_accumulator_decorator`

```python
def batch_accumulator_decorator(
    flush_interval: str,
    max_buffer_size: Option < usize >,
) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::batch_accumulator_decorator](../../rust/cloacina-python/computation_graph/#fn.batch_accumulator_decorator)

Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `flush_interval` | `str` |  |
| `max_buffer_size` | `Option < usize >` |  |


<details>
<summary>Source</summary>

```python
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
            let _py = args.py();
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
                capacity: 0,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.state_accumulator_decorator"></span>`state_accumulator_decorator`

```python
def state_accumulator_decorator(capacity: int) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::state_accumulator_decorator](../../rust/cloacina-python/computation_graph/#fn.state_accumulator_decorator)

Factory for `@cloaca.state_accumulator(capacity=N)`.

Registers a function as a *state* accumulator: a bounded `VecDeque` that
receives values, persists to the DAL on every write, and emits the full
list back as the boundary (enabling cyclic feedback patterns). Mirrors
Rust's `#[state_accumulator(capacity=…)]`.
Capacity semantics (see `StateAccumulator` in cloacina core):
- `> 0`: bounded — evicts oldest when at capacity
- `< 0`: unbounded — grows without limit
- `0`:  write-only sink — no history emitted back

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `capacity` | `int` |  |


<details>
<summary>Source</summary>

```python
pub fn state_accumulator_decorator(py: Python<'_>, capacity: i32) -> PyResult<PyObject> {
    let config_capacity = capacity;

    let decorator = PyCFunction::new_closure(
        py,
        None,
        None,
        move |args: &Bound<'_, PyTuple>,
              _kwargs: Option<&Bound<'_, PyDict>>|
              -> PyResult<PyObject> {
            let _py = args.py();
            let func = args.get_item(0)?;
            let func_name: String = func.getattr("__name__")?.extract()?;

            // Mirror capacity into config too, so it survives the
            // String-keyed config map used at the FFI/override boundary.
            let mut config = HashMap::new();
            config.insert("capacity".to_string(), config_capacity.to_string());

            let reg = PyAccumulatorRegistration {
                name: func_name.clone(),
                accumulator_type: "state".to_string(),
                config,
                capacity: config_capacity,
            };
            register_accumulator(func_name, func.clone().unbind(), reg);
            Ok(func.clone().unbind())
        },
    )?;

    Ok(decorator.into())
}
```

</details>



### <span id="fn.node"></span>`node`

```python
def node(func: PyObject) -> PyResult < PyObject >
```

> **Rust Implementation**: [cloacina-python::computation_graph::node](../../rust/cloacina-python/computation_graph/#fn.node)

The `@cloaca.node` decorator. Registers a function as a node in the current ComputationGraphBuilder context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `func` | `PyObject` |  |


<details>
<summary>Source</summary>

```python
pub fn node(py: Python<'_>, func: PyObject) -> PyResult<PyObject> {
    let _ctx = current_graph_context().ok_or_else(|| {
        PyValueError::new_err(
            "@cloaca.node must be used inside a ComputationGraphBuilder context manager",
        )
    })?;

    let func_name: String = func.getattr(py, "__name__")?.extract(py)?;
    register_node(func_name, func.clone_ref(py));

    // Return the function unchanged (transparent decorator)
    Ok(func)
}
```

</details>



