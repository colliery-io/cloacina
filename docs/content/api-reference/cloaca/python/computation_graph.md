# cloaca.python.computation_graph <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


Python computation graph bindings.

Mirrors the WorkflowBuilder + @task pattern:
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

### `cloaca.python.computation_graph.ComputationGraphBuilder`

> **Rust Implementation**: [cloacina::python::computation_graph::PyComputationGraphBuilder](../../rust/cloacina/python/computation_graph.md#class-computationgraphbuilder)

#### Methods

##### `new`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">new</span>(_py: , name: str, react: dict, graph: dict) -> <span style="color: var(--md-default-fg-color--light);">Self</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::PyComputationGraphBuilder::new](../../rust/cloacina/python/computation_graph.md#new)

<details>
<summary>Source</summary>

```python
    pub fn new(
        _py: Python<'_>,
        name: &str,
        react: &Bound<'_, PyDict>,
        graph: &Bound<'_, PyDict>,
    ) -> PyResult<Self> {
        let react_mode: String = react
            .get_item("mode")?
            .ok_or_else(|| PyKeyError::new_err("react dict missing 'mode'"))?
            .extract()?;

        let accumulators: Vec<String> = react
            .get_item("accumulators")?
            .ok_or_else(|| PyKeyError::new_err("react dict missing 'accumulators'"))?
            .downcast::<PyList>()
            .map_err(|_| PyTypeError::new_err("'accumulators' must be a list"))?
            .iter()
            .map(|item| item.extract::<String>())
            .collect::<PyResult<_>>()?;

        let nodes_decl = parse_graph_dict(graph)?;

        Ok(PyComputationGraphBuilder {
            name: name.to_string(),
            react_mode,
            accumulators,
            nodes_decl,
        })
    }
```

</details>



##### `__enter__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__enter__</span>(slf: PyRef&lt;Self&gt;) -> <span style="color: var(--md-default-fg-color--light);">PyRef&lt;Self&gt;</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::PyComputationGraphBuilder::__enter__](../../rust/cloacina/python/computation_graph.md#__enter__)

Context manager entry — establish graph context for @node decorators

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `slf` | `PyRef<Self>` |  |


<details>
<summary>Source</summary>

```python
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_graph_context(slf.name.clone());
        slf
    }
```

</details>



##### `__exit__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__exit__</span>(_exc_type: Optional[Any], _exc_value: Optional[Any], _traceback: Optional[Any]) -> <span style="color: var(--md-default-fg-color--light);">bool</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::PyComputationGraphBuilder::__exit__](../../rust/cloacina/python/computation_graph.md#__exit__)

Context manager exit — validate nodes against topology, build executor

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `_exc_type` | `Optional[Any]` |  |
| `_exc_value` | `Optional[Any]` |  |
| `_traceback` | `Optional[Any]` |  |


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

        // Build the executor
        let node_map: HashMap<String, PyNodeDecl> = self
            .nodes_decl
            .iter()
            .cloned()
            .map(|n| (n.name.clone(), n))
            .collect();
        let execution_order = compute_execution_order(&self.nodes_decl);

        let executor = PythonGraphExecutor {
            name: self.name.clone(),
            node_functions: registered_nodes,
            node_map,
            execution_order,
            react_mode: self.react_mode.clone(),
            accumulators: self.accumulators.clone(),
        };

        // Register the executor globally (similar to workflow registration)
        register_graph_executor(self.name.clone(), executor, py)?;

        Ok(false)
    }
```

</details>



##### `__repr__`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">__repr__</span>() -> <span style="color: var(--md-default-fg-color--light);">str</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::PyComputationGraphBuilder::__repr__](../../rust/cloacina/python/computation_graph.md#__repr__)

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



##### `execute`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">execute</span>(inputs: dict) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::PyComputationGraphBuilder::execute](../../rust/cloacina/python/computation_graph.md#execute)

Execute the computation graph with the given input cache.

`inputs` is a dict mapping source names to their data dicts.
Returns the terminal node's output dict.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `inputs` | `dict` |  |


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





### `cloaca.python.computation_graph.PythonGraphExecutor`

> **Rust Implementation**: [cloacina::python::computation_graph::PythonGraphExecutor](../../rust/cloacina/python/computation_graph.md#class-pythongraphexecutor)



## Functions

### `cloaca.python.computation_graph.passthrough_accumulator_decorator`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">passthrough_accumulator_decorator</span>(func: Any) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::passthrough_accumulator_decorator](../../rust/cloacina/python/computation_graph.md#fn-passthrough_accumulator_decorator)

The `@cloaca.passthrough_accumulator` decorator. Registers a function as a passthrough accumulator (Event → Output, no buffering).

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `func` | `Any` |  |


<details>
<summary>Source</summary>

```python
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
```

</details>



### `cloaca.python.computation_graph.stream_accumulator_decorator`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">stream_accumulator_decorator</span>(r#type: str, topic: str, group: Optional[str]) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::stream_accumulator_decorator](../../rust/cloacina/python/computation_graph.md#fn-stream_accumulator_decorator)

Factory for `@cloaca.stream_accumulator(type=..., topic=...)`. Returns a decorator function that registers the target with stream config.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `r#type` | `str` |  |
| `topic` | `str` |  |
| `group` | `Optional[str]` |  |


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
```

</details>



### `cloaca.python.computation_graph.polling_accumulator_decorator`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">polling_accumulator_decorator</span>(interval: str) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::polling_accumulator_decorator](../../rust/cloacina/python/computation_graph.md#fn-polling_accumulator_decorator)

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
```

</details>



### `cloaca.python.computation_graph.batch_accumulator_decorator`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">batch_accumulator_decorator</span>(flush_interval: str, max_buffer_size: Optional[int]) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::batch_accumulator_decorator](../../rust/cloacina/python/computation_graph.md#fn-batch_accumulator_decorator)

Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `flush_interval` | `str` |  |
| `max_buffer_size` | `Optional[int]` |  |


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
```

</details>



### `cloaca.python.computation_graph.node`

<div style="background: var(--md-code-bg-color); padding: 0.75em 1em; border-radius: 0.375em; border-left: 3px solid var(--md-primary-fg-color); margin-bottom: 1em;">
<code style="color: var(--md-code-fg-color); font-family: monospace;"><span style="color: var(--md-primary-fg-color); font-weight: 600;">node</span>(func: Any) -> <span style="color: var(--md-default-fg-color--light);">Any</span></code>
</div>

> **Rust Implementation**: [cloacina::python::computation_graph::node](../../rust/cloacina/python/computation_graph.md#fn-node)

The `@cloaca.node` decorator. Registers a function as a node in the current ComputationGraphBuilder context.

**Parameters:**

| Name | Type | Description |
|------|------|-------------|
| `func` | `Any` |  |


<details>
<summary>Source</summary>

```python
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
```

</details>
