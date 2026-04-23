# cloacina::python::computation_graph <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


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

## Structs

### `cloacina::python::computation_graph::PyAccumulatorRegistration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


**Derives:** `Debug`, `Clone`

Metadata for a registered Python accumulator.

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `accumulator_type` | `String` |  |
| `config` | `HashMap < String , String >` |  |



### `cloacina::python::computation_graph::PyNodeDecl`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


**Derives:** `Debug`, `Clone`

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `cache_inputs` | `Vec < String >` |  |
| `edge` | `PyEdgeDecl` |  |



### `cloacina::python::computation_graph::ComputationGraphBuilder`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.ComputationGraphBuilder](../../../cloaca/python/computation_graph.md#class-computationgraphbuilder)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `react_mode` | `String` |  |
| `accumulators` | `Vec < String >` |  |
| `nodes_decl` | `Vec < PyNodeDecl >` |  |

#### Methods

##### `new`

```rust
fn new (_py : Python < '_ > , name : & str , react : & Bound < '_ , PyDict > , graph : & Bound < '_ , PyDict > ,) -> PyResult < Self >
```

> **Python API**: [cloaca.python.computation_graph.ComputationGraphBuilder.new](../../../cloaca/python/computation_graph.md#new)

<details>
<summary>Source</summary>

```rust
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

```rust
fn __enter__ (slf : PyRef < Self >) -> PyRef < Self >
```

> **Python API**: [cloaca.python.computation_graph.ComputationGraphBuilder.__enter__](../../../cloaca/python/computation_graph.md#__enter__)

Context manager entry — establish graph context for @node decorators

<details>
<summary>Source</summary>

```rust
    pub fn __enter__(slf: PyRef<Self>) -> PyRef<Self> {
        push_graph_context(slf.name.clone());
        slf
    }
```

</details>



##### `__exit__`

```rust
fn __exit__ (& self , py : Python , _exc_type : Option < & Bound < PyAny > > , _exc_value : Option < & Bound < PyAny > > , _traceback : Option < & Bound < PyAny > > ,) -> PyResult < bool >
```

> **Python API**: [cloaca.python.computation_graph.ComputationGraphBuilder.__exit__](../../../cloaca/python/computation_graph.md#__exit__)

Context manager exit — validate nodes against topology, build executor

<details>
<summary>Source</summary>

```rust
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

```rust
fn __repr__ (& self) -> String
```

> **Python API**: [cloaca.python.computation_graph.ComputationGraphBuilder.__repr__](../../../cloaca/python/computation_graph.md#__repr__)

<details>
<summary>Source</summary>

```rust
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

```rust
fn execute (& self , py : Python < '_ > , inputs : & Bound < '_ , PyDict >) -> PyResult < PyObject >
```

> **Python API**: [cloaca.python.computation_graph.ComputationGraphBuilder.execute](../../../cloaca/python/computation_graph.md#execute)

Execute the computation graph with the given input cache.

`inputs` is a dict mapping source names to their data dicts.
Returns the terminal node's output dict.

<details>
<summary>Source</summary>

```rust
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





### `cloacina::python::computation_graph::PythonGraphExecutor`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.PythonGraphExecutor](../../../cloaca/python/computation_graph.md#class-pythongraphexecutor)

#### Fields

| Name | Type | Description |
|------|------|-------------|
| `name` | `String` |  |
| `node_functions` | `HashMap < String , PyObject >` |  |
| `node_map` | `HashMap < String , PyNodeDecl >` |  |
| `execution_order` | `Vec < String >` |  |
| `react_mode` | `String` |  |
| `accumulators` | `Vec < String >` |  |

#### Methods

##### `execute_sync` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn execute_sync (& self , py : Python < '_ > , inputs : & HashMap < String , PyObject > ,) -> PyResult < PyObject >
```

Execute the graph synchronously from Python with dict inputs.

Used by `ComputationGraphBuilder.execute()` for tutorials.

<details>
<summary>Source</summary>

```rust
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
```

</details>



##### `execute` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>
 <span class="plissken-badge plissken-badge-async" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-primary-fg-color); color: white;">async</span>


```rust
async fn execute (& self , cache : & crate :: computation_graph :: types :: InputCache ,) -> GraphResult
```

Execute the graph with the given input cache.

<details>
<summary>Source</summary>

```rust
    pub async fn execute(
        &self,
        cache: &crate::computation_graph::types::InputCache,
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
```

</details>





## Enums

### `cloacina::python::computation_graph::PyEdgeDecl` <span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


#### Variants

- **`Linear`**
- **`Routing`**
- **`Terminal`**



## Functions

### `cloacina::python::computation_graph::push_graph_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn push_graph_context (name : String)
```

<details>
<summary>Source</summary>

```rust
fn push_graph_context(name: String) {
    let mut ctx = ACTIVE_GRAPH_CONTEXT.lock().unwrap();
    *ctx = Some(name);
}
```

</details>



### `cloacina::python::computation_graph::pop_graph_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn pop_graph_context ()
```

<details>
<summary>Source</summary>

```rust
fn pop_graph_context() {
    let mut ctx = ACTIVE_GRAPH_CONTEXT.lock().unwrap();
    *ctx = None;
}
```

</details>



### `cloacina::python::computation_graph::current_graph_context`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn current_graph_context () -> Option < String >
```

<details>
<summary>Source</summary>

```rust
fn current_graph_context() -> Option<String> {
    ACTIVE_GRAPH_CONTEXT.lock().unwrap().clone()
}
```

</details>



### `cloacina::python::computation_graph::register_node`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn register_node (name : String , func : PyObject)
```

<details>
<summary>Source</summary>

```rust
fn register_node(name: String, func: PyObject) {
    NODE_REGISTRY.lock().unwrap().insert(name, func);
}
```

</details>



### `cloacina::python::computation_graph::drain_nodes`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn drain_nodes () -> HashMap < String , PyObject >
```

<details>
<summary>Source</summary>

```rust
fn drain_nodes() -> HashMap<String, PyObject> {
    let mut registry = NODE_REGISTRY.lock().unwrap();
    std::mem::take(&mut *registry)
}
```

</details>



### `cloacina::python::computation_graph::register_accumulator`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn register_accumulator (name : String , func : PyObject , reg : PyAccumulatorRegistration)
```

<details>
<summary>Source</summary>

```rust
fn register_accumulator(name: String, func: PyObject, reg: PyAccumulatorRegistration) {
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .insert(name, (func, reg));
}
```

</details>



### `cloacina::python::computation_graph::get_registered_accumulators`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_registered_accumulators () -> Vec < PyAccumulatorRegistration >
```

Get all registered accumulators (for testing/inspection).

<details>
<summary>Source</summary>

```rust
pub fn get_registered_accumulators() -> Vec<PyAccumulatorRegistration> {
    ACCUMULATOR_REGISTRY
        .lock()
        .unwrap()
        .values()
        .map(|(_, reg)| reg.clone())
        .collect()
}
```

</details>



### `cloacina::python::computation_graph::drain_accumulators`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn drain_accumulators () -> HashMap < String , (PyObject , PyAccumulatorRegistration) >
```

Drain all registered accumulators (used by builder on __exit__).

<details>
<summary>Source</summary>

```rust
pub fn drain_accumulators() -> HashMap<String, (PyObject, PyAccumulatorRegistration)> {
    let mut registry = ACCUMULATOR_REGISTRY.lock().unwrap();
    std::mem::take(&mut *registry)
}
```

</details>



### `cloacina::python::computation_graph::passthrough_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.passthrough_accumulator_decorator](../../../cloaca/python/computation_graph.md#passthrough_accumulator_decorator)

```rust
fn passthrough_accumulator_decorator (py : Python < '_ > , func : PyObject) -> PyResult < PyObject >
```

The `@cloaca.passthrough_accumulator` decorator. Registers a function as a passthrough accumulator (Event → Output, no buffering).

<details>
<summary>Source</summary>

```rust
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



### `cloacina::python::computation_graph::stream_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.stream_accumulator_decorator](../../../cloaca/python/computation_graph.md#stream_accumulator_decorator)

```rust
fn stream_accumulator_decorator (py : Python < '_ > , r#type : String , topic : String , group : Option < String > ,) -> PyResult < PyObject >
```

Factory for `@cloaca.stream_accumulator(type=..., topic=...)`. Returns a decorator function that registers the target with stream config.

<details>
<summary>Source</summary>

```rust
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



### `cloacina::python::computation_graph::polling_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.polling_accumulator_decorator](../../../cloaca/python/computation_graph.md#polling_accumulator_decorator)

```rust
fn polling_accumulator_decorator (py : Python < '_ > , interval : String) -> PyResult < PyObject >
```

Factory for `@cloaca.polling_accumulator(interval=...)`.

<details>
<summary>Source</summary>

```rust
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



### `cloacina::python::computation_graph::batch_accumulator_decorator`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.batch_accumulator_decorator](../../../cloaca/python/computation_graph.md#batch_accumulator_decorator)

```rust
fn batch_accumulator_decorator (py : Python < '_ > , flush_interval : String , max_buffer_size : Option < usize > ,) -> PyResult < PyObject >
```

Factory for `@cloaca.batch_accumulator(flush_interval=..., max_buffer_size=...)`.

<details>
<summary>Source</summary>

```rust
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



### `cloacina::python::computation_graph::node`

<span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-accent-fg-color); color: white;">Binding</span>


> **Python API**: [cloaca.python.computation_graph.node](../../../cloaca/python/computation_graph.md#node)

```rust
fn node (py : Python < '_ > , func : PyObject) -> PyResult < PyObject >
```

The `@cloaca.node` decorator. Registers a function as a node in the current ComputationGraphBuilder context.

<details>
<summary>Source</summary>

```rust
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



### `cloacina::python::computation_graph::register_graph_executor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn register_graph_executor (name : String , executor : PythonGraphExecutor , _py : Python < '_ > ,) -> PyResult < () >
```

<details>
<summary>Source</summary>

```rust
fn register_graph_executor(
    name: String,
    executor: PythonGraphExecutor,
    _py: Python<'_>,
) -> PyResult<()> {
    GRAPH_EXECUTORS.lock().unwrap().insert(name, executor);
    Ok(())
}
```

</details>



### `cloacina::python::computation_graph::get_graph_executor`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn get_graph_executor (name : & str) -> Option < PythonGraphExecutor >
```

Get a registered graph executor by name (for testing / reactor use).

<details>
<summary>Source</summary>

```rust
pub fn get_graph_executor(name: &str) -> Option<PythonGraphExecutor> {
    GRAPH_EXECUTORS.lock().unwrap().get(name).cloned()
}
```

</details>



### `cloacina::python::computation_graph::build_python_graph_declaration`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #4caf50; color: white;">pub</span>


```rust
fn build_python_graph_declaration (graph_name : & str , tenant_id : Option < String > , accumulator_overrides : & [cloacina_workflow_plugin :: types :: AccumulatorConfig] ,) -> Option < crate :: computation_graph :: scheduler :: ComputationGraphDeclaration >
```

Build a [`ComputationGraphDeclaration`] from a registered Python graph executor.

This bridges the Python CG world (decorators + `ComputationGraphBuilder`) into
the Rust `ComputationGraphScheduler` by wrapping the Python executor in a `CompiledGraphFn`.

<details>
<summary>Source</summary>

```rust
pub fn build_python_graph_declaration(
    graph_name: &str,
    tenant_id: Option<String>,
    accumulator_overrides: &[cloacina_workflow_plugin::types::AccumulatorConfig],
) -> Option<crate::computation_graph::scheduler::ComputationGraphDeclaration> {
    use crate::computation_graph::packaging_bridge::{
        PassthroughAccumulatorFactory, StreamBackendAccumulatorFactory,
    };
    use crate::computation_graph::reactor::{InputStrategy, ReactionCriteria};
    use crate::computation_graph::scheduler::{
        AccumulatorDeclaration, ComputationGraphDeclaration, ReactorDeclaration,
    };
    use cloacina_computation_graph::InputCache;
    use std::sync::Arc;

    let executor = get_graph_executor(graph_name)?;

    let criteria = match executor.react_mode.as_str() {
        "when_all" => ReactionCriteria::WhenAll,
        _ => ReactionCriteria::WhenAny,
    };

    let accumulator_names = executor.accumulators.clone();

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
            let factory: Arc<dyn crate::computation_graph::scheduler::AccumulatorFactory> =
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
    })
}
```

</details>



### `cloacina::python::computation_graph::execute_graph_sync`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn execute_graph_sync (py : Python < '_ > , node_functions : & HashMap < String , PyObject > , execution_order : & [String] , node_map : & HashMap < String , PyNodeDecl > , cache_values : & HashMap < String , serde_json :: Value > ,) -> Result < Vec < Box < dyn std :: any :: Any + Send > > , GraphError >
```

<details>
<summary>Source</summary>

```rust
fn execute_graph_sync(
    py: Python<'_>,
    node_functions: &HashMap<String, PyObject>,
    execution_order: &[String],
    node_map: &HashMap<String, PyNodeDecl>,
    cache_values: &HashMap<String, serde_json::Value>,
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

        // Build arguments
        let args = build_node_args(
            py,
            node_name,
            node_decl,
            cache_values,
            &node_results,
            &incoming,
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
```

</details>



### `cloacina::python::computation_graph::build_node_args`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn build_node_args < 'py > (py : Python < 'py > , node_name : & str , node_decl : & PyNodeDecl , cache_values : & HashMap < String , serde_json :: Value > , node_results : & HashMap < String , PyObject > , incoming : & HashMap < String , Vec < (String , Option < String >) > > ,) -> Result < Bound < 'py , PyTuple > , GraphError >
```

<details>
<summary>Source</summary>

```rust
fn build_node_args<'py>(
    py: Python<'py>,
    node_name: &str,
    node_decl: &PyNodeDecl,
    cache_values: &HashMap<String, serde_json::Value>,
    node_results: &HashMap<String, PyObject>,
    incoming: &HashMap<String, Vec<(String, Option<String>)>>,
) -> Result<Bound<'py, PyTuple>, GraphError> {
    let mut args: Vec<PyObject> = Vec::new();

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
```

</details>



### `cloacina::python::computation_graph::parse_graph_dict`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn parse_graph_dict (graph : & Bound < '_ , PyDict >) -> PyResult < Vec < PyNodeDecl > >
```

<details>
<summary>Source</summary>

```rust
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
```

</details>



### `cloacina::python::computation_graph::compute_execution_order`

<span class="plissken-badge plissken-badge-visibility" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: var(--md-default-fg-color--light); color: white;">private</span>


```rust
fn compute_execution_order (nodes : & [PyNodeDecl]) -> Vec < String >
```

<details>
<summary>Source</summary>

```rust
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
```

</details>
